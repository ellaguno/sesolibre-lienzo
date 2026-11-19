//! El fondo esculpible: una malla de alturas derivada de los trazos de relieve
//! (`lienzo_core::terrain`). Se regenera cuando cambia la obra.

use crate::material::CardMaterial;
use crate::world::{to_core, HALF_W, HEIGHT};
use crate::Obra;
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use lienzo_core::terrain::height_at;
use lienzo_core::Scene;

/// Extensión de la malla del suelo en unidades de mundo. La rejilla es fina en
/// el centro (donde se dibuja) y gruesa hacia fuera: una sola malla cubre
/// hasta la pared del fondo, sin plano aparte que tape las fosas.
pub const EXTENT: f32 = 95.0;
pub const CELLS: usize = 160;
const WARP: f32 = 2.6;

fn warp(u: f32) -> f32 {
    u.signum() * u.abs().powf(WARP) * EXTENT
}

#[derive(Component)]
pub struct Terrain;

/// Huella del relieve con la que se construyó la malla, para rehacerla solo
/// cuando cambian los trazos de relieve.
#[derive(Resource, Default)]
pub struct Built(pub u64);

pub fn relief_key(scene: &Scene) -> u64 {
    let mut h: u64 = 1469598103934665603;
    for s in &scene.strokes {
        if !matches!(
            s.brush,
            lienzo_core::Brush::Elevar | lienzo_core::Brush::Hundir
        ) {
            continue;
        }
        for p in &s.points {
            for v in [p.x, p.z] {
                h ^= v.to_bits() as u64;
                h = h.wrapping_mul(1099511628211);
            }
        }
        h ^= s.gesture_ms as u64;
        h = h.wrapping_mul(1099511628211);
    }
    h
}

/// Altura del suelo en unidades de mundo en (x, z).
pub fn floor_y(scene: &Scene, x: f32, z: f32) -> f32 {
    let c = to_core(Vec3::new(x, 0.0, z));
    height_at(scene, c.x, c.z) * HEIGHT
}

pub fn build_mesh(scene: &Scene) -> Mesh {
    let n = CELLS + 1;
    let mut pos = Vec::with_capacity(n * n);
    let mut uv = Vec::with_capacity(n * n);
    let mut heights = vec![0.0f32; n * n];
    let coord = |k: usize| warp(-1.0 + 2.0 * k as f32 / CELLS as f32);
    for j in 0..n {
        for i in 0..n {
            let x = coord(i);
            let z = coord(j);
            // fuera del volumen dibujable el relieve se apaga suavemente
            let edge = ((HALF_W * 1.6 - x.abs().max(z.abs())) / (HALF_W * 0.6)).clamp(0.0, 1.0);
            let y = floor_y(scene, x, z) * edge;
            heights[j * n + i] = y;
            pos.push([x, y, z]);
            uv.push([x / 12.0, z / 12.0]);
        }
    }
    let mut normals = Vec::with_capacity(n * n);
    for j in 0..n {
        for i in 0..n {
            let h = |ii: isize, jj: isize| -> f32 {
                let ii = ii.clamp(0, n as isize - 1) as usize;
                let jj = jj.clamp(0, n as isize - 1) as usize;
                heights[jj * n + ii]
            };
            let sx = (coord((i + 1).min(CELLS)) - coord(i.saturating_sub(1))).max(1e-4);
            let sz = (coord((j + 1).min(CELLS)) - coord(j.saturating_sub(1))).max(1e-4);
            let dx = (h(i as isize + 1, j as isize) - h(i as isize - 1, j as isize)) / sx;
            let dz = (h(i as isize, j as isize + 1) - h(i as isize, j as isize - 1)) / sz;
            let nrm = Vec3::new(-dx, 1.0, -dz).normalize();
            normals.push([nrm.x, nrm.y, nrm.z]);
        }
    }
    let mut idx = Vec::with_capacity(CELLS * CELLS * 6);
    for j in 0..CELLS {
        for i in 0..CELLS {
            let a = (j * n + i) as u32;
            let b = a + 1;
            let c = a + n as u32;
            let d = c + 1;
            idx.extend_from_slice(&[a, c, b, b, c, d]);
        }
    }
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, pos);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
    mesh.insert_indices(Indices::U32(idx));
    mesh
}

/// Rehace la malla cuando cambian los trazos de relieve.
pub fn update(
    obra: Res<Obra>,
    mut built: ResMut<Built>,
    mut meshes: ResMut<Assets<Mesh>>,
    q: Query<&Mesh3d, With<Terrain>>,
) {
    let key = relief_key(&obra.scene);
    if built.0 == key {
        return;
    }
    built.0 = key;
    for m in q.iter() {
        if let Some(mut mesh) = meshes.get_mut(&m.0) {
            *mesh = build_mesh(&obra.scene);
        }
    }
}

#[allow(dead_code)]
pub fn material_marker(_: &CardMaterial) {}
