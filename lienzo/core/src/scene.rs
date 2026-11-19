//! La obra. Es lo que se guarda, se comparte y se abre: una lista de trazos.
//! La música no se guarda; se deriva de esto cada vez.

use serde::{Deserialize, Serialize};

/// Posición en el volumen del mundo.
/// x: -1 (izquierda) .. 1 (derecha) · y: 0 (suelo) .. 1 (superficie) ·
/// z: -1 (fondo, lejos) .. 1 (frente, cerca de la cámara).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Point {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Point { x, y, z }
    }

    pub fn dist(&self, o: &Point) -> f32 {
        ((self.x - o.x).powi(2) + (self.y - o.y).powi(2) + (self.z - o.z).powi(2)).sqrt()
    }
}

/// Los pinceles semánticos del mundo submarino. Añadir uno es una variante aquí
/// más su entrada en `brush::spec`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Brush {
    Kelp,
    Coral,
    Anemona,
    Cardumen,
    Medusa,
    Burbujas,
    Ballena,
    Barco,
    /// Trazo de acuarela: color libre que se queda en el agua. Familia Color
    /// (DISEÑO §11.3); musicalmente es colchón.
    Color,
}

impl Brush {
    pub const ALL: [Brush; 9] = [
        Brush::Kelp,
        Brush::Coral,
        Brush::Anemona,
        Brush::Cardumen,
        Brush::Medusa,
        Brush::Burbujas,
        Brush::Ballena,
        Brush::Barco,
        Brush::Color,
    ];
}

/// Un gesto del autor. Un toque suelto es un trazo de un solo punto.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Stroke {
    pub brush: Brush,
    pub points: Vec<Point>,
    /// Duración del gesto en milisegundos. Da la dinámica: rápido = marcato.
    #[serde(default = "default_gesture_ms")]
    pub gesture_ms: u32,
    /// Color del trazo (sRGB 0..1), solo para la familia Color.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<[f32; 3]>,
}

fn default_gesture_ms() -> u32 {
    800
}

impl Stroke {
    pub fn tap(brush: Brush, p: Point) -> Self {
        Stroke {
            brush,
            points: vec![p],
            gesture_ms: 150,
            color: None,
        }
    }

    pub fn line(brush: Brush, points: Vec<Point>, gesture_ms: u32) -> Self {
        Stroke {
            brush,
            points,
            gesture_ms,
            color: None,
        }
    }

    pub fn with_color(mut self, rgb: [f32; 3]) -> Self {
        self.color = Some(rgb);
        self
    }

    /// Tono del color en grados (0..360), si el trazo tiene color.
    pub fn hue(&self) -> Option<f32> {
        let [r, g, b] = self.color?;
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let d = max - min;
        if d < 1e-4 {
            return Some(0.0);
        }
        let h = if max == r {
            60.0 * (((g - b) / d) % 6.0)
        } else if max == g {
            60.0 * ((b - r) / d + 2.0)
        } else {
            60.0 * ((r - g) / d + 4.0)
        };
        Some(h.rem_euclid(360.0))
    }

    pub fn length(&self) -> f32 {
        self.points.windows(2).map(|w| w[0].dist(&w[1])).sum()
    }

    pub fn centroid(&self) -> Point {
        let n = self.points.len().max(1) as f32;
        let (mut x, mut y, mut z) = (0.0, 0.0, 0.0);
        for p in &self.points {
            x += p.x;
            y += p.y;
            z += p.z;
        }
        Point::new(x / n, y / n, z / n)
    }

    /// Altura (y) muestreada en `n` puntos equidistantes a lo largo del trazo.
    pub fn heights(&self, n: usize) -> Vec<f32> {
        if self.points.len() < 2 {
            return vec![self.points.first().map(|p| p.y).unwrap_or(0.5); n];
        }
        let total = self.length().max(1e-6);
        (0..n)
            .map(|i| {
                let target = total * i as f32 / (n - 1).max(1) as f32;
                let mut acc = 0.0;
                for w in self.points.windows(2) {
                    let seg = w[0].dist(&w[1]);
                    if acc + seg >= target || seg == 0.0 {
                        let t = if seg == 0.0 {
                            0.0
                        } else {
                            (target - acc) / seg
                        };
                        return w[0].y + (w[1].y - w[0].y) * t;
                    }
                    acc += seg;
                }
                self.points.last().unwrap().y
            })
            .collect()
    }

