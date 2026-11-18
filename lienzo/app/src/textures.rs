//! Texturas procedurales: lo que no es sprite se genera aquí (puntos de luz,
//! rayos, suelo, fondo).

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

fn make(w: u32, h: u32, f: impl Fn(f32, f32) -> [f32; 4]) -> Image {
    let mut data = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let px = f((x as f32 + 0.5) / w as f32, (y as f32 + 0.5) / h as f32);
            for c in px {
                data.push((c.clamp(0.0, 1.0) * 255.0) as u8);
            }
        }
    }
    Image::new(
        Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}

/// Punto suave, para partículas y burbujas.
pub fn soft_dot() -> Image {
    make(64, 64, |u, v| {
        let d = ((u - 0.5).powi(2) + (v - 0.5).powi(2)).sqrt() * 2.0;
        let a = (1.0 - d).clamp(0.0, 1.0).powf(2.2);
        [0.75, 0.9, 0.95, a]
    })
}

/// Rayo de luz: brillante arriba, se funde hacia abajo, bordes suaves.
pub fn ray() -> Image {
    make(64, 512, |u, v| {
        let edge = (1.0 - ((u - 0.5).abs() * 2.0)).clamp(0.0, 1.0).powf(1.8);
        let fall = (1.0 - v).powf(2.4);
        let a = edge * fall;
        [0.55 * a, 0.8 * a, 0.9 * a, a]
    })
}

/// Fondo: degradado vertical de la superficie al abismo.
pub fn backdrop() -> Image {
    make(16, 256, |_, v| {
        let t = v.powf(1.3);
        let top = [0.11, 0.36, 0.50];
        let bottom = [0.012, 0.04, 0.11];
        [
            top[0] * (1.0 - t) + bottom[0] * t,
            top[1] * (1.0 - t) + bottom[1] * t,
            top[2] * (1.0 - t) + bottom[2] * t,
            1.0,
        ]
    })
}

/// Suelo de arena con cáusticas horneadas.
pub fn sand() -> Image {
    let hash = |x: i32, y: i32| -> f32 {
        let mut h = (x.wrapping_mul(374761393) ^ y.wrapping_mul(668265263)) as u32;
        h = (h ^ (h >> 13)).wrapping_mul(1274126177);
        (h ^ (h >> 16)) as f32 / u32::MAX as f32
    };
    let noise = move |x: f32, y: f32| -> f32 {
        let (xi, yi) = (x.floor() as i32, y.floor() as i32);
        let (fx, fy) = (x - xi as f32, y - yi as f32);
        let (sx, sy) = (fx * fx * (3.0 - 2.0 * fx), fy * fy * (3.0 - 2.0 * fy));
        let a = hash(xi, yi) * (1.0 - sx) + hash(xi + 1, yi) * sx;
        let b = hash(xi, yi + 1) * (1.0 - sx) + hash(xi + 1, yi + 1) * sx;
        a * (1.0 - sy) + b * sy
    };
    make(512, 512, move |u, v| {
        let n = 0.6 * noise(u * 12.0, v * 12.0)
            + 0.3 * noise(u * 31.0, v * 31.0)
            + 0.1 * noise(u * 90.0, v * 90.0);
        let c =
            ((u * 37.0 + v * 23.0).sin() * (u * 53.0 - v * 31.0).sin() * (u * 19.0 + 2.0).sin())
                .powi(2);
        let k = 0.75 + 0.35 * n + 0.45 * c;
        [0.22 * k, 0.35 * k, 0.45 * k, 1.0]
    })
}
