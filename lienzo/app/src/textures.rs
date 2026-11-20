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

/// Cáusticas: textura celular (Worley) *tileable*. Las líneas brillantes son
/// las fronteras entre celdas (segunda distancia menos primera), que es justo
/// la forma de la luz que el oleaje concentra en el fondo. En el shader se
/// muestrea dos veces con desplazamientos opuestos y se multiplica.
pub fn caustic_tile() -> Image {
    const N: usize = 256;
    let mut seed = 2024u32;
    let mut rnd = move || {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        (seed >> 8) as f32 / (1u32 << 24) as f32
    };
    // dos redes de celdas de distinto tamaño, con puntos muy desordenados:
    // la mezcla rompe cualquier cuadrícula
    let layer = |cells: usize, rnd: &mut dyn FnMut() -> f32| -> Vec<(f32, f32)> {
        (0..cells * cells)
            .map(|i| {
                let cx = (i % cells) as f32 / cells as f32;
                let cy = (i / cells) as f32 / cells as f32;
                (cx + rnd() * 1.3 / cells as f32, cy + rnd() * 1.3 / cells as f32)
            })
            .collect()
    };
    let coarse = layer(5, &mut rnd);
    let fine = layer(9, &mut rnd);
    let edge_of = |u: f32, v: f32, pts: &[(f32, f32)], cells: f32| -> f32 {
        let mut f1 = 9.0f32;
        let mut f2 = 9.0f32;
        for &(px, py) in pts {
            let mut dx = (u - px).abs();
            let mut dy = (v - py).abs();
            if dx > 0.5 {
                dx = 1.0 - dx;
            }
            if dy > 0.5 {
                dy = 1.0 - dy;
            }
            let d = (dx * dx + dy * dy).sqrt();
            if d < f1 {
                f2 = f1;
                f1 = d;
            } else if d < f2 {
                f2 = d;
            }
        }
        (f2 - f1) * cells
    };
    let mut img = make(N as u32, N as u32, move |u, v| {
        let e1 = edge_of(u, v, &coarse, 5.0);
        let e2 = edge_of(u, v, &fine, 9.0);
        let line = |e: f32| (1.0 - e * 3.4).clamp(0.0, 1.0).powf(2.4);
        let glow = |e: f32| (1.0 - e * 1.2).clamp(0.0, 1.0).powf(3.0) * 0.22;
        let c = (0.4 * (line(e1) + glow(e1)) + 0.75 * (line(e2) + glow(e2))).min(1.0);
        [c, c, c, 1.0]
    });
    // se muestrea por coordenadas de mundo: debe repetirse sin fin
    use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
    img.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        ..default()
    });
    img
}

/// Mancha de acuarela: borde suave e irregular, grano de papel, blanca (el
/// color lo pone el material).
pub fn blob() -> Image {
    make(96, 96, |u, v| {
        let x = u - 0.5;
        let y = v - 0.5;
        let ang = y.atan2(x);
        let wobble = 1.0 + 0.10 * (ang * 3.0).sin() + 0.06 * (ang * 7.0 + 1.3).cos();
        let d = (x * x + y * y).sqrt() * 2.0 / wobble;
        let edge = (1.0 - d).clamp(0.0, 1.0);
        // más pigmento en el borde, como la acuarela al secarse
        let a = (edge.powf(0.6) * 0.75 + (edge * (1.0 - edge) * 4.0).powf(2.0) * 0.35).min(1.0);
        let grain = 0.92 + 0.08 * (((u * 53.0).sin() * (v * 47.0).cos() * 13.0).fract());
        [grain, grain, grain, a * 0.9]
    })
}

/// Arcoíris: medio anillo con las bandas del espectro, suave y tenue. El
/// color lo da la textura; el material solo pone la intensidad.
pub fn rainbow() -> Image {
    make(256, 128, |u, v| {
        let x = (u - 0.5) * 2.0;
        let y = 1.0 - v; // 0 abajo, 1 arriba
        let r = (x * x + y * y).sqrt();
        // banda entre r=0.72 y r=1.0; fuera, nada
        let t = (r - 0.72) / 0.28;
        if !(0.0..=1.0).contains(&t) || y < 0.0 {
            return [0.0, 0.0, 0.0, 0.0];
        }
        // del rojo (fuera) al violeta (dentro)
        let h = (1.0 - t) * 300.0;
        let (rr, gg, bb) = hsv(h, 0.55, 1.0);
        let soft = (t * (1.0 - t) * 4.0).powf(0.6);
        let ends = (y * 3.0).clamp(0.0, 1.0); // se desvanece en los pies
        let a = soft * ends * 0.9;
        [rr * a, gg * a, bb * a, a]
    })
}

fn hsv(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
    let c = v * s;
    let hp = (h / 60.0) % 6.0;
    let x = c * (1.0 - ((hp % 2.0) - 1.0).abs());
    let (r, g, b) = match hp as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    (r + m, g + m, b + m)
}

/// Rayo de luz: brillante arriba, se funde hacia abajo, bordes suaves.
pub fn ray() -> Image {
    make(64, 512, |u, v| {
        // perfil gaussiano: sin bordes, solo un velo de luz
        let d = (u - 0.5) * 2.0;
        let edge = (-d * d * 5.0).exp() - (-5.0f32).exp();
        // nace a plena luz en la superficie y se disuelve mucho antes del fondo
        let fall = (1.0 - (v / 0.34).clamp(0.0, 1.0)).powf(1.5);
        let a = (edge * fall).max(0.0);
        [0.55 * a, 0.8 * a, 0.9 * a, a]
    })
}

/// Fondo: degradado vertical de la superficie al abismo.
pub fn backdrop(top: [f32; 3], bottom: [f32; 3]) -> Image {
    // v = 0 es el polo superior de la esfera (superficie), v = 0.5 el horizonte.
    // El horizonte queda a medio camino para que el suelo, con niebla del mismo
    // color, se funda sin costura.
    // En la esfera de Bevy v crece hacia ARRIBA, así que se invierte.
    make(16, 256, move |_, v| {
        let v = 1.0 - v;
        // Del polo (superficie) al horizonte: degradado. Por debajo del
        // horizonte, el color del horizonte, que es el de la niebla: el suelo
        // se funde con el fondo sin banda.
        let u = ((v - 0.10) / 0.40).clamp(0.0, 1.0);
        let t = (u * u * (3.0 - 2.0 * u)) * 0.52;
        [
            top[0] * (1.0 - t) + bottom[0] * t,
            top[1] * (1.0 - t) + bottom[1] * t,
            top[2] * (1.0 - t) + bottom[2] * t,
            1.0,
        ]
    })
}

/// Suelo de arena con cáusticas horneadas.
pub fn sand(color: [f32; 3]) -> Image {
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
        [color[0] * k, color[1] * k, color[2] * k, 1.0]
    })
}
