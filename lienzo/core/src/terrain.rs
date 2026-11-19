//! El relieve del fondo. No se guarda un mapa de alturas: se deriva de los
//! trazos de las familias *elevar* y *hundir*, como todo lo demás de la obra.
//! Coordenadas del core (x, z ∈ [-1, 1]); la altura en unidades del volumen
//! (1 = de suelo a superficie).

use crate::scene::{Brush, Scene, Stroke};

/// Altura máxima de una cresta y profundidad de una fosa, en unidades del
/// volumen.
pub const RAISE: f32 = 0.32;
pub const LOWER: f32 = -0.22;
/// Radio base de la pincelada, en unidades del core.
pub const RADIUS: f32 = 0.16;

/// Radio de una pincelada: un gesto lento ensancha.
pub fn radius_of(stroke: &Stroke) -> f32 {
    let slow = (1.0 - (stroke.speed() / 2.5).clamp(0.0, 1.0)) * 0.6;
    RADIUS * (1.0 + slow)
}

fn amplitude_of(stroke: &Stroke) -> f32 {
    match stroke.brush {
        Brush::Elevar => RAISE,
        Brush::Hundir => LOWER,
        _ => 0.0,
    }
}

/// Altura del relieve en (x, z). Suma de campanas a lo largo de cada trazo;
/// la contribución de un trazo se satura para que repasar no apile.
pub fn height_at(scene: &Scene, x: f32, z: f32) -> f32 {
    let mut h = 0.0;
    for s in &scene.strokes {
        let a = amplitude_of(s);
        if a == 0.0 || s.points.is_empty() {
            continue;
        }
        let r = radius_of(s);
        let r2 = r * r;
        let mut best = 0.0f32;
        // distancia mínima a la polilínea
        for w in s
            .points
            .windows(2)
            .chain(std::iter::once(&s.points[..1.min(s.points.len())]))
        {
            let (ax, az) = (w[0].x, w[0].z);
            let (bx, bz) = if w.len() > 1 {
                (w[1].x, w[1].z)
            } else {
                (ax, az)
            };
            let (dx, dz) = (bx - ax, bz - az);
            let len2 = dx * dx + dz * dz;
            let t = if len2 < 1e-8 {
                0.0
            } else {
                (((x - ax) * dx + (z - az) * dz) / len2).clamp(0.0, 1.0)
            };
            let (px, pz) = (ax + dx * t, az + dz * t);
            let d2 = (x - px) * (x - px) + (z - pz) * (z - pz);
            let g = (-d2 / r2 * 2.0).exp();
            best = best.max(g);
        }
        h += a * best;
    }
    h.clamp(LOWER * 1.5, RAISE * 1.5)
}

/// ¿La obra tiene relieve?
pub fn has_relief(scene: &Scene) -> bool {
    scene
        .strokes
        .iter()
        .any(|s| matches!(s.brush, Brush::Elevar | Brush::Hundir))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::Point;

    #[test]
    fn flat_without_relief() {
        let s = Scene::demo();
        assert_eq!(height_at(&s, 0.0, 0.0), 0.0);
    }

    #[test]
    fn ridge_rises_and_fades() {
        let mut s = Scene::new(1);
        s.strokes.push(Stroke::line(
            Brush::Elevar,
            vec![Point::new(-0.5, 0.0, 0.0), Point::new(0.5, 0.0, 0.0)],
            1500,
        ));
        let on = height_at(&s, 0.0, 0.0);
        let far = height_at(&s, 0.0, 0.9);
        assert!(on > 0.2, "{on}");
        assert!(far.abs() < 0.01, "{far}");
    }
}
