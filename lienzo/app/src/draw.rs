//! El plano de dibujo y los pinceles. Botón izquierdo dibuja; el trazo se
//! convierte en cartas y en música al soltar. Teclas: 1–8 pincel, [ ] profundidad
//! del plano, Z deshacer, C vaciar, S guardar.

use crate::cards::{self, Owned, SpriteMaterials};
use crate::input::Pointer;
use crate::material::CardMaterial;
use crate::orbit::Orbit;
use crate::world::{to_core, Common, HALF_W, HEIGHT};
use crate::Obra;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use lienzo_core::brush::{spec, Attach};
use lienzo_core::{Brush, Stroke};

/// Las dos vistas (DISEÑO §11.2): dibujar desde arriba, bucear dentro.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum View {
    Dibujo,
    Buceo,
}

/// Capa de altura donde cae lo dibujado en la vista cenital (pinceles de agua).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layer {
    Fondo,
    Agua,
    Superficie,
}

impl Layer {
    pub const ALL: [Layer; 3] = [Layer::Fondo, Layer::Agua, Layer::Superficie];

    /// Altura en coordenadas del core (0 suelo .. 1 superficie).
    pub fn height(&self) -> f32 {
        match self {
            Layer::Fondo => 0.12,
            Layer::Agua => 0.5,
            Layer::Superficie => 0.88,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Layer::Fondo => "fondo",
            Layer::Agua => "agua",
            Layer::Superficie => "superficie",
        }
    }
}

/// Los colores de la acuarela (sRGB). Pocos y bonitos; se eligen por toque.
pub const SWATCHES: [[f32; 3]; 8] = [
    [0.96, 0.55, 0.50], // coral
    [0.98, 0.76, 0.40], // ámbar
    [0.98, 0.92, 0.55], // arena luminosa
    [0.55, 0.85, 0.65], // verde agua
    [0.35, 0.80, 0.85], // turquesa
    [0.40, 0.55, 0.95], // azul
    [0.70, 0.55, 0.90], // violeta
    [0.96, 0.96, 0.98], // blanco
];

#[derive(Resource)]
pub struct DrawState {
    pub brush: Brush,
    pub swatch: usize,
    pub view: View,
    pub layer: Layer,
    /// Modo contemplación: sin paleta, la cámara deriva, tocar hace actuar.
    pub contemplate: bool,
    /// Distancia del plano de dibujo a la cámara, en unidades de mundo.
    pub depth: f32,
    points: Vec<Vec3>,
    started: Option<f64>,
    preview: Vec<Entity>,
    pub title_dirty: bool,
}

impl Default for DrawState {
    fn default() -> Self {
        DrawState {
            brush: Brush::Kelp,
            swatch: 4,
            // LIENZO_VIEW=buceo arranca buceando (pruebas y capturas)
            view: if std::env::var("LIENZO_VIEW")
                .map(|v| v == "buceo")
                .unwrap_or(false)
            {
                View::Buceo
            } else {
                View::Dibujo
            },
            layer: Layer::Agua,
            contemplate: false,
            depth: 15.0,
            points: vec![],
            started: None,
            preview: vec![],
            title_dirty: true,
        }
    }
}

pub fn brush_name(b: Brush) -> &'static str {
    match b {
        Brush::Kelp => "Kelp",
        Brush::Coral => "Coral",
        Brush::Anemona => "Anémona",
        Brush::Cardumen => "Cardumen",
        Brush::Medusa => "Medusa",
        Brush::Burbujas => "Burbujas",
        Brush::Ballena => "Ballena",
        Brush::Barco => "Barco",
        Brush::Color => "Color",
        Brush::Farol => "Farol",
        Brush::Haz => "Haz de luz",
        Brush::Arcoiris => "Arcoíris",
    }
}

