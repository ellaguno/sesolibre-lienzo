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
    // puntos al azar por todo el mosaico (no por celda): tamaños desiguales
    let layer = |n: usize, rnd: &mut dyn FnMut() -> f32| -> Vec<(f32, f32)> {
        (0..n).map(|_| (rnd(), rnd())).collect()
    };
    let coarse = layer(22, &mut rnd);
    let fine = layer(70, &mut rnd);
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
        let e1 = edge_of(u, v, &coarse, 4.7);
        let e2 = edge_of(u, v, &fine, 8.4);
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

/// Iconos de la barra para los pinceles sin sprite. Blancos con alfa: el
/// botón los tiñe.
pub fn icon(kind: &str) -> Image {
    let kind = kind.to_string();
    make(64, 64, move |u, v| {
        let x = (u - 0.5) * 2.0;
        let y = (0.5 - v) * 2.0; // +y arriba
        let disc = |cx: f32, cy: f32, r: f32| -> f32 {
            let d = ((x - cx).powi(2) + (y - cy).powi(2)).sqrt();
            (1.0 - (d - r) * 10.0).clamp(0.0, 1.0)
        };
        let ring = |cx: f32, cy: f32, r: f32, w: f32| -> f32 {
            let d = ((x - cx).powi(2) + (y - cy).powi(2)).sqrt();
            (1.0 - ((d - r).abs() - w) * 12.0).clamp(0.0, 1.0)
        };
        let (c, a): ([f32; 3], f32) = match kind.as_str() {
            "burbujas" => {
                let a = ring(-0.3, -0.35, 0.22, 0.03)
                    .max(ring(0.3, 0.05, 0.3, 0.03))
                    .max(ring(-0.15, 0.5, 0.16, 0.03));
                ([0.85, 0.95, 1.0], a)
            }
            "farol" => {
                let d = (x * x + y * y).sqrt();
                let glow = (1.0 - d * 1.1).clamp(0.0, 1.0).powf(1.6);
                let core = disc(0.0, 0.0, 0.22);
                ([1.0, 0.9, 0.65], (glow * 0.7 + core).min(1.0))
            }
            "haz" => {
                // tres haces inclinados que se apagan hacia abajo
                let mut a: f32 = 0.0;
                for (k, off) in [-0.45f32, 0.0, 0.45].iter().enumerate() {
                    let xx = x - off - 0.25 * (y - 1.0);
                    let w = 0.10 + 0.03 * k as f32;
                    a = a.max((1.0 - (xx.abs() / w)).clamp(0.0, 1.0) * ((y + 1.0) / 2.0).powf(0.8));
                }
                ([0.95, 1.0, 1.0], a)
            }
            "arcoiris" => {
                // medio arco de bandas
                let d = (x * x + (y + 0.5).powi(2)).sqrt();
                let a = if y > -0.5 { (1.0 - ((d - 0.8).abs() - 0.22) * 8.0).clamp(0.0, 1.0) } else { 0.0 };
                let t = ((d - 0.58) / 0.44).clamp(0.0, 1.0);
                let col = [
                    0.5 + 0.5 * (t * 6.28).cos(),
                    0.5 + 0.5 * (t * 6.28 - 2.1).cos(),
                    0.5 + 0.5 * (t * 6.28 - 4.2).cos(),
                ];
                (col, a)
            }
            "elevar" | "hundir" => {
                let up = kind == "elevar";
                let sign = if up { 1.0 } else { -1.0 };
                // una loma (o una fosa): perfil suave
                let h = sign * 0.55 * (-(x * x) * 3.0).exp() - 0.1;
                let a = if up {
                    (1.0 - (y - h) * 8.0).clamp(0.0, 1.0) * (y > -0.9) as i32 as f32
                } else {
                    (1.0 - (y - h) * 8.0).clamp(0.0, 1.0) * (y > -0.9) as i32 as f32
                };
                ([0.95, 0.9, 0.8], a)
            }
            _ => {
                // ruta: lazo de puntos
                let mut a: f32 = 0.0;
                for i in 0..10 {
                    let t = i as f32 / 10.0 * std::f32::consts::TAU;
                    a = a.max(disc(0.65 * t.cos(), 0.45 * t.sin(), 0.08));
                }
                ([1.0, 1.0, 1.0], a)
            }
        };
        [c[0], c[1], c[2], a]
    })
}

