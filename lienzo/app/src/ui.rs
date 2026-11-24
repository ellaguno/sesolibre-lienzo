//! La paleta: una fila de pinceles abajo y un botón de contemplación. En
//! contemplación la paleta se esconde; solo queda la obra.

use crate::draw::{brush_name, DrawState, Layer, View, SWATCHES};
use crate::textures;
use crate::Obra;
use bevy::picking::events::PointerClick;
use bevy::prelude::*;
use bevy::text::FontSize;
use lienzo_core::{Brush, Water};

/// Alto de la franja de la paleta. En pantalla táctil es más alta: botones de
/// dedo y dos filas.
/// Alto nominal de la paleta (una fila de iconos y una de acciones). La
/// paleta real se ajusta sola al envolver filas; para la zona muerta del
/// dibujo se usa `PaletteRoot`'s `ComputedNode`.
pub fn palette_h() -> f32 {
    icon_d() + row_h() + 30.0
}

/// Diámetro de los botones de icono.
fn icon_d() -> f32 {
    if cfg!(target_os = "android") {
        58.0
    } else {
        46.0
    }
}

fn row_h() -> f32 {
    font_px() * 2.4
}

fn font_px() -> f32 {
    if cfg!(target_os = "android") {
        16.0
    } else {
        14.0
    }
}

/// Separador en los textos. La fuente por defecto de Bevy no trae "·".
const SEP: &str = " · ";

#[derive(Component)]
pub struct PaletteRoot;

#[derive(Component)]
pub enum Action {
    Pick(Brush),
    Water(Water),
    View(View),
    Layer(Layer),
    Swatch(usize),
    Contemplate,
    Undo,
    Gallery,
}

#[derive(Component)]
pub struct Hint;

/// El disco del botón Color: muestra el color activo.
#[derive(Component)]
pub struct ColorDot;

// Botones transparentes y sutiles: un velo claro, borde fino, texto luminoso.
const BG: Color = Color::srgba(0.0, 0.0, 0.0, 0.0);
const BTN: Color = Color::srgba(1.0, 1.0, 1.0, 0.07);
const BTN_ON: Color = Color::srgba(1.0, 1.0, 1.0, 0.24);
const BORDER: Color = Color::srgba(1.0, 1.0, 1.0, 0.22);
const BORDER_ON: Color = Color::srgba(1.0, 1.0, 1.0, 0.65);
const TXT: Color = Color::srgba(0.94, 0.97, 0.98, 0.92);

#[derive(Component)]
pub struct WaterRow;

/// La fila desplazable de pinceles.
#[derive(Component)]
pub struct BrushRow;

/// Cuánto se arrastró la fila desde que se tocó: si fue un barrido, al
/// soltar no se elige pincel.
#[derive(Resource, Default)]
pub struct Swipe {
    pub moved: f32,
}

/// Botón de salir de la contemplación (aparece al tocar).
#[derive(Component)]
pub struct ExitBtn;

#[derive(Component)]
pub struct LayerCol;

#[derive(Component)]
pub struct SwatchRow;

/// La fuente de toda la interfaz (Noto Sans, OFL).
#[derive(Resource)]
pub struct UiFont(pub Handle<Font>);

pub fn load_font(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(UiFont(assets.load("fonts/NotoSans-Variable.ttf")));
}

