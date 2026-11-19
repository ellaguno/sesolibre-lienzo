//! La ruta de buceo: por dónde nada la cámara (y el oído) durante la
//! contemplación. La dibuja el autor (pincel Ruta) o, si no hay, se genera
//! una que recorre lo dibujado. Coordenadas del core.

use crate::scene::{Brush, Point, Scene};

/// La ruta dibujada más reciente, si existe.
pub fn drawn(scene: &Scene) -> Option<Vec<Point>> {
    scene
        .strokes
        .iter()
        .rev()
        .find(|s| s.brush == Brush::Ruta && s.points.len() >= 2)
        .map(|s| s.points.clone())
}

/// Ruta automática: un lazo que visita lo dibujado en orden angular alrededor
/// del centro, a media agua, sin acercarse demasiado al borde. Sin trazos, un
/// círculo.
pub fn auto(scene: &Scene) -> Vec<Point> {
    let mut pts: Vec<(f32, Point)> = scene
        .strokes
        .iter()
        .filter(|s| {
            !matches!(
                s.brush,
                Brush::Ruta | Brush::Elevar | Brush::Hundir | Brush::Color
            )
        })
        .map(|s| {
            let c = s.centroid();
            (c.z.atan2(c.x), c)
        })
        .collect();
    if pts.len() < 3 {
        return (0..12)
            .map(|i| {
                let a = i as f32 / 12.0 * std::f32::consts::TAU;
                Point::new(0.7 * a.cos(), 0.4, 0.7 * a.sin())
            })
            .collect();
    }
    pts.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    // se nada un poco por fuera de cada elemento, hacia el centro no: desde
    // el lado opuesto al centro, para verlo de frente
    pts.iter()
        .map(|(_, c)| {
            let r = (c.x * c.x + c.z * c.z).sqrt().max(0.2);
            let k = (r + 0.35).min(0.95) / r;
            // a media agua, algo por encima de lo que se mira
            Point::new(c.x * k, (c.y * 0.6 + 0.12).clamp(0.2, 0.55), c.z * k)
        })
        .collect()
}

/// La ruta efectiva de la obra: la dibujada o la automática.
pub fn of(scene: &Scene) -> Vec<Point> {
    drawn(scene).unwrap_or_else(|| auto(scene))
}

/// Punto de una ruta cerrada a lo largo de `u` ∈ [0, 1) (Catmull-Rom).
pub fn sample(route: &[Point], u: f32) -> Point {
    let n = route.len();
    if n == 0 {
        return Point::new(0.0, 0.5, 0.0);
    }
    if n == 1 {
        return route[0];
    }
    let u = u.rem_euclid(1.0) * n as f32;
    let i = u.floor() as usize % n;
    let t = u - u.floor();
    let p = |k: isize| route[(i as isize + k).rem_euclid(n as isize) as usize];
    let (p0, p1, p2, p3) = (p(-1), p(0), p(1), p(2));
    let cr = |a: f32, b: f32, c: f32, d: f32| {
        0.5 * (2.0 * b
            + (-a + c) * t
            + (2.0 * a - 5.0 * b + 4.0 * c - d) * t * t
            + (-a + 3.0 * b - 3.0 * c + d) * t * t * t)
    };
    // Catmull-Rom puede sobrepasar los puntos: la ruta nunca sale del agua
    Point::new(
        cr(p0.x, p1.x, p2.x, p3.x).clamp(-0.95, 0.95),
        cr(p0.y, p1.y, p2.y, p3.y).clamp(0.12, 0.9),
        cr(p0.z, p1.z, p2.z, p3.z).clamp(-0.95, 0.95),
    )
}

/// Longitud aproximada de la ruta cerrada, en unidades del core.
pub fn length(route: &[Point]) -> f32 {
    let steps = (route.len() * 8).max(8);
    (0..steps)
        .map(|i| {
            let a = sample(route, i as f32 / steps as f32);
            let b = sample(route, (i + 1) as f32 / steps as f32);
            a.dist(&b)
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_route_is_a_loop_inside_the_volume() {
        let s = Scene::demo();
        let r = of(&s);
        assert!(r.len() >= 3);
        for i in 0..50 {
            let p = sample(&r, i as f32 / 50.0);
            assert!(p.x.abs() <= 1.0 && p.z.abs() <= 1.0 && (0.0..=1.0).contains(&p.y));
        }
        assert!(length(&r) > 1.0);
    }
}
