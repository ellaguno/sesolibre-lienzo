//! Guardar, abrir y la galería. Las obras viven en
//! `~/.local/share/lienzo/obras/*.json` (o `$LIENZO_OBRAS`). Son kilobytes:
//! la música se deriva al abrirlas.

use crate::cards::{self, Owned, SpriteMaterials};
use crate::draw::DrawState;
use crate::material::CardMaterial;
use crate::world::Common;
use crate::Obra;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::ButtonState;
use bevy::picking::events::PointerClick;
use bevy::prelude::*;
use bevy::text::FontSize;
use lienzo_core::Scene;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn obras_dir() -> PathBuf {
    if let Ok(d) = std::env::var("LIENZO_OBRAS") {
        return PathBuf::from(d);
    }
    #[cfg(target_os = "android")]
    if let Some(p) = bevy::android::ANDROID_APP
        .get()
        .and_then(|a| a.internal_data_path())
    {
        return p.join("obras");
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".local/share/lienzo/obras")
}

pub struct Entry {
    pub path: PathBuf,
    pub title: String,
    pub strokes: usize,
}

#[derive(Resource, Default)]
pub struct Gallery {
    pub open: bool,
    pub entries: Vec<Entry>,
    pub editing_title: bool,
    pub status: String,
    pub rebuild: bool,
}

#[derive(Component)]
pub struct GalleryRoot;

#[derive(Component)]
pub struct GalleryList;

#[derive(Component)]
pub enum GalleryAction {
    Open(PathBuf),
    New,
    Save,
    Export,
    Close,
}

fn slug(s: &str) -> String {
    let mut out = String::new();
    for c in s.to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    let out = out.trim_matches('-').to_string();
    if out.is_empty() {
        "obra".into()
    } else {
        out
    }
}

fn scan() -> Vec<Entry> {
    let mut out = vec![];
    if let Ok(rd) = std::fs::read_dir(obras_dir()) {
        for e in rd.flatten() {
            let p = e.path();
            let hidden = p
                .file_name()
                .map(|n| n.to_string_lossy().starts_with('.'))
                .unwrap_or(true);
            if !hidden && p.extension().map(|x| x == "json").unwrap_or(false) {
                if let Ok(s) = std::fs::read_to_string(&p) {
                    if let Ok(sc) = Scene::from_json(&s) {
                        let title = if sc.title.is_empty() {
                            p.file_stem().unwrap().to_string_lossy().into_owned()
                        } else {
                            sc.title.clone()
                        };
                        out.push(Entry {
                            path: p,
                            title,
                            strokes: sc.strokes.len(),
                        });
                    }
                }
            }
        }
    }
    out.sort_by(|a, b| {
        let ma = std::fs::metadata(&a.path).and_then(|m| m.modified()).ok();
        let mb = std::fs::metadata(&b.path).and_then(|m| m.modified()).ok();
        mb.cmp(&ma)
    });
    out
}

/// La obra en curso se guarda sola en `.actual.json` (y la ruta de galería
/// de la que viene, si la hay, en `.actual.path`): al volver se continúa.
fn actual_json() -> PathBuf {
    obras_dir().join(".actual.json")
}

fn actual_path() -> PathBuf {
    obras_dir().join(".actual.path")
}

/// Escena con la que arranca la app: la obra en curso si la hay; si no (la
/// primera vez), un lienzo en blanco, y la demo queda en la galería.
pub fn startup_scene() -> (Scene, Option<PathBuf>) {
    let dir = obras_dir();
    if let Some(sc) = std::fs::read_to_string(actual_json())
        .ok()
        .and_then(|s| Scene::from_json(&s).ok())
    {
        let path = std::fs::read_to_string(actual_path())
            .ok()
            .map(|p| PathBuf::from(p.trim()))
            .filter(|p| p.exists());
        return (sc, path);
    }
    // primera vez: la demo pasa a la galería y se empieza en blanco
    let _ = std::fs::create_dir_all(&dir);
    let demo = dir.join("demo.json");
    if !demo.exists() {
        let mut d = Scene::demo();
        d.title = "Demo".into();
        let _ = std::fs::write(&demo, d.to_json());
    }
    let mut sc = Scene::new(new_seed());
    sc.title = String::new();
    (sc, None)
}