/// Iconos de herramientas (línea blanca con alfa): lo que no es un pincel.
pub fn tool_icon(kind: &str) -> Image {
    let kind = kind.to_string();
    make(64, 64, move |u, v| {
        let x = (u - 0.5) * 2.0;
        let y = (0.5 - v) * 2.0;
        // segmento con grosor
        let seg = |ax: f32, ay: f32, bx: f32, by: f32, w: f32| -> f32 {
            let (dx, dy) = (bx - ax, by - ay);
            let t = (((x - ax) * dx + (y - ay) * dy) / (dx * dx + dy * dy).max(1e-6)).clamp(0.0, 1.0);
            let d = ((x - ax - dx * t).powi(2) + (y - ay - dy * t).powi(2)).sqrt();
            (1.0 - (d - w) * 14.0).clamp(0.0, 1.0)
        };
        let ring = |cx: f32, cy: f32, rx: f32, ry: f32, w: f32| -> f32 {
            let d = (((x - cx) / rx).powi(2) + ((y - cy) / ry).powi(2)).sqrt();
            (1.0 - ((d - 1.0) * rx.min(ry) - w).abs() * 14.0 + w * 14.0).clamp(0.0, 1.0)
        };
        let disc = |cx: f32, cy: f32, r: f32| -> f32 {
            let d = ((x - cx).powi(2) + (y - cy).powi(2)).sqrt();
            (1.0 - (d - r) * 14.0).clamp(0.0, 1.0)
        };
        let w = 0.08;
        let a: f32 = match kind.as_str() {
            "dibujar" => {
                // un lápiz inclinado: cuerpo, punta y goma
                seg(-0.35, -0.35, 0.45, 0.45, w * 1.7)
                    .max(seg(-0.35, -0.35, -0.6, -0.6, w * 0.9))
                    .max(seg(0.3, 0.6, 0.6, 0.3, w))
            }
            "bucear" => {
                // visor de buceo: dos óculos unidos y la cinta
                ring(-0.32, 0.05, 0.32, 0.28, w)
                    .max(ring(0.32, 0.05, 0.32, 0.28, w))
                    .max(seg(-0.05, 0.05, 0.05, 0.05, w))
                    .max(seg(-0.64, 0.15, -0.9, 0.3, w))
                    .max(seg(0.64, 0.15, 0.9, 0.3, w))
            }
            "deshacer" => {
                // flecha que vuelve a la izquierda: arco por arriba y punta
                let d = (x * x + (y + 0.15).powi(2)).sqrt();
                let ang = (y + 0.15).atan2(x);
                let arc = if ang > 0.0 && ang < 3.1 { (1.0 - ((d - 0.5).abs() - w) * 14.0).clamp(0.0, 1.0) } else { 0.0 };
                arc.max(seg(-0.5, -0.15, -0.75, 0.2, w))
                    .max(seg(-0.5, -0.15, -0.15, 0.1, w))
            }
            "galeria" => {
                // cuadrícula de cuatro obras
                let sq = |cx: f32, cy: f32| -> f32 {
                    let h = 0.26;
                    seg(cx - h, cy - h, cx + h, cy - h, w * 0.8)
                        .max(seg(cx + h, cy - h, cx + h, cy + h, w * 0.8))
                        .max(seg(cx + h, cy + h, cx - h, cy + h, w * 0.8))
                        .max(seg(cx - h, cy + h, cx - h, cy - h, w * 0.8))
                };
                sq(-0.36, 0.36).max(sq(0.36, 0.36)).max(sq(-0.36, -0.36)).max(sq(0.36, -0.36))
            }
            "flecha_der" => seg(-0.25, 0.5, 0.3, 0.0, w * 1.2).max(seg(0.3, 0.0, -0.25, -0.5, w * 1.2)),
            "flecha_izq" => seg(0.25, 0.5, -0.3, 0.0, w * 1.2).max(seg(-0.3, 0.0, 0.25, -0.5, w * 1.2)),
            "salir" => {
                // una cruz
                seg(-0.45, -0.45, 0.45, 0.45, w).max(seg(-0.45, 0.45, 0.45, -0.45, w))
            }
            "contemplar" => {
                // un ojo: almendra y pupila
                let lid = |sgn: f32| -> f32 {
                    let yy = sgn * 0.55 * (1.0 - x * x / 0.81).max(0.0).sqrt();
                    (1.0 - ((y - yy).abs() - w) * 14.0).clamp(0.0, 1.0) * ((x.abs() < 0.9) as i32 as f32)
                };
                lid(1.0).max(lid(-1.0)).max(disc(0.0, 0.0, 0.2))
            }
            "capa_superficie" | "capa_agua" | "capa_fondo" => {
                // marco con una línea en la altura de la capa
                let yy = match kind.as_str() {
                    "capa_superficie" => 0.45,
                    "capa_agua" => 0.0,
                    _ => -0.45,
                };
                let frame = seg(-0.6, -0.6, 0.6, -0.6, w * 0.6)
                    .max(seg(0.6, -0.6, 0.6, 0.6, w * 0.6))
                    .max(seg(0.6, 0.6, -0.6, 0.6, w * 0.6))
                    .max(seg(-0.6, 0.6, -0.6, -0.6, w * 0.6));
                (frame * 0.5).max(seg(-0.45, yy, 0.45, yy, w * 1.3))
            }
            _ => 0.0,
        };
        [1.0, 1.0, 1.0, a]
    })
}

