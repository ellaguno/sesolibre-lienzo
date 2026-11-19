//! La paleta: una fila de pinceles abajo y un botón de contemplación. En
//! contemplación la paleta se esconde; solo queda la obra.

use crate::draw::{brush_name, DrawState};
use crate::Obra;
use bevy::picking::events::PointerClick;
use bevy::prelude::*;
use bevy::text::FontSize;
use lienzo_core::Brush;

/// Alto de la franja de la paleta. En pantalla táctil es más alta: botones de
/// dedo y dos filas.
pub fn palette_h() -> f32 {
    if cfg!(target_os = "android") {
        230.0
    } else {
        84.0
    }
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
    Contemplate,
    Undo,
    Gallery,
}

#[derive(Component)]
pub struct Hint;

const BG: Color = Color::srgba(0.02, 0.08, 0.16, 0.72);
const BTN: Color = Color::srgba(0.10, 0.26, 0.38, 0.9);
const BTN_ON: Color = Color::srgba(0.85, 0.55, 0.30, 0.95);
const TXT: Color = Color::srgb(0.88, 0.94, 0.96);

/// La fuente de toda la interfaz (Noto Sans, OFL).
#[derive(Resource)]
pub struct UiFont(pub Handle<Font>);

pub fn load_font(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(UiFont(assets.load("fonts/NotoSans-Variable.ttf")));
}

pub fn setup(mut commands: Commands, font: Res<UiFont>) {
    commands
        .spawn((
            PaletteRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                bottom: Val::Px(0.0),
                height: Val::Px(palette_h()),
                flex_wrap: FlexWrap::Wrap,
                row_gap: Val::Px(8.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                column_gap: Val::Px(8.0),
                ..default()
            },
            BackgroundColor(BG),
        ))
        .with_children(|row| {
            for (i, b) in Brush::ALL.iter().enumerate() {
                button(
                    row,
                    &font.0,
                    Action::Pick(*b),
                    &format!("{} {}", i + 1, brush_name(*b)),
                );
            }
            spacer(row);
            button(row, &font.0, Action::Undo, "deshacer");
            button(row, &font.0, Action::Gallery, "galería");
            button(row, &font.0, Action::Contemplate, "contemplar");
        });
    commands.spawn((
        Hint,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(14.0),
            top: Val::Px(if cfg!(target_os = "android") {
                56.0
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
            padding: UiRect::axes(Val::Px(font_px() * 0.9), Val::Px(font_px() * 0.65)),
            border_radius: BorderRadius::all(Val::Px(8.0)),
            ..default()
        },
        BackgroundColor(BTN),
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
        ));
    });
}

/// Observador global de clics sobre la paleta.
pub fn on_click(
    click: On<PointerClick>,
    q: Query<&Action>,
    mut st: ResMut<DrawState>,
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
        Action::Contemplate => {
            st.contemplate = !st.contemplate;
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
    mut buttons: Query<(&Action, &mut BackgroundColor)>,
    mut root: Query<&mut Node, With<PaletteRoot>>,
    mut hint: Query<&mut Text, With<Hint>>,
) {
    for (action, mut bg) in buttons.iter_mut() {
        let on = match action {
            Action::Pick(b) => *b == st.brush,
            Action::Contemplate => st.contemplate,
            Action::Undo | Action::Gallery => false,
        };
        bg.0 = if on { BTN_ON } else { BTN };
    }
    if let Ok(mut node) = root.single_mut() {
        node.display = if st.contemplate {
            Display::None
        } else {
            Display::Flex
        };
    }
    if let Ok(mut text) = hint.single_mut() {
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
                "([ ] ajusta){SEP}izquierdo dibuja{SEP}derecho orbita{SEP}Z deshacer{SEP}C vaciar{SEP}S guardar{SEP}E exportar audio{SEP}N nueva{SEP}G galería{SEP}A contemplar"
            }
            .replace("{SEP}", SEP);
            format!(
                "{}{SEP}plano a {:.0} {ayuda}",
                brush_name(st.brush),
                st.depth
            )
        };
        let s = format!("{titulo}\n{s}\n{}", gallery.status);
        if text.0 != s {
            text.0 = s;
        }
    }
}