/// Autoguardado: en cuanto la obra cambia (y tras un respiro) se escribe
/// `.actual.json`; si la obra viene de la galería, también se actualiza ahí.
/// Un niño no tiene que acordarse de guardar.
pub fn autosave(
    time: Res<Time>,
    mut obra: ResMut<Obra>,
    mut last: Local<(String, f32, bool)>,
) {
    let key = format!(
        "{}|{:?}|{}|{}|{:?}",
        obra.scene.strokes.len(),
        obra.scene.water,
        obra.scene.title,
        obra.scene.seed,
        obra.path
    );
    let now = time.elapsed_secs();
    let (ref mut last_key, ref mut since, ref mut pending) = *last;
    if *last_key != key {
        *last_key = key;
        *since = now;
        *pending = true;
        return;
    }
    if *pending && now - *since > 1.5 {
        *pending = false;
        let dir = obras_dir();
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(actual_json(), obra.scene.to_json());
        match &obra.path {
            // solo se reescriben obras de la galería, nunca un archivo
            // abierto desde fuera (las escenas de prueba del repo)
            Some(p) if p.starts_with(&dir) => {
                let _ = std::fs::write(actual_path(), p.to_string_lossy().as_bytes());
                let _ = std::fs::write(p, obra.scene.to_json());
            }
            Some(_) => {
                let _ = std::fs::remove_file(actual_path());
            }
            None => {
                let _ = std::fs::remove_file(actual_path());
            }
        }
        obra.dirty = false;
    }
}

/// Guarda la obra. Sin ruta, inventa una a partir del título.
pub fn save(obra: &mut Obra) -> Result<PathBuf, String> {
    let dir = obras_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    if obra.scene.title.is_empty() {
        obra.scene.title = format!("Obra {}", scan().len() + 1);
    }
    let path = obra.path.clone().unwrap_or_else(|| {
        let mut p = dir.join(format!("{}.json", slug(&obra.scene.title)));
        let mut n = 2;
        while p.exists() {
            p = dir.join(format!("{}-{}.json", slug(&obra.scene.title), n));
            n += 1;
        }
        p
    });
    std::fs::write(&path, obra.scene.to_json()).map_err(|e| e.to_string())?;
    obra.path = Some(path.clone());
    Ok(path)
}

fn new_seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(7)
}

/// Sustituye la obra abierta: vacía las cartas, pone la escena nueva y avisa al audio.
fn replace_scene(
    scene: Scene,
    path: Option<PathBuf>,
    obra: &mut Obra,
    commands: &mut Commands,
    owned: &Query<Entity, With<Owned>>,
    common: &Common,
    mats: &mut SpriteMaterials,
    assets: &AssetServer,
    materials: &mut Assets<CardMaterial>,
) {
    for e in owned.iter() {
        commands.entity(e).despawn();
    }
    for (i, s) in scene.strokes.iter().enumerate() {
        cards::spawn_stroke(commands, common, mats, assets, materials, &scene, s, i);
    }
    obra.scene = scene;
    obra.path = path;
    obra.dirty = true;
    obra.new_stroke = None;
}

const BG: Color = Color::srgba(0.0, 0.04, 0.09, 0.80);
const BTN: Color = Color::srgba(1.0, 1.0, 1.0, 0.07);
const TXT: Color = Color::srgba(0.94, 0.97, 0.98, 0.92);

pub fn setup(mut commands: Commands, font: Res<crate::ui::UiFont>) {
    commands
        .spawn((
            GalleryRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(20.0),
                right: Val::Percent(20.0),
                top: Val::Px(50.0),
                bottom: Val::Px(110.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                padding: UiRect::all(Val::Px(16.0)),
                border_radius: BorderRadius::all(Val::Px(12.0)),
                display: Display::None,
                ..default()
            },
            BackgroundColor(BG),
        ))
        .with_children(|p| {
            p.spawn((
                Text::new("Galería"),
                TextFont {
                    font: font.0.clone().into(),
                    font_size: FontSize::Px(20.0),
                    ..default()
                },
                TextColor(TXT),
            ));
            p.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(8.0),
                ..default()
            })
            .with_children(|row| {
                button(row, &font.0, GalleryAction::New, "nueva obra");
                button(row, &font.0, GalleryAction::Save, "guardar");
                button(row, &font.0, GalleryAction::Export, "exportar audio");
                button(row, &font.0, GalleryAction::Close, "cerrar");
            });
            p.spawn((
                GalleryList,
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(6.0),
                    overflow: Overflow::scroll_y(),
                    flex_grow: 1.0,
                    ..default()
                },
            ));
        });
}

