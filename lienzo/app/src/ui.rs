//! La paleta: una fila de pinceles abajo y un botón de contemplación. En
//! contemplación la paleta se esconde; solo queda la obra.

use crate::draw::{brush_name, DrawState};
use crate::Obra;
use bevy::picking::events::PointerClick;
use bevy::prelude::*;
use bevy::text::FontSize;
use lienzo_core::Brush;

pub const PALETTE_H: f32 = 84.0;

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

pub fn setup(mut commands: Commands) {
    commands
        .spawn((
            PaletteRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                bottom: Val::Px(0.0),
                height: Val::Px(PALETTE_H),
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
                    Action::Pick(*b),
                    &format!("{} {}", i + 1, brush_name(*b)),
                );
            }
            spacer(row);
            button(row, Action::Undo, "deshacer");
            button(row, Action::Gallery, "galería");
            button(row, Action::Contemplate, "contemplar");
        });
    commands.spawn((
        Hint,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(14.0),
            top: Val::Px(12.0),
            ..default()
        },
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(14.0),
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

fn button(row: &mut ChildSpawnerCommands, action: Action, label: &str) {
    row.spawn((
        action,
        Node {
            padding: UiRect::axes(Val::Px(12.0), Val::Px(9.0)),
            border_radius: BorderRadius::all(Val::Px(8.0)),
            ..default()
        },
        BackgroundColor(BTN),
    ))
    .with_children(|b| {
        b.spawn((
            Text::new(label.to_string()),
            TextFont {
                font_size: FontSize::Px(14.0),
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
            format!("{} (T renombra)", crate::gallery::title_of(&obra))
        };
        let s = if st.contemplate {
            format!(
                "contemplación · {} trazos · toca un elemento para que actúe · botón derecho orbita · A o Esc vuelve a dibujar",
                obra.scene.strokes.len()
            )
        } else {
            format!(
                "{} · plano a {:.0} ([ ] ajusta) · izquierdo dibuja · derecho orbita · Z deshacer · C vaciar · S guardar · N nueva · G galería · A contemplar",
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
