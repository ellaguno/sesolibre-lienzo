//! Los trazos como cartas de papel en el espacio. Cada pincel decide qué
//! sprites pone a lo largo del trazo, de qué tamaño y cómo se mueven.

use crate::material::CardMaterial;
use crate::world::{to_world, Billboard, Common};
use crate::Obra;
use bevy::prelude::*;
use lienzo_core::{Brush, Stroke};
use std::collections::HashMap;

#[derive(Resource, Default)]
pub struct SpriteMaterials {
    cache: HashMap<&'static str, Handle<CardMaterial>>,
    colors: HashMap<[u8; 3], Handle<CardMaterial>>,
    glows: HashMap<[u8; 3], Handle<CardMaterial>>,
}

impl SpriteMaterials {
    fn get(
        &mut self,
        name: &'static str,
        assets: &AssetServer,
        materials: &mut Assets<CardMaterial>,
    ) -> Handle<CardMaterial> {
        self.cache
            .entry(name)
            .or_insert_with(|| {
                materials.add(CardMaterial::sprite(
                    assets.load(format!("sprites/cut/{name}.png")),
                ))
            })
            .clone()
    }
}

impl SpriteMaterials {
    /// Material de acuarela de un color (uno por color, compartido).
    fn color(
        &mut self,
        rgb: [f32; 3],
        blob: &Handle<Image>,
        materials: &mut Assets<CardMaterial>,
    ) -> Handle<CardMaterial> {
        let key = [
            (rgb[0] * 255.0) as u8,
            (rgb[1] * 255.0) as u8,
            (rgb[2] * 255.0) as u8,
        ];
        self.colors
            .entry(key)
            .or_insert_with(|| {
                materials.add(CardMaterial::watercolor(
                    blob.clone(),
                    Color::srgb(rgb[0], rgb[1], rgb[2]),
                ))
            })
            .clone()
    }
}

impl SpriteMaterials {
    fn glow(
        &mut self,
        rgb: [f32; 3],
        tex: &Handle<Image>,
        materials: &mut Assets<CardMaterial>,
    ) -> Handle<CardMaterial> {
        let key = [
            (rgb[0] * 255.0) as u8,
            (rgb[1] * 255.0) as u8,
            (rgb[2] * 255.0) as u8,
        ];
        self.glows
            .entry(key)
            .or_insert_with(|| materials.add(CardMaterial::glow(tex.clone(), rgb, 0.85)))
            .clone()
    }
}

impl SpriteMaterials {
    /// Material de haz de luz dibujado (compartido).
    fn beam(
        &mut self,
        tex: &Handle<Image>,
        materials: &mut Assets<CardMaterial>,
    ) -> Handle<CardMaterial> {
        self.glows
            .entry([1, 2, 3])
            .or_insert_with(|| {
                let mut m = CardMaterial::glow(tex.clone(), [1.0, 0.96, 0.85], 0.3);
                m.extra.z = -1.0;
                materials.add(m)
            })
            .clone()
    }
}

/// Proporción ancho/alto de cada sprite, medida al recortar.
pub fn aspect(name: &str) -> f32 {
    match name {
        "medusa" => 512.0 / 608.0,
        "pez_a_1" => 923.0 / 346.0,
        "pez_a_2" => 802.0 / 252.0,
        "pez_a_3" => 850.0 / 324.0,
        "pez_b" => 609.0 / 451.0,
        "pez_c" => 477.0 / 468.0,
        "coral_a_1" => 763.0 / 631.0,
        "coral_a_2" => 538.0 / 616.0,
        "coral_b" => 623.0 / 484.0,
        "coral_b_2" => 623.0 / 489.0,
        "anemona" => 519.0 / 544.0,
        "kelp_hoja" => 243.0 / 721.0,
        "ballena" => 1026.0 / 371.0,
        "barco" => 909.0 / 591.0,
        "roca_a_1" | "roca_a_2" => 1.3,
        "roca_b" => 0.6,
        "manta" => 664.0 / 454.0,
        _ => 1.0,
    }
}