fn button(row: &mut ChildSpawnerCommands, font: &Handle<Font>, action: GalleryAction, label: &str) {
    row.spawn((
        action,
        Node {
            padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
            border: UiRect::all(Val::Px(1.0)),
            border_radius: BorderRadius::all(Val::Px(999.0)),
            ..default()
        },
        BackgroundColor(BTN),
        BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.22)),
    ))
    .with_children(|b| {
        b.spawn((
            Text::new(label.to_string()),
            TextFont {
                font: font.clone().into(),
                font_size: FontSize::Px(14.0),
                ..default()
            },
            TextColor(TXT),
        ));
    });
}

pub fn on_click(
    click: On<PointerClick>,
    q: Query<&GalleryAction>,
    mut g: ResMut<Gallery>,
    mut pending: ResMut<Pending>,
) {
    let Ok(action) = q.get(click.event().entity) else {
        return;
    };
    match action {
        GalleryAction::Open(p) => pending.0 = Some(Cmd::Open(p.clone())),
        GalleryAction::New => pending.0 = Some(Cmd::New),
        GalleryAction::Save => pending.0 = Some(Cmd::Save),
        GalleryAction::Export => pending.0 = Some(Cmd::Export),
        GalleryAction::Close => {
            g.open = false;
        }
    }
}

pub enum Cmd {
    Open(PathBuf),
    New,
    Save,
    Export,
}

/// Render en curso: el hilo avisa por aquí cuando termina.
#[derive(Resource, Default)]
pub struct Exporting(
    pub Option<std::sync::Mutex<std::sync::mpsc::Receiver<Result<PathBuf, String>>>>,
);

#[derive(Resource)]
pub struct Bank(pub std::sync::Arc<lienzo_core::SampleBank>);

pub const EXPORT_SECONDS: f64 = 180.0;

/// Órdenes pendientes (de clics u observadores) que se ejecutan en `apply`,
/// donde hay acceso a todo lo necesario.
#[derive(Resource, Default)]
pub struct Pending(pub Option<Cmd>);

pub fn keys(
    keys: Res<ButtonInput<KeyCode>>,
    mut input: MessageReader<KeyboardInput>,
    mut g: ResMut<Gallery>,
    mut obra: ResMut<Obra>,
    mut pending: ResMut<Pending>,
    mut st: ResMut<DrawState>,
) {
    if g.editing_title {
        for ev in input.read() {
            if ev.state != ButtonState::Pressed {
                continue;
            }
            match &ev.logical_key {
                Key::Enter | Key::Escape => {
                    g.editing_title = false;
                    st.title_dirty = true;
                }
                Key::Backspace => {
                    obra.scene.title.pop();
                }
                _ => {
                    if let Some(t) = &ev.text {
                        if !t.chars().any(|c| c.is_control()) && obra.scene.title.len() < 40 {
                            obra.scene.title.push_str(t);
                        }
                    }
                }
            }
        }
        return;
    }
    if keys.just_pressed(KeyCode::KeyG) {
        g.open = !g.open;
        g.rebuild = g.open;
    }
    if keys.just_pressed(KeyCode::KeyT) {
        g.editing_title = true;
    }
    if keys.just_pressed(KeyCode::KeyN) {
        pending.0 = Some(Cmd::New);
    }
    if keys.just_pressed(KeyCode::KeyS) {
        pending.0 = Some(Cmd::Save);
    }
    if keys.just_pressed(KeyCode::KeyE) {
        pending.0 = Some(Cmd::Export);
    }
}