pub fn keys(
    keys: Res<ButtonInput<KeyCode>>,
    gallery: Res<crate::gallery::Gallery>,
    mut st: ResMut<DrawState>,
    mut obra: ResMut<Obra>,
    mut commands: Commands,
    owned: Query<(Entity, &Owned)>,
) {
    if gallery.editing_title {
        return;
    }
    let digits = [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
        KeyCode::Digit6,
        KeyCode::Digit7,
        KeyCode::Digit8,
    ];
    for (i, k) in digits.iter().enumerate() {
        if keys.just_pressed(*k) {
            st.brush = Brush::ALL[i];
            st.title_dirty = true;
        }
    }
    if keys.just_pressed(KeyCode::KeyA) || keys.just_pressed(KeyCode::Escape) {
        st.contemplate = !st.contemplate;
        if st.contemplate {
            st.view = View::Buceo;
        }
        st.title_dirty = true;
    }
    if keys.just_pressed(KeyCode::KeyV) {
        st.view = match st.view {
            View::Dibujo => View::Buceo,
            View::Buceo => View::Dibujo,
        };
        st.title_dirty = true;
    }
    let cycle = |l: Layer, up: bool| -> Layer {
        let i = Layer::ALL.iter().position(|x| *x == l).unwrap() as i32;
        let n = Layer::ALL.len() as i32;
        Layer::ALL[((i + if up { 1 } else { -1 }).rem_euclid(n)) as usize]
    };
    if keys.just_pressed(KeyCode::BracketLeft) {
        match st.view {
            View::Dibujo => st.layer = cycle(st.layer, false),
            View::Buceo => st.depth = (st.depth - 1.0).max(4.0),
        }
        st.title_dirty = true;
    }
    if keys.just_pressed(KeyCode::BracketRight) {
        match st.view {
            View::Dibujo => st.layer = cycle(st.layer, true),
            View::Buceo => st.depth = (st.depth + 1.0).min(26.0),
        }
        st.title_dirty = true;
    }
    if keys.just_pressed(KeyCode::KeyZ) && !obra.scene.strokes.is_empty() {
        let last = obra.scene.strokes.len() - 1;
        obra.scene.strokes.pop();
        for (e, o) in owned.iter() {
            if o.0 == last {
                commands.entity(e).despawn();
            }
        }
        obra.dirty = true;
        st.title_dirty = true;
    }
    if keys.just_pressed(KeyCode::KeyC) {
        obra.scene.strokes.clear();
        for (e, _) in owned.iter() {
            commands.entity(e).despawn();
        }
        obra.dirty = true;
        st.title_dirty = true;
    }
}

/// Dónde cae el cursor según el pincel: en el suelo, o en el plano de dibujo
/// paralelo a la cámara.
/// Rayo contra un plano horizontal a altura `y`.
fn hit_horizontal(o: Vec3, d: Vec3, y: f32) -> Option<Vec3> {
    if d.y.abs() < 1e-5 {
        return None;
    }
    let t = (y - o.y) / d.y;
    if t < 0.0 {
        return None;
    }
    Some(o + d * t)
}

fn cursor_point(
    pos: Option<Vec2>,
    cam: &Camera,
    cam_tf: &GlobalTransform,
    brush: Brush,
    depth: f32,
    view: View,
    layer: Layer,
) -> Option<Vec3> {
    let pos = pos?;
    let ray = cam.viewport_to_world(cam_tf, pos).ok()?;
    let o = ray.origin;
    let d: Vec3 = ray.direction.into();
    let hit = match spec(brush).attach {
        Attach::Floor => hit_horizontal(o, d, 0.0)?,
        // desde arriba, lo de agua cae en la capa elegida
        Attach::Water if view == View::Dibujo => hit_horizontal(o, d, layer.height() * HEIGHT)?,
        Attach::Water => {
            let n: Vec3 = cam_tf.forward().into();
            let p0 = o + n * depth;
            let denom = d.dot(n);
            if denom.abs() < 1e-5 {
                return None;
            }
            o + d * ((p0 - o).dot(n) / denom)
        }
    };
    Some(Vec3::new(
        hit.x.clamp(-HALF_W, HALF_W),
        hit.y.clamp(0.0, HEIGHT),
        hit.z.clamp(-HALF_W, HALF_W),
    ))
}

