//! Lienzo — la app. Bevy para ver y tocar; el core para componer y sonar.

mod audio;
mod cards;
mod dive;
mod draw;
mod gallery;
mod input;
mod material;
mod orbit;
mod paper;
mod terrain;
mod textures;
mod ui;
mod water;
mod world;

use bevy::prelude::*;
use bevy::window::WindowResolution;
use lienzo_core::{SampleBank, Scene};
use std::path::PathBuf;
use std::sync::Arc;

/// La obra abierta. Fuente de verdad en la app; se manda al audio al cambiar.
#[derive(Resource)]
pub struct Obra {
    pub scene: Scene,
    pub dirty: bool,
    /// Índice del trazo recién añadido, para que suene al instante.
    pub new_stroke: Option<usize>,
    pub path: Option<PathBuf>,
}

/// En Android los assets viven dentro del APK y se leen con el AssetManager.
#[cfg(target_os = "android")]
pub struct ApkSource;

#[cfg(target_os = "android")]
impl lienzo_core::AssetSource for ApkSource {
    fn read(&self, rel: &str) -> Result<Vec<u8>, String> {
        use std::io::Read;
        let am = bevy::android::ANDROID_APP
            .get()
            .ok_or("ANDROID_APP no inicializado")?
            .asset_manager();
        let path = std::ffi::CString::new(format!("audio/{rel}")).map_err(|e| e.to_string())?;
        let mut asset = am
            .open(&path)
            .ok_or_else(|| format!("no está en el APK: audio/{rel}"))?;
        let mut bytes = Vec::new();
        asset.read_to_end(&mut bytes).map_err(|e| e.to_string())?;
        Ok(bytes)
    }
}

/// Interruptores de depuración para bisecar en el teléfono sin recompilar.
/// Android: `<external files>/debug.txt`; escritorio: `LIENZO_DEBUG`. Una
/// palabra por línea: no_terrain, no_surface, no_caustics, no_backdrop,
/// no_bloom, no_rays, no_cards.
#[derive(Resource, Default, Debug, Clone)]
pub struct DebugFlags(pub std::collections::HashSet<String>);

impl DebugFlags {
    pub fn has(&self, k: &str) -> bool {
        self.0.contains(k)
    }
}

fn debug_flags() -> DebugFlags {
    #[allow(unused_mut)]
    let mut text = std::env::var("LIENZO_DEBUG").unwrap_or_default();
    #[cfg(target_os = "android")]
    if let Some(p) = bevy::android::ANDROID_APP
        .get()
        .and_then(|a| a.external_data_path())
    {
        if let Ok(t) = std::fs::read_to_string(p.join("debug.txt")) {
            text.push('\n');
            text.push_str(&t);
        }
    }
    DebugFlags(
        text.split(|c: char| c.is_whitespace() || c == ',')
            .filter(|w| !w.is_empty())
            .map(|w| w.to_string())
            .collect(),
    )
}

/// Raíz del proyecto: la carpeta que contiene `assets/manifest.toml`.
#[cfg(not(target_os = "android"))]
fn project_root() -> PathBuf {
    let mut d = std::env::current_dir().expect("cwd");
    loop {
        if d.join("assets").join("manifest.toml").exists() {
            return d;
        }
        if !d.pop() {
            panic!("no encuentro assets/manifest.toml subiendo desde el directorio actual");
        }
    }
}

/// Ajustes del renderer. En Android se usa GLES: el backend Vulkan de wgpu
/// aborta en los drivers Mali (pool de descriptores agotado en un camino que
/// wgpu considera imposible), y pasa igual con los *binding arrays* apagados.
fn render_plugin() -> bevy::render::RenderPlugin {
    use bevy::render::settings::{Backends, WgpuLimits, WgpuSettings};
    let mut settings = WgpuSettings::default();
    // LIENZO_GLES=1 reproduce en escritorio el camino de Android (GL + límites)
    let gles = cfg!(target_os = "android") || std::env::var("LIENZO_GLES").is_ok();
    if gles {
        settings.backends = Some(Backends::GL);
        // Sin storage buffers (GLES no los ofrece en el vértice): Bevy pasa a
        // uniformes, como en WebGL2. Bevy solo lo hace solo en wasm, y con la
        // prioridad por defecto ignora `limits`; `constrained_limits` sí acota.
        let mut limits = WgpuLimits::downlevel_webgl2_defaults();
        limits.max_texture_dimension_2d = 8192;
        settings.constrained_limits = Some(limits);
    }
    bevy::render::RenderPlugin {
        render_creation: settings.into(),
        ..default()
    }
}

/// Hilos de Bevy. En el teléfono, los hilos del pool de cómputo giran en
/// espera activa y gastan CPU sin trabajo que hacer; con uno por pool basta
/// para esta app (la mezcla de audio va en su propio hilo, fuera de Bevy).
fn task_pools(flags: &DebugFlags) -> bevy::app::TaskPoolPlugin {
    use bevy::app::{TaskPoolOptions, TaskPoolPlugin};
    let mut o = TaskPoolOptions::default();
    // Sospecha: con un solo hilo por pool, en GLES no se dibuja nada. Solo con
    // la bandera `pools_min` se limita.
    if flags.has("pools_min") {
        o.min_total_threads = 1;
        o.max_total_threads = 3;
        for pool in [&mut o.io, &mut o.async_compute, &mut o.compute] {
            pool.min_threads = 1;
            pool.max_threads = 1;
        }
    }
    TaskPoolPlugin {
        task_pool_options: o,
    }
}