    /// Velocidad media del gesto en unidades de mundo por segundo.
    pub fn speed(&self) -> f32 {
        self.length() / (self.gesture_ms.max(1) as f32 / 1000.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum World {
    Abismo,
}

/// El agua: color, luz y tonalidad de la obra. Se puede cambiar siempre; la
/// misma obra "amanece" en otro mar. Añadir una es dato en `theory::tonality` y
/// en la app, no código nuevo.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Water {
    #[default]
    Abismo,
    Caribe,
    Hawai,
    Cristalina,
}

impl Water {
    pub const ALL: [Water; 4] = [
        Water::Abismo,
        Water::Caribe,
        Water::Hawai,
        Water::Cristalina,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            Water::Abismo => "Abismo",
            Water::Caribe => "Caribe",
            Water::Hawai => "Hawái",
            Water::Cristalina => "Cristalina",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Scene {
    pub version: u32,
    pub world: World,
    #[serde(default)]
    pub water: Water,
    pub seed: u64,
    #[serde(default)]
    pub title: String,
    pub strokes: Vec<Stroke>,
}

impl Scene {
    pub fn new(seed: u64) -> Self {
        Scene {
            version: 1,
            world: World::Abismo,
            water: Water::Abismo,
            seed,
            title: String::new(),
            strokes: vec![],
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("escena serializable")
    }

    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }

    /// La obra que se usó en la Fase 0, para tener algo que escuchar desde el
    /// primer día sin app.
    pub fn demo() -> Self {
        let mut s = Scene::new(7);
        s.title = "Abismo (demo)".into();
        let curve = |n: usize, x0: f32, x1: f32, y0: f32, amp: f32, z: f32| -> Vec<Point> {
            (0..n)
                .map(|i| {
                    let u = i as f32 / (n - 1) as f32;
                    Point::new(x0 + (x1 - x0) * u, y0 + amp * (u * 6.9).sin(), z)
                })
                .collect()
        };
        // kelp: un trazo que sube y baja, lento
        s.strokes.push(Stroke::line(
            Brush::Kelp,
            vec![
                Point::new(-0.8, 0.10, -0.3),
                Point::new(-0.6, 0.35, -0.3),
                Point::new(-0.4, 0.70, -0.3),
                Point::new(-0.2, 0.90, -0.3),
                Point::new(0.0, 0.75, -0.3),
                Point::new(0.2, 0.40, -0.3),
                Point::new(0.4, 0.20, -0.3),
            ],
            2600,
        ));
        // cardumen: rápido y ondulado
        s.strokes.push(Stroke::line(
            Brush::Cardumen,
            curve(9, -0.3, 0.6, 0.55, 0.18, 0.2),
            700,
        ));
        s.strokes
            .push(Stroke::tap(Brush::Anemona, Point::new(0.1, 0.05, 0.6)));
        s.strokes
            .push(Stroke::tap(Brush::Medusa, Point::new(0.6, 0.6, 0.4)));
        s.strokes.push(Stroke::line(
            Brush::Coral,
            curve(5, 0.2, 0.7, 0.05, 0.03, 0.3),
            1500,
        ));
        s.strokes
            .push(Stroke::tap(Brush::Ballena, Point::new(-0.7, 0.8, -0.9)));
        s.strokes
            .push(Stroke::tap(Brush::Barco, Point::new(0.7, 0.0, -0.2)));
        s.strokes.push(Stroke::line(
            Brush::Burbujas,
            curve(6, 0.3, 0.35, 0.2, 0.05, 0.5),
            900,
        ));
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_json() {
        let s = Scene::demo();
        let j = s.to_json();
        let back = Scene::from_json(&j).unwrap();
        assert_eq!(back.strokes.len(), s.strokes.len());
        assert_eq!(back.seed, s.seed);
    }

    #[test]
    fn heights_follow_contour() {
        let st = Stroke::line(
            Brush::Kelp,
            vec![Point::new(0.0, 0.0, 0.0), Point::new(1.0, 1.0, 0.0)],
            1000,
        );
        let h = st.heights(3);
        assert!((h[0] - 0.0).abs() < 1e-5);
        assert!((h[1] - 0.5).abs() < 1e-5);
        assert!((h[2] - 1.0).abs() < 1e-5);
    }
}