pub fn draw(
    ptr: Res<Pointer>,
    time: Res<Time>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cams: Query<(&Camera, &GlobalTransform), With<Orbit>>,
    mut st: ResMut<DrawState>,
    mut obra: ResMut<Obra>,
    gallery: Res<crate::gallery::Gallery>,
    mut commands: Commands,
    common: Res<Common>,
    mut mats: ResMut<SpriteMaterials>,
    assets: Res<AssetServer>,
    mut materials: ResMut<Assets<CardMaterial>>,
) {
    let (Ok(window), Ok((cam, cam_tf))) = (windows.single(), cams.single()) else {
        return;
    };
    let brush = st.brush;
    let depth = st.depth;
    if st.contemplate || gallery.open {
        return;
    }
    // la paleta ocupa la franja inferior: ahí no se dibuja
    if let Some(c) = ptr.pos {
        if c.y > window.height() - crate::ui::palette_h() && st.started.is_none() {
            return;
        }
    }
    if ptr.cancel && st.started.is_some() {
        // entró un segundo dedo: era una órbita, no un trazo
        st.started = None;
        st.points.clear();
        for e in st.preview.drain(..) {
            commands.entity(e).despawn();
        }
        return;
    }

    if ptr.just_down {
        st.points.clear();
        st.started = Some(time.elapsed_secs_f64());
    }
    if ptr.down && st.started.is_some() {
        if let Some(p) = cursor_point(ptr.pos, cam, cam_tf, brush, depth, st.view, st.layer) {
            let far_enough = st
                .points
                .last()
                .map(|l| l.distance(p) > 0.12)
                .unwrap_or(true);
            if far_enough {
                st.points.push(p);
                let e = commands
                    .spawn((
                        Mesh3d(common.quad.clone()),
                        MeshMaterial3d(common.dot.clone()),
                        Transform::from_translation(p).with_scale(Vec3::splat(0.18)),
                        crate::world::Billboard,
                    ))
                    .id();
                st.preview.push(e);
            }
        }
    }
    if ptr.just_up {
        if let Some(t0) = st.started.take() {
            for e in st.preview.drain(..) {
                commands.entity(e).despawn();
            }
            if st.points.is_empty() {
                return;
            }
            let ms = ((time.elapsed_secs_f64() - t0) * 1000.0).max(60.0) as u32;
            let pts: Vec<lienzo_core::Point> = st.points.iter().map(|p| to_core(*p)).collect();
            let is_tap = pts.len() == 1 || ms < 180;
            let mut stroke = if is_tap {
                Stroke::tap(brush, pts[0])
            } else {
                Stroke::line(brush, pts, ms)
            };
            if brush.uses_color() {
                stroke = stroke.with_color(SWATCHES[st.swatch]);
            }
            let index = obra.scene.strokes.len();
            cards::spawn_stroke(
                &mut commands,
                &common,
                &mut mats,
                &assets,
                &mut materials,
                &stroke,
                index,
            );
            obra.scene.strokes.push(stroke);
            obra.new_stroke = Some(index);
            obra.dirty = true;
            st.title_dirty = true;
            st.points.clear();
        }
    }
}

/// En contemplación, tocar un elemento lo hace actuar: suena su motivo y la
/// carta da un pequeño pulso. Se elige la carta más cercana al cursor en pantalla.
pub fn touch(
    ptr: Res<Pointer>,
    cams: Query<(&Camera, &GlobalTransform), With<Orbit>>,
    st: Res<DrawState>,
    link: Res<crate::audio::AudioLink>,
    mut cards: Query<(&GlobalTransform, &Owned, &mut cards::Card)>,
) {
    if !st.contemplate || !ptr.just_down {
        return;
    }
    let Ok((cam, cam_tf)) = cams.single() else {
        return;
    };
    let Some(cursor) = ptr.pos else {
        return;
    };
    let mut best: Option<(f32, usize)> = None;
    for (tf, owned, _) in cards.iter() {
        if let Ok(p) = cam.world_to_viewport(cam_tf, tf.translation()) {
            let d = p.distance(cursor);
            if d < 60.0 && best.map(|b| d < b.0).unwrap_or(true) {
                best = Some((d, owned.0));
            }
        }
    }
    if let Some((_, index)) = best {
        link.play(index);
        for (_, owned, mut card) in cards.iter_mut() {
            if owned.0 == index {
                card.pulse = 1.0;
            }
        }
    }
}

/// El título de la ventana resume el estado.
pub fn title(
    mut st: ResMut<DrawState>,
    obra: Res<Obra>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    if !st.title_dirty {
        return;
    }
    st.title_dirty = false;
    if let Ok(mut w) = windows.single_mut() {
        w.title = if st.contemplate {
            format!(
                "Lienzo — contemplación · {} trazos · toca un elemento · A vuelve a dibujar",
                obra.scene.strokes.len()
            )
        } else {
            format!(
                "Lienzo — {} · plano a {:.0} · {} trazos",
                brush_name(st.brush),
                st.depth,
                obra.scene.strokes.len()
            )
        };
    }
}