#[bevy_main]
pub fn main() {
    if let Err(e) = run() {
        eprintln!("lienzo: {e:#}");
    }
    // En Android la actividad puede destruirse y volver a crearse en el mismo
    // proceso, y winit no permite un segundo bucle de eventos: se sale del
    // proceso para que la siguiente apertura arranque limpia.
    #[cfg(target_os = "android")]
    std::process::exit(0);
}

pub fn run() -> anyhow::Result<()> {
    #[cfg(not(target_os = "android"))]
    let (asset_plugin, bank) = {
        let assets_dir = project_root().join("assets");
        let bank = SampleBank::load(&assets_dir.join("audio"))?;
        (
            AssetPlugin {
                file_path: assets_dir.to_string_lossy().into_owned(),
                ..default()
            },
            bank,
        )
    };
    #[cfg(target_os = "android")]
    let (asset_plugin, bank) = (AssetPlugin::default(), SampleBank::load_from(&ApkSource)?);
    let bank = Arc::new(bank);
    let arg = std::env::args().nth(1).map(PathBuf::from);
    let (scene, path) = match &arg {
        Some(p) if p.exists() => (Scene::from_json(&std::fs::read_to_string(p)?)?, arg.clone()),
        Some(_) => (Scene::new(7), arg.clone()),
        // sin argumento: la obra en curso, o un lienzo en blanco la primera vez
        None => gallery::startup_scene(),
    };

    let link = audio::start(bank.clone(), scene.clone())?;

    let flags = debug_flags();
    let _ = material::MIN_SHADER.set(flags.has("min_shader"));
    if !flags.0.is_empty() {
        eprintln!("lienzo: depuración {:?}", flags.0);
    }
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Lienzo".into(),
                    resolution: WindowResolution::new(1280, 800),
                    ..default()
                }),
                ..default()
            })
            .set(asset_plugin)
            .set(render_plugin())
            .set(task_pools(&flags)),
    );
    bevy::asset::embedded_asset!(app, "card.wesl");
    bevy::asset::embedded_asset!(app, "card_min.wesl");
    // Obra contemplativa, no juego: ~30 fps en el teléfono y 60 en escritorio
    // bastan, y responde al instante a cualquier gesto. En segundo plano, casi
    // nada. Esto es lo que más batería y CPU ahorra.
    app.insert_resource(bevy::winit::WinitSettings {
        focused_mode: bevy::winit::UpdateMode::reactive(std::time::Duration::from_millis(
            if cfg!(target_os = "android") { 33 } else { 16 },
        )),
        unfocused_mode: bevy::winit::UpdateMode::reactive_low_power(
            std::time::Duration::from_millis(250),
        ),
    });
    app.add_plugins(bevy::pbr::MaterialPlugin::<material::CardMaterial>::default())
        .insert_resource(Obra {
            scene,
            dirty: false,
            new_stroke: None,
            path,
        })
        .insert_resource(flags)
        .init_resource::<draw::DrawState>()
        .init_resource::<input::Pointer>()
        .init_resource::<water::Applied>()
        .init_resource::<terrain::Built>()
        .init_resource::<dive::Dive>()
        .init_resource::<gallery::Gallery>()
        .init_resource::<gallery::Pending>()
        .init_resource::<gallery::Exporting>()
        .insert_resource(gallery::Bank(bank))
        .insert_resource(link)
        .init_resource::<cards::SpriteMaterials>()
        .init_resource::<ui::Swipe>()
        .init_resource::<paper::Paper>()
        .add_systems(
            Startup,
            (
                ui::load_font,
                world::setup,
                cards::spawn_scene,
                ui::setup,
                gallery::setup,
            )
                .chain(),
        )
        .add_observer(ui::on_click)
        .add_observer(gallery::on_click)
        .add_systems(
            Update,
            (
                (
                    input::gather,
                    gallery::keys,
                    gallery::apply,
                    gallery::autosave,
                    paper::ease,
                    gallery::refresh,
                    draw::keys,
                    draw::draw,
                    draw::touch,
                    ui::refresh,
                    water::apply,
                    terrain::update,
                    cards::resettle,
                    orbit::control,
                )
                    .chain(),
                (
                    dive::swim,
                    dive::listener,
                    cards::route_visibility,
                    paper::hide_submerged,
                    paper::fps_trace,
                    world::drift_particles.run_if(paper::underwater),
                    world::sway_rays.run_if(paper::underwater),
                    world::refraction,
                    cards::animate,
                    world::billboards,
                    draw::title,
                    sync_audio,
                    shot,
                )
                    .chain(),
            )
                .chain(),
        )
        .run();
    Ok(())
}

/// Con `LIENZO_SHOT=ruta.png`, guarda la ventana al fotograma 120 y cierra.
/// Es la forma de "ver" la app sin pantalla (sesiones sin captura, CI).
fn shot(mut commands: Commands, mut n: Local<u32>, mut exit: MessageWriter<AppExit>) {
    let Ok(path) = std::env::var("LIENZO_SHOT") else {
        return;
    };
    *n += 1;
    if *n == 120 {
        commands
            .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
            .observe(bevy::render::view::screenshot::save_to_disk(path));
    }
    if *n == 200 {
        exit.write(AppExit::Success);
    }
}

fn sync_audio(mut obra: ResMut<Obra>, link: Res<audio::AudioLink>) {
    if obra.dirty {
        link.set_scene(obra.scene.clone(), obra.new_stroke.take());
        obra.dirty = false;
    }
}
