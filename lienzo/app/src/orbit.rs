//! Cámara orbital acotada. Arrastrar gira, rueda acerca. Nunca sale del
//! volumen ni mira desde abajo del suelo.

use crate::draw::{DrawState, View};
use crate::input::Pointer;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

#[derive(Component)]
pub struct Orbit {
    pub target: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    /// Deriva automática (modo contemplación).
    pub drift: bool,
    /// Segundos desde la última órbita manual.
    pub idle: f32,
    /// Distancia elegida en la vista cenital (la rueda/pinza la ajustan).
    pub top_distance: f32,
    /// Inclinación elegida en el buceo.
    pub dive_pitch: f32,
}

impl Default for Orbit {
    fn default() -> Self {
        Orbit {
            target: Vec3::new(0.0, 2.2, 0.0),
            yaw: 0.0,
            pitch: 0.12,
            distance: 15.0,
            drift: true,
            idle: 99.0,
            top_distance: 17.0,
            // LIENZO_PITCH=-0.5 arranca mirando hacia arriba (pruebas y capturas)
            dive_pitch: std::env::var("LIENZO_PITCH")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.12),
        }
    }
}

pub const YAW_LIMIT: f32 = 1.15;
pub const PITCH_MIN: f32 = -0.55;
pub const PITCH_MAX: f32 = 0.9;
/// Vista cenital: 65° sobre la horizontal (25° de inclinación respecto a la
/// vertical), lo justo para leer la altura de las cartas.
pub const TOP_PITCH: f32 = 1.13;
pub const DIST_MIN: f32 = 7.0;
pub const DIST_MAX: f32 = 22.0;

pub fn control(
    ptr: Res<Pointer>,
    time: Res<Time>,
    st: Res<DrawState>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut q: Query<(&mut Orbit, &mut Transform)>,
) {
    let Ok((mut o, mut tf)) = q.single_mut() else {
        return;
    };
    let dt = time.delta_secs();
    let delta = ptr.orbit;
    let top = st.view == View::Dibujo;
    if delta != Vec2::ZERO {
        o.yaw -= delta.x * 0.004;
        if !top {
            o.dive_pitch = (o.dive_pitch + delta.y * 0.004).clamp(PITCH_MIN, PITCH_MAX);
        }
        o.idle = 0.0;
    } else {
        o.idle += dt;
    }
    if top {
        o.top_distance = (o.top_distance - ptr.zoom).clamp(10.0, 26.0);
    } else {
        o.distance = (o.distance - ptr.zoom).clamp(DIST_MIN, DIST_MAX);
    }
    if o.drift && o.idle > 8.0 && !top {
        o.yaw += 0.025 * dt * (time.elapsed_secs() * 0.07).cos();
    }
    o.yaw = o.yaw.clamp(-YAW_LIMIT, YAW_LIMIT);

    // La transición entre vistas es la misma cámara que se inclina y
    // desciende: se persigue el objetivo de cada vista con suavidad.
    let (pitch_goal, dist_goal, target_goal) = if top {
        (TOP_PITCH, o.top_distance, Vec3::new(0.0, 0.8, 0.0))
    } else {
        (o.dive_pitch, o.distance, Vec3::new(0.0, 2.2, 0.0))
    };
    let k = 1.0 - (-3.0 * dt).exp();
    o.pitch += (pitch_goal - o.pitch) * k;
    let t_now = o.target;
    o.target = t_now + (target_goal - t_now) * k;
    let dist_now = tf.translation.distance(o.target);
    let dist = dist_now + (dist_goal - dist_now) * k;

    // En vertical (teléfono) la cámara retrocede para que quepa el ancho.
    let portrait = windows
        .single()
        .map(|w| (w.height() / w.width()).max(1.0))
        .unwrap_or(1.0);
    let dist = dist * (1.0 + 0.55 * (portrait - 1.0));
    let pos = o.target
        + Vec3::new(
            o.yaw.sin() * o.pitch.cos(),
            o.pitch.sin(),
            o.yaw.cos() * o.pitch.cos(),
        ) * dist;
    *tf = Transform::from_translation(pos).looking_at(o.target, Vec3::Y);
}