#[derive(Component)]
pub struct Card {
    pub stroke: usize,
    pub kind: Motion,
    pub phase: f32,
    pub base: Vec3,
    pub flip: bool,
    /// 1.0 justo al tocar; decae a 0.
    pub pulse: f32,
    pub size: Vec2,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Motion {
    Sway,
    Swim,
    Bob,
    Rise,
    /// Latido lento de luz (farol).
    Glow,
    Still,
}

#[derive(Component)]
pub struct Owned(pub usize);

#[derive(Component)]
pub struct RouteDot;

/// Los puntos de la ruta solo se ven al dibujar.
pub fn route_visibility(
    st: Res<crate::draw::DrawState>,
    mut q: Query<&mut Visibility, With<RouteDot>>,
) {
    let show = st.view == crate::draw::View::Dibujo && !st.contemplate;
    for mut v in q.iter_mut() {
        *v = if show {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

fn hash01(a: usize, b: usize) -> f32 {
    let mut h =
        (a as u64).wrapping_mul(0x9E3779B97F4A7C15) ^ (b as u64).wrapping_mul(0xBF58476D1CE4E5B9);
    h ^= h >> 31;
    h = h.wrapping_mul(0x94D049BB133111EB);
    (h >> 40) as f32 / (1u64 << 24) as f32
}

/// Puntos equidistantes a lo largo del trazo, cada `step` unidades de mundo.
fn along(stroke: &Stroke, step: f32) -> Vec<(Vec3, Vec3)> {
    let pts: Vec<Vec3> = stroke.points.iter().map(|p| to_world(*p)).collect();
    if pts.len() < 2 {
        return vec![(pts[0], Vec3::X)];
    }
    let mut out = vec![];
    let mut carry = 0.0;
    for w in pts.windows(2) {
        let seg = w[1] - w[0];
        let len = seg.length();
        if len < 1e-5 {
            continue;
        }
        let dir = seg / len;
        let mut d = carry;
        while d <= len {
            out.push((w[0] + dir * d, dir));
            d += step;
        }
        carry = d - len;
    }
    if out.is_empty() {
        out.push((pts[0], Vec3::X));
    }
    out
}

fn spawn_card(
    commands: &mut Commands,
    common: &Common,
    mats: &mut SpriteMaterials,
    assets: &AssetServer,
    materials: &mut Assets<CardMaterial>,
    name: &'static str,
    pos: Vec3,
    height: f32,
    flip: bool,
    stroke: usize,
    kind: Motion,
    phase: f32,
) {
    let w = height * aspect(name) * if flip { -1.0 } else { 1.0 };
    // las cartas se anclan por su base: pos es el punto de apoyo
    let base = pos + Vec3::Y * height * 0.5;
    commands.spawn((
        Mesh3d(common.quad.clone()),
        MeshMaterial3d(mats.get(name, assets, materials)),
        Transform::from_translation(base).with_scale(Vec3::new(w, height, 1.0)),
        Card {
            stroke,
            kind,
            phase,
            base,
            flip,
            pulse: 0.0,
            size: Vec2::new(w, height),
        },
        Owned(stroke),
        Billboard,
    ));
}

/// Materializa un trazo nuevo. Determinista por índice de trazo.
pub fn spawn_stroke(
    commands: &mut Commands,
    common: &Common,
    mats: &mut SpriteMaterials,
    assets: &AssetServer,
    materials: &mut Assets<CardMaterial>,
    scene: &lienzo_core::Scene,
    stroke: &Stroke,
    index: usize,
) {
    let h = |k: usize| hash01(index, k);
    // los elementos de suelo se asientan sobre el relieve
    let floor = |v: Vec3| Vec3::new(v.x, crate::terrain::floor_y(scene, v.x, v.z), v.z);
    match stroke.brush {
        Brush::Kelp => {
            for (k, (p, _)) in along(stroke, 0.75).into_iter().enumerate() {
                let height = 0.9 + 0.8 * h(k);
                let jitter = Vec3::new(h(k + 100) - 0.5, 0.0, h(k + 200) - 0.5) * 0.5;
                spawn_card(
                    commands,
                    common,
                    mats,
                    assets,
                    materials,
                    "kelp_hoja",
                    floor(p) + jitter,
                    height,
                    k % 2 == 0,
                    index,
                    Motion::Sway,
                    h(k + 300) * 6.28,
                );
            }
        }
        Brush::Coral => {
            let names = ["coral_a_1", "coral_a_2", "coral_b", "coral_b_2"];
            for (k, (p, _)) in along(stroke, 0.9).into_iter().enumerate() {
                let name = names[(k + (h(k) * 4.0) as usize) % 4];
                spawn_card(
                    commands,
                    common,
                    mats,
                    assets,
                    materials,
                    name,
                    floor(p),
                    0.8 + 0.7 * h(k + 1),
                    h(k + 2) > 0.5,
                    index,
                    Motion::Still,
                    0.0,
                );
            }
        }
        Brush::Anemona => {
            let p = to_world(stroke.points[0]);
            spawn_card(
                commands,
                common,
                mats,
                assets,
                materials,
                "anemona",
                floor(p),
                0.9 + 0.3 * h(0),
                false,
                index,
                Motion::Still,
                0.0,
            );
        }
        Brush::Cardumen => {
            let names = ["pez_a_1", "pez_a_2", "pez_a_3"];
            for (k, (p, dir)) in along(stroke, 0.5).into_iter().enumerate() {
                let jitter = Vec3::new(0.0, h(k + 50) - 0.5, h(k + 60) - 0.5) * 0.35;
                spawn_card(
                    commands,
                    common,
                    mats,
                    assets,
                    materials,
                    names[k % 3],
                    p + jitter,
                    0.28 + 0.14 * h(k),
                    dir.x < 0.0,
                    index,
                    Motion::Swim,
                    h(k + 70) * 6.28,
                );
            }
        }
        Brush::Medusa => {
            let p = to_world(stroke.points[0]);
            spawn_card(
                commands,
                common,
                mats,
                assets,
                materials,
                "medusa",
                p - Vec3::Y * 0.6,
                1.2 + 0.3 * h(0),
                false,
                index,
                Motion::Bob,
                h(1) * 6.28,
            );
        }
        Brush::Burbujas => {
            for (k, (p, _)) in along(stroke, 0.3).into_iter().enumerate() {
                let s = 0.08 + 0.12 * h(k);
                commands.spawn((
                    Mesh3d(common.quad.clone()),
                    MeshMaterial3d(common.dot.clone()),
                    Transform::from_translation(p).with_scale(Vec3::splat(s)),
                    Card {
                        stroke: index,
                        kind: Motion::Rise,
                        phase: h(k + 9) * 6.28,
                        base: p,
                        flip: false,
                        pulse: 0.0,
                        size: Vec2::splat(s),
                    },
                    Owned(index),
                    Billboard,
                ));
            }
        }
        Brush::Ballena => {
            let p = to_world(stroke.points[0]);
            spawn_card(
                commands,
                common,
                mats,
                assets,
                materials,
                "ballena",
                p - Vec3::Y * 0.8,
                1.6,
                p.x > 0.0,
                index,
                Motion::Swim,
                0.0,
            );
        }
        Brush::Color => {
            // manchas de acuarela encadenadas a lo largo del gesto; más grandes
            // si el gesto fue lento
            let rgb = stroke.color.unwrap_or([0.9, 0.6, 0.5]);
            let mat = mats.color(rgb, &common.blob, materials);
            let slow = (1.0 - (stroke.speed() / 2.5).clamp(0.0, 1.0)) * 0.5;
            for (k, (p, _)) in along(stroke, 0.22).into_iter().enumerate() {
                let r = 0.55 + 0.45 * h(k) + slow;
                let jitter = Vec3::new(h(k + 31) - 0.5, h(k + 41) - 0.5, h(k + 51) - 0.5) * 0.25;
                commands.spawn((
                    Mesh3d(common.quad.clone()),
                    MeshMaterial3d(mat.clone()),
                    Transform::from_translation(p + jitter)
                        .with_rotation(Quat::from_rotation_z(h(k + 61) * 6.28))
                        .with_scale(Vec3::new(r, r, 1.0)),
                    Card {
                        stroke: index,
                        kind: Motion::Still,
                        phase: 0.0,
                        base: p + jitter,
                        flip: false,
                        pulse: 0.0,
                        size: Vec2::splat(r),
                    },
                    Owned(index),
                    Billboard,
                ));
            }
        }
        Brush::Farol => {
            // núcleo brillante + halo amplio, con latido
            let rgb = stroke.color.unwrap_or([0.6, 0.9, 1.0]);
            let p = to_world(stroke.points[0]);
            let core = mats.glow(rgb, &common.glow_tex, materials);
            let halo_rgb = [rgb[0] * 0.5, rgb[1] * 0.5, rgb[2] * 0.5];
            let halo = mats.glow(halo_rgb, &common.glow_tex, materials);
            for (mat, r, ph) in [(core, 0.55, 0.0), (halo, 2.2, 1.7)] {
                commands.spawn((
                    Mesh3d(common.quad.clone()),
                    MeshMaterial3d(mat),
                    Transform::from_translation(p).with_scale(Vec3::new(r, r, 1.0)),
                    Card {
                        stroke: index,
                        kind: Motion::Glow,
                        phase: h(0) * 6.28 + ph,
                        base: p,
                        flip: false,
                        pulse: 0.0,
                        size: Vec2::splat(r),
                    },
                    Owned(index),
                    Billboard,
                ));
            }
        }
        Brush::Haz => {
            // haces de sol a lo largo del gesto: nacen en la superficie, de
            // cara a la cámara, y se funden como los rayos de ambiente
            let mat = mats.beam(&common.ray_tex, materials);
            for (k, (p, _)) in along(stroke, 0.7).into_iter().enumerate() {
                let w = 0.8 + 1.2 * h(k);
                let height = 11.0 + 3.0 * h(k + 7);
                commands.spawn((
                    Mesh3d(common.quad.clone()),
                    MeshMaterial3d(mat.clone()),
                    Transform::from_xyz(p.x, 5.0 - height * 0.5, p.z)
                        .with_scale(Vec3::new(w, height, 1.0)),
                    crate::world::Ray {
                        tilt: 0.10 + 0.12 * h(k + 3),
                        phase: h(k + 9) * 6.28,
                        width: w,
                        x: p.x,
                        z: p.z,
                        drawn: true,
                    },
                    Owned(index),
                ));
            }
        }
        Brush::Arcoiris => {
            // un arco tenue que abarca el gesto
            let pts: Vec<Vec3> = stroke.points.iter().map(|q| to_world(*q)).collect();
            let a = pts[0];
            let b = *pts.last().unwrap();
            let span = (a.distance(b)).max(2.0);
            let center = (a + b) * 0.5;
            let mat = mats.glow([1.0, 1.0, 1.0], &common.rainbow_tex, materials);
            let size = Vec2::new(span * 1.2, span * 0.6);
            commands.spawn((
                Mesh3d(common.quad.clone()),
                MeshMaterial3d(mat),
                Transform::from_translation(center + Vec3::Y * size.y * 0.3)
                    .with_scale(Vec3::new(size.x, size.y, 1.0)),
                Card {
                    stroke: index,
                    kind: Motion::Still,
                    phase: 0.0,
                    base: center + Vec3::Y * size.y * 0.3,
                    flip: false,
                    pulse: 0.0,
                    size,
                },
                Owned(index),
                Billboard,
            ));
        }
        Brush::Elevar => {
            // rocas sueltas sobre la cresta, pocas
            let names = ["roca_a_1", "roca_a_2", "roca_b"];
            for (k, (p, _)) in along(stroke, 1.1).into_iter().enumerate() {
                if h(k + 77) < 0.45 {
                    continue;
                }
                let name = names[(h(k) * 3.0) as usize % 3];
                let jitter = Vec3::new(h(k + 100) - 0.5, 0.0, h(k + 200) - 0.5) * 0.6;
                let q = p + jitter;
                spawn_card(
                    commands,
                    common,
                    mats,
                    assets,
                    materials,
                    name,
                    floor(q) - Vec3::Y * 0.08,
                    0.5 + 0.5 * h(k + 1),
                    h(k + 2) > 0.5,
                    index,
                    Motion::Still,
                    0.0,
                );
            }
        }
        Brush::Hundir => {}
        Brush::Ruta => {
            // puntitos de luz a lo largo de la ruta; en el buceo se ocultan
            for (k, (p, _)) in along(stroke, 0.35).into_iter().enumerate() {
                commands.spawn((
                    Mesh3d(common.quad.clone()),
                    MeshMaterial3d(common.dot.clone()),
                    Transform::from_translation(p).with_scale(Vec3::splat(0.14)),
                    Card {
                        stroke: index,
                        kind: Motion::Still,
                        phase: h(k) * 6.28,
                        base: p,
                        flip: false,
                        pulse: 0.0,
                        size: Vec2::splat(0.14),
                    },
                    RouteDot,
                    Owned(index),
                    Billboard,
                ));
            }
        }
        Brush::Barco => {
            let p = to_world(stroke.points[0]);
            spawn_card(
                commands,
                common,
                mats,
                assets,
                materials,
                "barco",
                floor(p),
                2.2,
                p.x < 0.0,
                index,
                Motion::Still,
                0.0,
            );
        }
    }
}

pub fn spawn_scene(
    flags: Res<crate::DebugFlags>,
    mut commands: Commands,
    common: Res<Common>,
    mut mats: ResMut<SpriteMaterials>,
    assets: Res<AssetServer>,
    mut materials: ResMut<Assets<CardMaterial>>,
    scene: Res<crate::Obra>,
) {
    if flags.has("no_cards") {
        return;
    }
    for (i, s) in scene.scene.strokes.iter().enumerate() {
        spawn_stroke(
            &mut commands,
            &common,
            &mut mats,
            &assets,
            &mut materials,
            &scene.scene,
            s,
            i,
        );
    }
}

/// Cuando cambia el relieve, todas las cartas se vuelven a asentar.
pub fn resettle(
    obra: Res<Obra>,
    mut last: Local<u64>,
    mut commands: Commands,
    owned: Query<Entity, With<Owned>>,
    common: Res<Common>,
    mut mats: ResMut<SpriteMaterials>,
    assets: Res<AssetServer>,
    mut materials: ResMut<Assets<CardMaterial>>,
) {
    let key = crate::terrain::relief_key(&obra.scene);
    if *last == key {
        return;
    }
    let first = *last == 0;
    *last = key;
    if first {
        return;
    }
    for e in owned.iter() {
        commands.entity(e).despawn();
    }
    for (i, s) in obra.scene.strokes.iter().enumerate() {
        spawn_stroke(
            &mut commands,
            &common,
            &mut mats,
            &assets,
            &mut materials,
            &obra.scene,
            s,
            i,
        );
    }
}

/// Todo vive: el kelp se mece, los peces nadan, la medusa pulsa, las burbujas suben.
pub fn animate(time: Res<Time>, mut q: Query<(&mut Transform, &mut Card)>) {
    let t = time.elapsed_secs();
    for (mut tf, mut c) in q.iter_mut() {
        if c.pulse > 0.0 {
            c.pulse = (c.pulse - time.delta_secs() * 1.6).max(0.0);
        }
        let k = 1.0 + 0.18 * (c.pulse * std::f32::consts::PI).sin();
        tf.scale = Vec3::new(c.size.x * k, c.size.y * k, 1.0);
        match c.kind {
            Motion::Sway => {
                let a = (t * 0.55 + c.phase).sin() * 0.07;
                let (y, _, _) = tf.rotation.to_euler(EulerRot::YXZ);
                tf.rotation = Quat::from_euler(EulerRot::YXZ, y, 0.0, a);
            }
            Motion::Swim => {
                let dir = if c.flip { -1.0 } else { 1.0 };
                tf.translation.x = c.base.x + dir * (t * 0.25 + c.phase).sin() * 0.6;
                tf.translation.y = c.base.y + (t * 0.9 + c.phase).sin() * 0.06;
            }
            Motion::Bob => {
                tf.translation.y = c.base.y + (t * 0.5 + c.phase).sin() * 0.25;
                let s = 1.0 + (t * 1.1 + c.phase).sin() * 0.04;
                tf.scale.x *= s;
                tf.scale.y /= s;
            }
            Motion::Rise => {
                let y = (c.base.y + (t * 0.35 + c.phase) * 0.6) % 5.5;
                tf.translation.y = y;
                tf.translation.x = c.base.x + (t * 0.8 + c.phase).sin() * 0.08;
            }
            Motion::Glow => {
                let k = 1.0 + 0.12 * (t * 0.9 + c.phase).sin();
                tf.scale = Vec3::new(c.size.x * k, c.size.y * k, 1.0);
            }
            Motion::Still => {}
        }
    }
}
