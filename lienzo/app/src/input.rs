//! Un solo puntero para ratón y dedos. El resto de la app no sabe cuál es.
//!
//! Ratón: izquierdo dibuja/toca · derecho o central orbita · rueda acerca.
//! Táctil: un dedo dibuja/toca · dos dedos orbitan y la pinza acerca. Si cae un
//! segundo dedo a media línea, el trazo se cancela (era una órbita).

use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::input::touch::Touches;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

#[derive(Resource, Default, Debug)]
pub struct Pointer {
    /// Posición del puntero principal en la ventana (px lógicos).
    pub pos: Option<Vec2>,
    pub down: bool,
    pub just_down: bool,
    pub just_up: bool,
    /// El gesto en curso dejó de ser un trazo (entró un segundo dedo).
    pub cancel: bool,
    /// Arrastre de órbita en este fotograma.
    pub orbit: Vec2,
    /// Cambio de distancia de cámara en este fotograma (+ acerca).
    pub zoom: f32,
    pub is_touch: bool,
}

#[derive(Default)]
pub struct TouchState {
    fingers: usize,
    pinch_dist: Option<f32>,
    /// Tras una órbita a dos dedos no se dibuja hasta soltar todo.
    locked: bool,
}

pub fn gather(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut motion: MessageReader<MouseMotion>,
    mut wheel: MessageReader<MouseWheel>,
    touches: Res<Touches>,
    mut p: ResMut<Pointer>,
    mut ts: Local<TouchState>,
) {
    *p = Pointer::default();
    let pressed: Vec<&bevy::input::touch::Touch> = touches.iter().collect();
    let n = pressed.len();

    if n > 0 || ts.fingers > 0 {
        p.is_touch = true;
        if n >= 2 {
            let a = pressed[0];
            let b = pressed[1];
            let delta = (a.delta() + b.delta()) * 0.5;
            let dist = a.position().distance(b.position());
            if ts.fingers < 2 {
                // acaba de entrar el segundo dedo
                if ts.fingers == 1 {
                    p.cancel = true;
                }
                ts.pinch_dist = Some(dist);
            } else {
                p.orbit = delta;
                if let Some(d0) = ts.pinch_dist {
                    p.zoom = (dist - d0) * 0.02;
                }
                ts.pinch_dist = Some(dist);
            }
            ts.locked = true;
        } else if n == 1 {
            let t = pressed[0];
            p.pos = Some(t.position());
            if !ts.locked {
                p.down = true;
                p.just_down = ts.fingers == 0;
            }
        } else {
            // n == 0: se soltó todo
            if ts.fingers == 1 && !ts.locked {
                p.just_up = true;
                p.pos = touches.iter_just_released().next().map(|t| t.position());
            }
            ts.locked = false;
            ts.pinch_dist = None;
        }
        ts.fingers = n;
        // los mensajes de ratón se descartan para no mezclar
        motion.clear();
        wheel.clear();
        return;
    }

    // ratón
    p.pos = windows.single().ok().and_then(|w| w.cursor_position());
    p.down = mouse.pressed(MouseButton::Left);
    p.just_down = mouse.just_pressed(MouseButton::Left);
    p.just_up = mouse.just_released(MouseButton::Left);
    let mut delta = Vec2::ZERO;
    for m in motion.read() {
        delta += m.delta;
    }
    if mouse.pressed(MouseButton::Right) || mouse.pressed(MouseButton::Middle) {
        p.orbit = delta;
    }
    for w in wheel.read() {
        p.zoom += w.y * 0.8;
    }
}
