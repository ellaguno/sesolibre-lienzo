//! El aspecto de cada agua: colores, niebla, luz. La música del agua vive en
//! `lienzo_core::theory::tonality`; aquí solo lo que se ve. Cambiar de agua en
//! caliente retiñe todo lo que ya está en escena.

use crate::material::CardMaterial;
use crate::textures;
use crate::world::Common;
use crate::Obra;
use bevy::prelude::*;
use lienzo_core::Water;

#[derive(Clone, Copy, Debug)]
pub struct Look {
    /// Agua cerca de la superficie y en el abismo (fondo esférico).
    pub top: [f32; 3],
    pub bottom: [f32; 3],
    pub fog: Color,
    pub fog_density: f32,
    /// Arena.
    pub sand: [f32; 3],
    /// Intensidad de los rayos de luz y de las partículas.
    pub rays: f32,
    pub particles: f32,
    pub clear: Color,
}

pub fn look(w: Water) -> Look {
    match w {
        Water::Abismo => Look {
            top: [0.11, 0.36, 0.50],
            bottom: [0.012, 0.04, 0.11],
            fog: Color::srgb(0.05, 0.16, 0.26),
            fog_density: 0.045,
            sand: [0.22, 0.35, 0.45],
            rays: 0.55,
            particles: 0.35,
            clear: Color::srgb(0.012, 0.04, 0.11),
        },
        Water::Caribe => Look {
            top: [0.55, 0.90, 0.92],
            bottom: [0.06, 0.42, 0.55],
            fog: Color::srgb(0.30, 0.72, 0.78),
            fog_density: 0.028,
            sand: [0.80, 0.78, 0.66],
            rays: 0.95,
            particles: 0.25,
            clear: Color::srgb(0.06, 0.42, 0.55),
        },
        Water::Hawai => Look {
            top: [0.20, 0.52, 0.85],
            bottom: [0.02, 0.10, 0.30],
            fog: Color::srgb(0.10, 0.32, 0.58),
            fog_density: 0.036,
            sand: [0.16, 0.16, 0.19],
            rays: 0.80,
            particles: 0.30,
            clear: Color::srgb(0.02, 0.10, 0.30),
        },
        Water::Cristalina => Look {
            top: [0.72, 0.94, 0.90],
            bottom: [0.25, 0.60, 0.62],
            fog: Color::srgb(0.55, 0.85, 0.84),
            fog_density: 0.018,
            sand: [0.72, 0.74, 0.62],
            rays: 0.70,
            particles: 0.18,
            clear: Color::srgb(0.25, 0.60, 0.62),
        },
    }
}

/// El agua que está aplicada en escena ahora mismo.
#[derive(Resource, Default)]
pub struct Applied(pub Option<Water>);

/// Si la obra cambió de agua, retiñe: fondo, arena, niebla de todos los
/// materiales, rayos, partículas y color de limpieza.
pub fn apply(
    obra: Res<Obra>,
    mut applied: ResMut<Applied>,
    common: Res<Common>,
    mut materials: ResMut<Assets<CardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut clear: ResMut<ClearColor>,
) {
    let w = obra.scene.water;
    if applied.0 == Some(w) {
        return;
    }
    applied.0 = Some(w);
    let lk = look(w);
    clear.0 = lk.clear;
    let fog: LinearRgba = lk.fog.into();
    for (_, m) in materials.iter_mut() {
        m.fog_color = fog;
        m.params.x = lk.fog_density;
    }
    if let Some(mut m) = materials.get_mut(&common.backdrop) {
        m.texture = images.add(textures::backdrop(lk.top, lk.bottom));
    }
    if let Some(mut m) = materials.get_mut(&common.sand) {
        m.texture = images.add(textures::sand(lk.sand));
    }
    if let Some(mut m) = materials.get_mut(&common.ray) {
        m.color = Color::srgba(1.0, 1.0, 1.0, lk.rays).into();
    }
    if let Some(mut m) = materials.get_mut(&common.dot) {
        m.color = Color::srgba(1.0, 1.0, 1.0, lk.particles).into();
    }
}
