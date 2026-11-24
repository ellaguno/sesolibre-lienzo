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
            // colores (solo con pinceles de color)
            col.spawn((
                SwatchRow,
                Node {
                    width: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    column_gap: Val::Px(8.0),
                    ..default()
                },
            ))
            .with_children(|row| {
                for (i, c) in SWATCHES.iter().enumerate() {
                    row.spawn((
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
                button(row, &font.0, Action::View(View::Dibujo), "dibujar");
                button(row, &font.0, Action::View(View::Buceo), "bucear");
                spacer(row);
                button(row, &font.0, Action::Undo, "deshacer");
                button(row, &font.0, Action::Gallery, "galería");
                button(row, &font.0, Action::Contemplate, "contemplar");
            });
            // fila de iconos (en vertical se parte en dos)
            col.spawn(Node {
                width: Val::Percent(100.0),
                flex_wrap: FlexWrap::Wrap,
                justify_content: JustifyContent::Center,
                column_gap: Val::Px(6.0),
                row_gap: Val::Px(6.0),
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|row| {
                for (b, img, aspect) in icons {
                    let mut btn = row.spawn((
                        Action::Pick(b),
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
                top: Val::Percent(40.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                ..default()
            },
        ))
        .with_children(|col| {
            for l in [Layer::Superficie, Layer::Agua, Layer::Fondo] {
                button(col, &font.0, Action::Layer(l), l.name());
            }
        });
    commands
        .spawn((
            WaterRow,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(14.0),
                top: Val::Px(if cfg!(target_os = "android") {
                    56.0
                } else {
                    12.0
                }),
                column_gap: Val::Px(6.0),
                ..default()
            },
        ))
        .with_children(|row| {
            for w in Water::ALL {
                button(row, &font.0, Action::Water(w), w.name());
            }
        });
    commands.spawn((
        Hint,
        // la ayuda no intercepta toques: tapaba la fila de aguas en el teléfono
        bevy::picking::Pickable::IGNORE,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(14.0),
            top: Val::Px(if cfg!(target_os = "android") {
                104.0
            } else {
                12.0
            }),
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
) {
    let Ok(action) = q.get(click.event().entity) else {
        return;
    };
    match action {
        Action::Pick(b) => {
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
    st: Res<DrawState>,
    obra: Res<Obra>,
    gallery: Res<crate::gallery::Gallery>,
    mut buttons: Query<(&Action, &mut BackgroundColor, &mut BorderColor)>,
    mut root: Query<&mut Node, Or<(With<PaletteRoot>, With<WaterRow>)>>,
    mut layers: Query<&mut Node, (With<LayerCol>, Without<PaletteRoot>, Without<WaterRow>)>,
    mut swatches: Query<
        &mut Node,
        (
            With<SwatchRow>,
            Without<LayerCol>,
            Without<PaletteRoot>,
            Without<WaterRow>,
        ),
    >,
    mut hint: Query<&mut Text, With<Hint>>,
    mut dot: Query<&mut BackgroundColor, (With<ColorDot>, Without<Action>)>,
) {
    if let Ok(mut bg) = dot.single_mut() {
        let c = SWATCHES[st.swatch];
        bg.0 = Color::srgba(c[0], c[1], c[2], 0.95);
    }
    for (action, mut bg, mut border) in buttons.iter_mut() {
        let on = match action {
            Action::Pick(b) => *b == st.brush,
            Action::Water(w) => *w == obra.scene.water,
            Action::View(v) => *v == st.view,
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
    for mut node in swatches.iter_mut() {
        node.display = if st.contemplate || !st.brush.uses_color() {
            Display::None
        } else {
            Display::Flex
        };
    }
    for mut node in layers.iter_mut() {
        node.display = if st.contemplate || st.view != View::Dibujo {
            Display::None
        } else {
            Display::Flex
        };
    }
    if let Ok(mut text) = hint.single_mut() {
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
        if text.0 != s {
            text.0 = s;
        }
    }
}
