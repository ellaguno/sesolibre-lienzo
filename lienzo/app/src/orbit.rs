//! Cámara orbital acotada. Arrastrar gira, rueda acerca. Nunca sale del
//! volumen ni mira desde abajo del suelo.

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
        }
    }
}

pub const YAW_LIMIT: f32 = 1.15;
pub const PITCH_MIN: f32 = -0.05;
pub const PITCH_MAX: f32 = 0.9;
pub const DIST_MIN: f32 = 7.0;
pub const DIST_MAX: f32 = 22.0;

pub fn control(
    ptr: Res<Pointer>,
    time: Res<Time>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut q: Query<(&mut Orbit, &mut Transform)>,
) {
    let Ok((mut o, mut tf)) = q.single_mut() else {
        return;
    };
    let delta = ptr.orbit;
    if delta != Vec2::ZERO {
        o.yaw -= delta.x * 0.004;
        o.pitch += delta.y * 0.004;
        o.idle = 0.0;
    } else {
        o.idle += time.delta_secs();
    }
    o.distance -= ptr.zoom;
    if o.drift && o.idle > 8.0 {
        o.yaw += 0.025 * time.delta_secs() * (time.elapsed_secs() * 0.07).cos();
    }
    o.yaw = o.yaw.clamp(-YAW_LIMIT, YAW_LIMIT);
    o.pitch = o.pitch.clamp(PITCH_MIN, PITCH_MAX);
    o.distance = o.distance.clamp(DIST_MIN, DIST_MAX);

    // En vertical (teléfono) la cámara retrocede para que quepa el ancho.
    let portrait = windows
        .single()
        .map(|w| (w.height() / w.width()).max(1.0))
        .unwrap_or(1.0);
    let dist = o.distance * (1.0 + 0.55 * (portrait - 1.0));
    let pos = o.target
        + Vec3::new(
            o.yaw.sin() * o.pitch.cos(),
            o.pitch.sin(),
            o.yaw.cos() * o.pitch.cos(),
        ) * dist;
    *tf = Transform::from_translation(pos).looking_at(o.target, Vec3::Y);
}