/// Disco con los colores de un agua (cielo arriba, abismo abajo).
pub fn water_icon(top: [f32; 3], bottom: [f32; 3]) -> Image {
    make(64, 64, move |u, v| {
        let x = (u - 0.5) * 2.0;
        let y = (0.5 - v) * 2.0;
        let d = (x * x + y * y).sqrt();
        let a = (1.0 - (d - 0.82) * 14.0).clamp(0.0, 1.0);
        let t = ((y + 0.8) / 1.6).clamp(0.0, 1.0);
        let t = t * t * (3.0 - 2.0 * t);
        [
            bottom[0] + (top[0] - bottom[0]) * t,
            bottom[1] + (top[1] - bottom[1]) * t,
            bottom[2] + (top[2] - bottom[2]) * t,
            a,
        ]
    })
}

/// Blanco liso (para materiales que solo usan el color).
pub fn white() -> Image {
    make(4, 4, |_, _| [1.0, 1.0, 1.0, 1.0])
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
        // muy transparente: las manchas se superponen como acuarela
        [1.0, 1.0, 1.0, a * 0.32]
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
        // se enciende suavemente bajo la superficie (nunca un corte arriba) y
        // se disuelve mucho antes del fondo
        let top = (v / 0.12).clamp(0.0, 1.0);
        let top = top * top * (3.0 - 2.0 * top);
        let fall = top * (1.0 - ((v - 0.12) / 0.30).clamp(0.0, 1.0)).powf(1.5);
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
    // ruido de valor *tileable*: la red de enteros se envuelve en el periodo
    let noise = move |x: f32, y: f32, period: i32| -> f32 {
        let (xi, yi) = (x.floor() as i32, y.floor() as i32);
        let (fx, fy) = (x - xi as f32, y - yi as f32);
        let (sx, sy) = (fx * fx * (3.0 - 2.0 * fx), fy * fy * (3.0 - 2.0 * fy));
        let w = |a: i32| a.rem_euclid(period);
        let a = hash(w(xi), w(yi)) * (1.0 - sx) + hash(w(xi + 1), w(yi)) * sx;
        let b = hash(w(xi), w(yi + 1)) * (1.0 - sx) + hash(w(xi + 1), w(yi + 1)) * sx;
        a * (1.0 - sy) + b * sy
    };
    make(512, 512, move |u, v| {
        // grano fino y ondulaciones suaves, sin manchas grandes en cuadrícula
        let n = 0.25 * noise(u * 7.0, v * 7.0, 7)
            + 0.35 * noise(u * 23.0, v * 23.0, 23)
            + 0.25 * noise(u * 61.0, v * 61.0, 61)
            + 0.15 * noise(u * 160.0, v * 160.0, 160);
        let k = 0.86 + 0.28 * n;
        [color[0] * k, color[1] * k, color[2] * k, 1.0]
    })
}
