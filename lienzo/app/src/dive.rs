//! El buceo: en contemplación la cámara nada por la ruta de la obra (la
//! dibujada o la automática) en bucle, mirando hacia donde avanza. La cámara
//! es la posición de escucha: el motor recibe dónde está y hacia dónde mira.

use crate::audio::AudioLink;
use crate::draw::{DrawState, View};
use crate::input::Pointer;
use crate::orbit::Orbit;
use crate::world::{to_core, to_world};
use crate::Obra;
use bevy::prelude::*;
use lienzo_core::route;

/// Velocidad de nado, en unidades de mundo por segundo.
pub const SPEED: f32 = 1.1;

#[derive(Resource)]
pub struct Dive {
    pub trace: f32,
    pub active: bool,
    /// Posición a lo largo de la ruta, 0..1.
    pub u: f32,
    /// Segundos desde el último gesto del usuario durante el buceo.
    idle: f32,
    /// Mirada suavizada.
    look: Vec3,
    pos: Vec3,
    route: Vec<lienzo_core::Point>,
    route_len: f32,
    key: usize,
}

impl Default for Dive {
    fn default() -> Self {
        Dive {
            trace: 0.0,
            active: false,
            u: std::env::var("LIENZO_DIVE_U")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.0),
            idle: 99.0,
            look: Vec3::new(0.0, 2.0, -1.0),
            pos: Vec3::new(0.0, 2.5, 8.0),
            route: vec![],
            route_len: 1.0,
            key: usize::MAX,
        }
    }
}

/// Nada por la ruta mientras se contempla. Un gesto de órbita suspende el
/// nado (la cámara queda donde está) y a los 8 s se retoma.
pub fn swim(
    time: Res<Time>,
    obra: Res<Obra>,
    st: Res<DrawState>,
    ptr: Res<Pointer>,
    mut dive: ResMut<Dive>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut q: Query<(&mut Orbit, &mut Transform), With<Camera3d>>,
) {
    let Ok((mut orbit, mut tf)) = q.single_mut() else {
        return;
    };
    let wants = st.contemplate && st.view == View::Buceo;
    if !wants {
        if dive.active {
            dive.active = false;
            orbit.adopt(&tf);
        }
        return;
    }
    if ptr.orbit != Vec2::ZERO || ptr.zoom != 0.0 {
        dive.idle = 0.0;
        if dive.active {
            dive.active = false;
            orbit.adopt(&tf);
        }
        return;
    }
    dive.idle += time.delta_secs();
    if !dive.active {
        if dive.idle < 8.0 {
            return;
        }
        dive.active = true;
        dive.pos = tf.translation;
        dive.look = tf.translation + tf.forward().as_vec3() * 4.0;
    }
    // la ruta se recalcula si cambió la obra
    let key = obra.scene.strokes.len();
    if dive.key != key {
        dive.key = key;
        dive.route = route::of(&obra.scene);
        dive.route_len = route::length(&dive.route).max(0.5) * crate::world::HALF_W;
    }
    let dt = time.delta_secs();
    dive.u = (dive.u + SPEED * dt / dive.route_len).rem_euclid(1.0);
    let mut target = to_world(route::sample(&dive.route, dive.u));
    let mut ahead = to_world(route::sample(&dive.route, dive.u + 0.04));
    // en vertical (teléfono) el campo visual es estrecho: la ruta se aleja
    // del centro en proporción, como hace la cámara de órbita
    let portrait = windows
        .single()
        .ok()
        .filter(|w| w.width() > 1.0 && w.height() > 1.0)
        .map(|w| (w.height() / w.width()).clamp(1.0, 3.0))
        .unwrap_or(1.0);
    // y siempre un poco por fuera del lienzo: se contempla la obra entera
    let wide = 1.4 * (1.0 + 0.45 * (portrait - 1.0));
    target.x *= wide;
    target.z *= wide;
    ahead.x *= wide;
    ahead.z *= wide;
    // nunca dentro del relieve: al menos 1.7 por encima del suelo
    let floor = crate::terrain::floor_y(&obra.scene, target.x, target.z);
    target.y = target.y.max(floor + 2.3);
    // la mirada va por delante y se vuelve hacia la obra (el centro), un
    // poco por debajo de la cámara: se contempla, no se mira la arena
    let center = Vec3::new(0.0, (target.y - 1.2).max(floor + 1.0), 0.0);
    ahead = ahead * 0.55 + center * 0.45;
    ahead.y = ahead.y.min(target.y - 0.2);
    // suavidad: la cámara persigue la ruta, la mirada va un poco por delante
    let k = 1.0 - (-2.5 * dt).exp();
    let (p0, l0) = (dive.pos, dive.look);
    dive.pos = p0 + (target - p0) * k;
    dive.look = l0 + (ahead - l0) * (1.0 - (-1.8 * dt).exp());
    let pos = dive.pos;
    let look = dive.look;
    *tf = Transform::from_translation(pos).looking_at(look, Vec3::Y);
    if std::env::var("LIENZO_TRACE").is_ok() {
        dive.trace += dt;
        if dive.trace > 3.0 {
            dive.trace = 0.0;
            info!(
                "buceo u={:.2} wide={:.2} pos=({:.1},{:.1},{:.1}) target=({:.1},{:.1},{:.1}) ruta={} pts",
                dive.u, wide, pos.x, pos.y, pos.z, target.x, target.y, target.z, dive.route.len()
            );
        }
    }
}

/// Avisa al motor dónde está el oído.
pub fn listener(
    link: Res<AudioLink>,
    dive: Res<Dive>,
    st: Res<DrawState>,
    mut last: Local<f32>,
    time: Res<Time>,
    q: Query<&Transform, With<Camera3d>>,
) {
    *last += time.delta_secs();
    if *last < 0.1 {
        return;
    }
    *last = 0.0;
    let Ok(tf) = q.single() else { return };
    let c = to_core(tf.translation);
    let f = tf.forward().as_vec3();
    let yaw = f.x.atan2(-f.z);
    // la espacialización plena solo al bucear por la ruta; al orbitar, media
    let amount = if dive.active {
        1.0
    } else if st.view == View::Buceo {
        0.5
    } else {
        0.0
    };
    link.set_listener(lienzo_core::Listener {
        x: c.x,
        z: c.z,
        yaw,
        amount,
    });
}