pub fn apply(
    mut pending: ResMut<Pending>,
    mut g: ResMut<Gallery>,
    mut obra: ResMut<Obra>,
    mut commands: Commands,
    owned: Query<Entity, With<Owned>>,
    common: Res<Common>,
    mut mats: ResMut<SpriteMaterials>,
    assets: Res<AssetServer>,
    mut materials: ResMut<Assets<CardMaterial>>,
    mut st: ResMut<DrawState>,
    bank: Res<Bank>,
    mut exporting: ResMut<Exporting>,
) {
    // ¿terminó un export?
    let done = exporting
        .0
        .as_ref()
        .and_then(|rx| rx.lock().ok().and_then(|rx| rx.try_recv().ok()));
    if let Some(r) = done {
        g.status = match r {
            Ok(p) => format!("audio exportado: {}", p.display()),
            Err(e) => format!("no se pudo exportar: {e}"),
        };
        exporting.0 = None;
    }
    let Some(cmd) = pending.0.take() else {
        return;
    };
    match cmd {
        Cmd::Export => {
            if exporting.0.is_some() {
                g.status = "ya hay un export en curso".into();
            } else {
                // se guarda primero para que el WAV quede junto al JSON
                match save(&mut obra) {
                    Ok(json) => {
                        let wav = json.with_extension("wav");
                        let scene = obra.scene.clone();
                        let bank = bank.0.clone();
                        let (tx, rx) = std::sync::mpsc::channel();
                        exporting.0 = Some(std::sync::Mutex::new(rx));
                        g.status = format!("exportando {:.0} s de audio…", EXPORT_SECONDS);
                        std::thread::spawn(move || {
                            let audio = lienzo_core::render_offline(
                                bank,
                                &scene,
                                EXPORT_SECONDS,
                                Default::default(),
                            );
                            let r = lienzo_core::write_wav(&wav, &audio)
                                .map(|_| wav)
                                .map_err(|e| e.to_string());
                            let _ = tx.send(r);
                        });
                    }
                    Err(e) => g.status = format!("no se pudo guardar antes de exportar: {e}"),
                }
            }
        }
        Cmd::Save => match save(&mut obra) {
            Ok(p) => {
                g.status = format!("guardada: {}", p.file_name().unwrap().to_string_lossy());
                g.rebuild = true;
            }
            Err(e) => g.status = format!("no se pudo guardar: {e}"),
        },
        Cmd::New => {
            // lo dibujado no se pierde: si tiene trazos y no estaba en la
            // galería, se guarda ahí antes de empezar de cero
            if !obra.scene.strokes.is_empty() && obra.path.is_none() {
                let _ = save(&mut obra);
            }
            let mut sc = Scene::new(new_seed());
            sc.title = String::new();
            replace_scene(
                sc,
                None,
                &mut obra,
                &mut commands,
                &owned,
                &common,
                &mut mats,
                &assets,
                &mut materials,
            );
            g.open = false;
            g.status = "obra nueva".into();
        }
        Cmd::Open(p) => match std::fs::read_to_string(&p)
            .ok()
            .and_then(|s| Scene::from_json(&s).ok())
        {
            Some(sc) => {
                g.status = format!("abierta: {}", sc.title);
                replace_scene(
                    sc,
                    Some(p),
                    &mut obra,
                    &mut commands,
                    &owned,
                    &common,
                    &mut mats,
                    &assets,
                    &mut materials,
                );
                g.open = false;
            }
            None => g.status = format!("no se pudo abrir {}", p.display()),
        },
    }
    st.title_dirty = true;
}

/// Muestra u oculta el panel y reconstruye la lista cuando hace falta.
pub fn refresh(
    mut g: ResMut<Gallery>,
    font: Res<crate::ui::UiFont>,
    mut commands: Commands,
    mut root: Query<&mut Node, With<GalleryRoot>>,
    list: Query<Entity, With<GalleryList>>,
) {
    if let Ok(mut node) = root.single_mut() {
        node.display = if g.open { Display::Flex } else { Display::None };
    }
    if !g.rebuild {
        return;
    }
    g.rebuild = false;
    g.entries = scan();
    let Ok(list) = list.single() else { return };
    commands.entity(list).despawn_children();
    commands.entity(list).with_children(|p| {
        if g.entries.is_empty() {
            p.spawn((
                Text::new("Todavía no hay obras guardadas. Dibuja algo y pulsa S."),
                TextFont {
                    font: font.0.clone().into(),
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(Color::srgba(0.8, 0.9, 0.95, 0.7)),
            ));
        }
        for e in &g.entries {
            button(
                p,
                &font.0,
                GalleryAction::Open(e.path.clone()),
                &format!("{}  ·  {} trazos", e.title, e.strokes),
            );
        }
    });
}

pub fn title_of(obra: &Obra) -> String {
    if obra.scene.title.is_empty() {
        "sin título".into()
    } else {
        obra.scene.title.clone()
    }
}
