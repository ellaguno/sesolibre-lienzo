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
            // en el papel es un factor de zoom: 1 = el lienzo cabe justo
            top_distance: 1.0,
            // LIENZO_PITCH=-0.5 arranca mirando hacia arriba (pruebas y capturas)
            dive_pitch: std::env::var("LIENZO_PITCH")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.12),
        }
    }
}

impl Orbit {
    /// Toma como estado propio la pose actual de la cámara, para que al pasar
    /// de nadar a orbitar no haya salto.
    pub fn adopt(&mut self, tf: &Transform) {
        let d = tf.translation - self.target;
        self.distance = d.length().clamp(DIST_MIN, DIST_MAX);
        self.yaw = d.x.atan2(d.z);
        self.dive_pitch = (d.y / d.length().max(0.001))
            .asin()
            .clamp(PITCH_MIN, PITCH_MAX);
        self.pitch = self.dive_pitch;
        self.idle = 0.0;
    }
}

pub const YAW_LIMIT: f32 = std::f32::consts::PI;
pub const PITCH_MIN: f32 = -0.55;
pub const PITCH_MAX: f32 = 0.9;
/// Vista de dibujo: en picado, como un papel plano. Con un campo visual muy
/// estrecho desde lejos ("teleobjetivo") no se percibe perspectiva, y aun
/// así el viaje al contemplar es continuo: el campo se abre mientras baja.
pub const TOP_PITCH: f32 = std::f32::consts::FRAC_PI_2 - 0.002;
/// Campo visual vertical del papel (radianes) y del buceo.
pub const TOP_FOV: f32 = 0.18;
pub const DIVE_FOV: f32 = std::f32::consts::FRAC_PI_4;
pub const DIST_MIN: f32 = 7.0;
pub const DIST_MAX: f32 = 22.0;

pub fn control(
    ptr: Res<Pointer>,
    time: Res<Time>,
    st: Res<DrawState>,
    dive: Res<crate::dive::Dive>,
    obra: Res<crate::Obra>,
    windows: Query<&Window, With<PrimaryWindow>>,
    palette: Query<&ComputedNode, With<crate::ui::PaletteRoot>>,
    mut q: Query<(&mut Orbit, &mut Transform, &mut Projection)>,
) {
    if dive.active {
        return;
    }
    let Ok((mut o, mut tf, mut proj)) = q.single_mut() else {
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
        o.top_distance = (o.top_distance - ptr.zoom * 0.06).clamp(0.5, 1.8);
    } else {
        o.distance = (o.distance - ptr.zoom).clamp(DIST_MIN, DIST_MAX);
    }
    if o.drift && o.idle > 8.0 && !top {
        o.yaw += 0.025 * dt * (time.elapsed_secs() * 0.07).cos();
    }
    o.yaw = o.yaw.clamp(-YAW_LIMIT, YAW_LIMIT);

    // La transición entre vistas es la misma cámara que se inclina y
    // desciende: se persigue el objetivo de cada vista con suavidad.
    let (w, h) = windows
        .single()
        .ok()
        .filter(|w| w.width() > 1.0 && w.height() > 1.0)
        .map(|w| (w.width(), w.height()))
        .unwrap_or((1.0, 1.0));
    let (pitch_goal, dist_goal, target_goal, fov_goal) = if top {
        // el lienzo (±HALF_W) cabe en el área libre: entre la fila de aguas
        // (arriba) y la paleta (abajo), con el campo estrecho
        let ph = palette
            .single()
            .map(|n| n.size().y * n.inverse_scale_factor())
            .unwrap_or_else(|_| crate::ui::palette_h());
        let top_margin = 64.0;
        let free_h = (h - ph - top_margin).max(120.0);
        let half = crate::world::HALF_W * 1.06 * o.top_distance;
        let t = (TOP_FOV * 0.5).tan();
        let d_v = half * (h / free_h) / t; // limitado por el alto libre
        let d_h = half / (t * (w / h)); // limitado por el ancho
        let d = d_v.max(d_h);
        // el centro del área libre está por encima del centro de pantalla:
        // se mira a un punto "más abajo" del lienzo para subirlo
        let upp = 2.0 * d * t / h; // unidades de mundo por píxel
        let shift = (h * 0.5 - (top_margin + free_h * 0.5)) * upp;
        let screen_down = Vec3::new(o.yaw.sin(), 0.0, o.yaw.cos());
        (TOP_PITCH, d, screen_down * shift, TOP_FOV)
    } else {
        (o.dive_pitch, o.distance, Vec3::new(0.0, 2.2, 0.0), DIVE_FOV)
    };
    // al entrar a contemplar la cámara baja despacio: es un viaje
    let rate = if st.contemplate { 1.0 } else { 3.0 };
    let k = 1.0 - (-rate * dt).exp();
    o.pitch += (pitch_goal - o.pitch) * k;
    if let Projection::Perspective(pp) = &mut *proj {
        pp.fov += (fov_goal - pp.fov) * k;
    }
    let t_now = o.target;
    o.target = t_now + (target_goal - t_now) * k;
    // En vertical (teléfono) la cámara retrocede para que quepa el ancho. El
    // factor se aplica al OBJETIVO, nunca a la distancia medida: si se
    // aplicara a la medida, se acumularía cada fotograma y la cámara se iría
    // al infinito (pasó en Android).
    let portrait = (h / w).clamp(1.0, 3.0);
    let dist_goal = if top {
        dist_goal
    } else {
        dist_goal * (1.0 + 0.55 * (portrait - 1.0))
    };
    let mut dist_now = tf.translation.distance(o.target);
    if !dist_now.is_finite() || dist_now < 0.1 || dist_now > 2000.0 {
        dist_now = dist_goal;
    }
    let dist = dist_now + (dist_goal - dist_now) * k;
    let mut pos = o.target
        + Vec3::new(
            o.yaw.sin() * o.pitch.cos(),
            o.pitch.sin(),
            o.yaw.cos() * o.pitch.cos(),
        ) * dist;
    // Nunca bajo la arena ni fuera del agua: en buceo la cámara se queda
    // entre el fondo y la superficie aunque la órbita pida más.
    if !top {
        let floor = crate::terrain::floor_y(&obra.scene, pos.x, pos.z);
        pos.y = pos.y.clamp(floor + 0.8, crate::world::HEIGHT - 0.4);
    }
    if pos.is_finite() && (pos - o.target).length() > 0.01 {
        // "arriba" en pantalla: el cielo al mirar al frente; al mirar en
        // picado, el lado opuesto a la cámara (la hoja no se voltea)
        let away = -Vec3::new(o.yaw.sin(), 0.0, o.yaw.cos());
        let up = Vec3::Y * o.pitch.cos() + away * o.pitch.sin();
        *tf = Transform::from_translation(pos).looking_at(o.target, up);
    }
}