pub fn setup(
    mut commands: Commands,
    font: Res<UiFont>,
    assets: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
) {
    // Barra de pinceles: el icono es la imagen del elemento.
    let d = icon_d();
    let sprite = |name: &'static str| -> (Handle<Image>, f32) {
        (
            assets.load(format!("sprites/cut/{name}.png")),
            crate::cards::aspect(name),
        )
    };
    let gen = |images: &mut Assets<Image>, kind: &str| -> (Handle<Image>, f32) {
        (images.add(textures::icon(kind)), 1.0)
    };
    let mut icons: Vec<(Brush, Handle<Image>, f32)> = Vec::new();
    for b in Brush::ALL.iter() {
        let (img, aspect) = match b {
            Brush::Kelp => sprite("kelp_hoja"),
            Brush::Coral => sprite("coral_a_1"),
            Brush::Anemona => sprite("anemona"),
            Brush::Cardumen => sprite("pez_b"),
            Brush::Medusa => sprite("medusa"),
            Brush::Ballena => sprite("ballena"),
            Brush::Barco => sprite("barco"),
            Brush::Burbujas => gen(&mut images, "burbujas"),
            Brush::Farol => gen(&mut images, "farol"),
            Brush::Haz => gen(&mut images, "haz"),
            Brush::Arcoiris => gen(&mut images, "arcoiris"),
            Brush::Elevar => gen(&mut images, "elevar"),
            Brush::Hundir => gen(&mut images, "hundir"),
            Brush::Ruta => gen(&mut images, "ruta"),
            Brush::Color => (Handle::default(), 1.0),
        };
        icons.push((*b, img, aspect));
    }
    commands
        .spawn((
            PaletteRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                bottom: Val::Px(0.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(10.0),
                justify_content: JustifyContent::End,
                align_items: AlignItems::Center,
                padding: UiRect::axes(Val::Px(8.0), Val::Px(10.0)),
                ..default()
            },
            BackgroundColor(BG),
        ))
        .with_children(|col| {
            // fila corta: vista, deshacer, galería, contemplar
            col.spawn(Node {
                width: Val::Percent(100.0),
                flex_wrap: FlexWrap::Wrap,
                justify_content: JustifyContent::Center,
                column_gap: Val::Px(8.0),
                row_gap: Val::Px(8.0),
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|row| {
                let f = &font.0;
                tool(row, f, Action::View(View::Dibujo), images.add(textures::tool_icon("dibujar")), "dibujar");
                tool(row, f, Action::View(View::Buceo), images.add(textures::tool_icon("bucear")), "bucear");
                tool(row, f, Action::Undo, images.add(textures::tool_icon("deshacer")), "deshacer");
                tool(row, f, Action::Gallery, images.add(textures::tool_icon("galeria")), "galería");
                tool(row, f, Action::Contemplate, images.add(textures::tool_icon("contemplar")), "contemplar");
                // colores (solo con pinceles de color): a la derecha; si no
                // caben (vertical) pasan al renglón siguiente
                row.spawn((
                    SwatchRow,
                    Node {
                        column_gap: Val::Px(8.0),
                        align_items: AlignItems::Center,
                        margin: UiRect::left(Val::Px(16.0)),
                        ..default()
                    },
                ))
                .with_children(|sw| {
                    for (i, c) in SWATCHES.iter().enumerate() {
                        sw.spawn((
                            Action::Swatch(i),
                            Node {
                                width: Val::Px(30.0),
                                height: Val::Px(30.0),
                                border: UiRect::all(Val::Px(2.0)),
                                border_radius: BorderRadius::all(Val::Px(999.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(c[0], c[1], c[2], 0.9)),
                            BorderColor::all(BORDER),
                        ));
                    }
                });
            });
            // fila de pinceles: un solo renglón que se desplaza con el dedo
            // (o la rueda); lo que no cabe asoma cortado en el borde
            col.spawn((
                BrushRow,
                Node {
                    width: Val::Percent(100.0),
                    overflow: Overflow::scroll_x(),
                    column_gap: Val::Px(6.0),
                    padding: UiRect::horizontal(Val::Px(6.0)),
                    align_items: AlignItems::Center,
                    ..default()
                },
                ScrollPosition::DEFAULT,
            ))
            .observe(
                |drag: On<bevy::picking::events::PointerDrag>,
                 mut q: Query<&mut ScrollPosition, With<BrushRow>>,
                 mut swipe: ResMut<Swipe>| {
                    if let Ok(mut sp) = q.get_mut(drag.event().entity) {
                        sp.0.x = (sp.0.x - drag.event().delta.x).max(0.0);
                        swipe.moved += drag.event().delta.x.abs();
                    }
                },
            )
            .observe(
                |press: On<bevy::picking::events::PointerPress>, mut swipe: ResMut<Swipe>| {
                    let _ = press;
                    swipe.moved = 0.0;
                },
            )
            .observe(
                |scroll: On<bevy::picking::events::PointerScroll>,
                 mut q: Query<&mut ScrollPosition, With<BrushRow>>| {
                    if let Ok(mut sp) = q.get_mut(scroll.event().entity) {
                        let step = match scroll.event().unit {
                            bevy::input::mouse::MouseScrollUnit::Line => 40.0,
                            bevy::input::mouse::MouseScrollUnit::Pixel => 1.0,
                        };
                        sp.0.x = (sp.0.x - (scroll.event().y + scroll.event().x) * step).max(0.0);
                    }
                },
            )
            .with_children(|row| {
                for (b, img, aspect) in icons {
                    let mut btn = row.spawn((
                        Action::Pick(b),
                        Node {
                            width: Val::Px(d),
                            height: Val::Px(d),
                            // en la fila desplazable no se encogen
                            flex_shrink: 0.0,
                            border: UiRect::all(Val::Px(1.0)),
                            border_radius: BorderRadius::all(Val::Px(999.0)),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(BTN),
                        BorderColor::all(BORDER),
                    ));
                    if b == Brush::Color {
                        btn.with_children(|c| {
                            c.spawn((
                                ColorDot,
                                bevy::picking::Pickable::IGNORE,
                                Node {
                                    width: Val::Px(d * 0.5),
                                    height: Val::Px(d * 0.5),
                                    border_radius: BorderRadius::all(Val::Px(999.0)),
                                    ..default()
                                },
                                BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.9)),
                            ));
                        });
                    } else {
                        // la imagen cabe en un cuadro del 70 % del botón
                        let box_ = d * 0.7;
                        let (w, h) = if aspect >= 1.0 {
                            (box_, box_ / aspect)
                        } else {
                            (box_ * aspect, box_)
                        };
                        btn.with_children(|c| {
                            c.spawn((
                                bevy::picking::Pickable::IGNORE,
                                ImageNode::new(img),
                                Node {
                                    width: Val::Px(w),
                                    height: Val::Px(h),
                                    ..default()
                                },
                            ));
                        });
                    }
                }
            });
        });
    commands
        .spawn((
            LayerCol,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(14.0),
                bottom: Val::Px(palette_h() + 12.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                ..default()
            },
        ))
        .with_children(|col| {
            for (l, k) in [
                (Layer::Superficie, "capa_superficie"),
                (Layer::Agua, "capa_agua"),
                (Layer::Fondo, "capa_fondo"),
            ] {
                tool(col, &font.0, Action::Layer(l), images.add(textures::tool_icon(k)), l.name());
            }
        });
    commands
        .spawn((
            WaterRow,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(14.0),
                top: Val::Px(12.0),
                column_gap: Val::Px(6.0),
                ..default()
            },
        ))
        .with_children(|row| {
            for w in Water::ALL {
                let lk = crate::water::raw_look(w);
                // las aguas son mundos: un disco con su cielo y su abismo
                let img = images.add(textures::water_icon(lk.top, lk.bottom));
                let d = icon_d() * 0.8;
                row.spawn((
                    Action::Water(w),
                    Node {
                        width: Val::Px(d),
                        height: Val::Px(d),
                        border: UiRect::all(Val::Px(1.0)),
                        border_radius: BorderRadius::all(Val::Px(999.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(BTN),
                    BorderColor::all(BORDER),
                ))
                .with_children(|c| {
                    c.spawn((
                        bevy::picking::Pickable::IGNORE,
                        ImageNode::new(img),
                        Node {
                            width: Val::Px(d * 0.78),
                            height: Val::Px(d * 0.78),
                            ..default()
                        },
                    ));
                });
            }
        });
    commands
        .spawn((
            ExitBtn,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(14.0),
                top: Val::Px(12.0),
                display: Display::None,
                ..default()
            },
        ))
        .with_children(|row| {
            tool(row, &font.0, Action::Contemplate, images.add(textures::tool_icon("salir")), "salir");
        });
    commands.spawn((
        Hint,
        // la ayuda no intercepta toques: tapaba la fila de aguas en el teléfono
        bevy::picking::Pickable::IGNORE,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(14.0),
            top: Val::Px(12.0),
            right: Val::Px(14.0),
            ..default()
        },
        Text::new(""),
        TextFont {
            font: font.0.clone().into(),
            font_size: FontSize::Px(font_px()),
            ..default()
        },
        TextColor(Color::srgba(0.8, 0.9, 0.95, 0.75)),
        TextShadow {
            offset: Vec2::new(1.0, 1.0),
            color: Color::srgba(0.0, 0.0, 0.0, 0.55),
        },
    ));
}

fn spacer(row: &mut ChildSpawnerCommands) {
    row.spawn(Node {
        width: Val::Px(24.0),
        ..default()
    });
}

/// Botón de herramienta: cuadrado redondeado con un icono de línea. Se
/// distingue a simple vista de los pinceles (círculos con la imagen).
fn tool(
    row: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    action: Action,
    img: Handle<Image>,
    label: &str,
) {
    let d = icon_d() * 0.86;
    let fs = font_px() * 0.78;
    row.spawn((
        action,
        Node {
            min_width: Val::Px(d * 1.25),
            height: Val::Px(d + fs + 6.0),
            padding: UiRect::axes(Val::Px(6.0), Val::Px(3.0)),
            border: UiRect::all(Val::Px(1.0)),
            border_radius: BorderRadius::all(Val::Px(d * 0.24)),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            row_gap: Val::Px(1.0),
            flex_shrink: 0.0,
            ..default()
        },
        BackgroundColor(BTN),
        BorderColor::all(BORDER),
    ))
    .with_children(|c| {
        c.spawn((
            bevy::picking::Pickable::IGNORE,
            ImageNode::new(img).with_color(TXT),
            Node {
                width: Val::Px(d * 0.58),
                height: Val::Px(d * 0.58),
                ..default()
            },
        ));
        c.spawn((
            bevy::picking::Pickable::IGNORE,
            Text::new(label.to_string()),
            TextFont {
                font: font.clone().into(),
                font_size: FontSize::Px(fs),
                ..default()
            },
            TextColor(TXT),
        ));
    });
}

#[allow(dead_code)]
fn button(row: &mut ChildSpawnerCommands, font: &Handle<Font>, action: Action, label: &str) {
    row.spawn((
        action,
        Node {
            padding: UiRect::axes(Val::Px(font_px() * 0.9), Val::Px(font_px() * 0.55)),
            border: UiRect::all(Val::Px(1.0)),
            border_radius: BorderRadius::all(Val::Px(999.0)),
            ..default()
        },
        BackgroundColor(BTN),
        BorderColor::all(BORDER),
    ))
    .with_children(|b| {
        b.spawn((
            Text::new(label.to_string()),
            TextFont {
                font: font.clone().into(),
                font_size: FontSize::Px(font_px()),
                ..default()
            },
            TextColor(TXT),
            TextShadow {
                offset: Vec2::new(1.0, 1.0),
                color: Color::srgba(0.0, 0.0, 0.0, 0.45),
            },
        ));
    });
}

/// Observador global de clics sobre la paleta.
pub fn on_click(
    click: On<PointerClick>,
    q: Query<&Action>,
    mut st: ResMut<DrawState>,
    mut obra: ResMut<Obra>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    swipe: Res<Swipe>,
) {
    let Ok(action) = q.get(click.event().entity) else {
        return;
    };
    match action {
        Action::Pick(b) => {
            if swipe.moved > 12.0 {
                return;
            }
            st.brush = *b;
            st.title_dirty = true;
        }
        Action::Water(w) => {
            if obra.scene.water != *w {
                obra.scene.water = *w;
                obra.dirty = true;
                st.title_dirty = true;
            }
        }
        Action::Contemplate => {
            st.contemplate = !st.contemplate;
            if st.contemplate {
                st.view = View::Buceo;
            }
            st.title_dirty = true;
        }
        Action::View(v) => {
            st.view = *v;
            st.title_dirty = true;
        }
        Action::Layer(l) => {
            st.layer = *l;
            st.title_dirty = true;
        }
        Action::Swatch(i) => {
            st.swatch = *i;
            st.brush = Brush::Color;
            st.title_dirty = true;
        }
        // deshacer vive en draw::keys; se simula la tecla
        Action::Undo => keys.press(KeyCode::KeyZ),
        Action::Gallery => keys.press(KeyCode::KeyG),
    }
}

/// Colores de la paleta, visibilidad en contemplación y el texto de ayuda.
pub fn refresh(
    time: Res<Time>,
    st: Res<DrawState>,
    obra: Res<Obra>,
    gallery: Res<crate::gallery::Gallery>,
    mut buttons: Query<(&Action, &mut BackgroundColor, &mut BorderColor, &mut Node)>,
    mut root: Query<&mut Node, (With<PaletteRoot>, Without<WaterRow>, Without<Action>)>,
    mut waters: Query<&mut Node, (With<WaterRow>, Without<PaletteRoot>, Without<Action>)>,
    mut layers: Query<
        &mut Node,
        (
            With<LayerCol>,
            Without<PaletteRoot>,
            Without<WaterRow>,
            Without<Action>,
        ),
    >,
    palette: Query<&ComputedNode, With<PaletteRoot>>,
    mut hint_since: Local<f32>,
    mut hint_last: Local<String>,
    mut swatches: Query<
        &mut Node,
        (
            With<SwatchRow>,
            Without<LayerCol>,
            Without<PaletteRoot>,
            Without<WaterRow>,
            Without<Action>,
        ),
    >,
    mut hint: Query<(&mut Text, &mut Node), (With<Hint>, Without<Action>, Without<LayerCol>, Without<PaletteRoot>, Without<WaterRow>, Without<SwatchRow>, Without<ExitBtn>)>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut dot: Query<&mut BackgroundColor, (With<ColorDot>, Without<Action>)>,
    mut exit: Query<
        &mut Node,
        (
            With<ExitBtn>,
            Without<SwatchRow>,
            Without<LayerCol>,
            Without<PaletteRoot>,
            Without<WaterRow>,
            Without<Action>,
        ),
    >,
) {
    if let Ok(mut node) = exit.single_mut() {
        let show = st.contemplate && time.elapsed_secs() < st.exit_until;
        let want = if show { Display::Flex } else { Display::None };
        if node.display != want {
            node.display = want;
        }
    }
    if let Ok(mut bg) = dot.single_mut() {
        let c = SWATCHES[st.swatch];
        bg.0 = Color::srgba(c[0], c[1], c[2], 0.95);
    }
    for (action, mut bg, mut border, mut node) in buttons.iter_mut() {
        let on = match action {
            Action::Pick(b) => *b == st.brush,
            // el botón de vista muestra la OTRA vista: un solo botón alterna
            Action::View(v) if *v == st.view => {
                node.display = Display::None;
                continue;
            }
            Action::View(_) => {
                node.display = Display::Flex;
                false
            }
            Action::Water(w) => *w == obra.scene.water,
            Action::Layer(l) => *l == st.layer,
            Action::Swatch(i) => *i == st.swatch && st.brush.uses_color(),
            Action::Contemplate => st.contemplate,
            Action::Undo | Action::Gallery => false,
        };
        if !matches!(action, Action::Swatch(_)) {
            bg.0 = if on { BTN_ON } else { BTN };
        }
        *border = BorderColor::all(if on { BORDER_ON } else { BORDER });
    }
    for mut node in root.iter_mut() {
        node.display = if st.contemplate {
            Display::None
        } else {
            Display::Flex
        };
    }
    for mut node in waters.iter_mut() {
        node.display = if st.contemplate || st.view != View::Dibujo {
            Display::None
        } else {
            Display::Flex
        };
    }
    for mut node in swatches.iter_mut() {
        node.display = if st.contemplate || !st.brush.uses_color() {
            Display::None
        } else {
            Display::Flex
        };
    }
    let ph = palette
        .single()
        .map(|n| n.size().y * n.inverse_scale_factor())
        .unwrap_or_else(|_| palette_h());
    for mut node in layers.iter_mut() {
        node.display = if st.contemplate || st.view != View::Dibujo {
            Display::None
        } else {
            Display::Flex
        };
        // siempre justo encima de la paleta, mida lo que mida
        node.bottom = Val::Px(ph + 12.0);
    }
    if let Ok((mut text, mut hnode)) = hint.single_mut() {
        // en pantallas bajas (teléfono horizontal) la columna de capas sube
        // hasta la ayuda: la ayuda se corre a la derecha
        let short = windows.single().map(|w| w.height() < 560.0).unwrap_or(false);
        let layers_on = !st.contemplate && st.view == View::Dibujo;
        hnode.left = Val::Px(if short && layers_on { 118.0 } else { 14.0 });
        if st.contemplate {
            // contemplar es solo la obra
            if !text.0.is_empty() {
                text.0.clear();
            }
            return;
        }
        let titulo = if gallery.editing_title {
            format!("título: {}_   (Enter termina)", obra.scene.title)
        } else {
            if cfg!(target_os = "android") {
                crate::gallery::title_of(&obra)
            } else {
                format!("{} (T renombra)", crate::gallery::title_of(&obra))
            }
        };
        let s = if st.contemplate {
            format!(
                "contemplación{SEP}{} trazos{SEP}toca un elemento para que actúe{SEP}A o Esc vuelve a dibujar",
                obra.scene.strokes.len()
            )
        } else {
            let ayuda = if cfg!(target_os = "android") {
                "un dedo dibuja{SEP}dos dedos orbitan"
            } else {
                "[ ] capa o plano{SEP}V vista{SEP}izquierdo dibuja{SEP}derecho orbita{SEP}Z deshacer{SEP}C vaciar{SEP}S guardar{SEP}E exportar audio{SEP}N nueva{SEP}G galería{SEP}A contemplar"
            }
            .replace("{SEP}", SEP);
            match st.view {
                View::Dibujo => format!(
                    "{}{SEP}capa: {}{SEP}{ayuda}",
                    brush_name(st.brush),
                    st.layer.name()
                ),
                View::Buceo => format!(
                    "{}{SEP}plano a {:.0}{SEP}{ayuda}",
                    brush_name(st.brush),
                    st.depth
                ),
            }
        };
        let s = format!("{titulo}\n{s}\n{}", gallery.status);
        if *hint_last != s {
            *hint_last = s.clone();
            text.0 = s;
            *hint_since = time.elapsed_secs();
        } else if cfg!(target_os = "android")
            && !text.0.is_empty()
            && time.elapsed_secs() - *hint_since > 7.0
        {
            // en el teléfono la ayuda se retira sola: pantalla limpia
            text.0.clear();
        }
    }
}
