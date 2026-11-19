//! Del trazo al motivo. El trazo ES la partitura: su contorno es la melodía,
//! su longitud la duración, su velocidad la dinámica.

use crate::brush::{spec_for, Articulation, BrushSpec};
use crate::rng::Rng;
use crate::scene::Stroke;

#[derive(Clone, Debug, PartialEq)]
pub struct Note {
    /// Grado de la escala (ver `theory`).
    pub degree: i32,
    /// Inicio, en pulsos desde el inicio del motivo.
    pub at: f64,
    /// Duración en pulsos.
    pub beats: f64,
    /// 0..1
    pub vel: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Motif {
    pub notes: Vec<Note>,
    /// Duración total en pulsos.
    pub beats: f64,
}

impl Motif {
    pub fn is_empty(&self) -> bool {
        self.notes.is_empty()
    }

    pub fn lowest(&self) -> i32 {
        self.notes.iter().map(|n| n.degree).min().unwrap_or(0)
    }

    pub fn highest(&self) -> i32 {
        self.notes.iter().map(|n| n.degree).max().unwrap_or(0)
    }
}

/// Cuántas notas da un trazo. Un toque: 1–3. Un trazo largo: hasta 10.
fn note_count(stroke: &Stroke, rng: &mut Rng) -> usize {
    if stroke.points.len() < 2 {
        return 1 + rng.below(3);
    }
    let len = stroke.length();
    ((len * 7.0).round() as usize).clamp(2, 10)
}

/// Velocidad del gesto → dinámica. Lento y largo = suave; rápido = marcato.
fn velocity(stroke: &Stroke) -> f32 {
    let v = stroke.speed();
    (0.5 + 0.5 * ((v - 0.2) / 1.8).clamp(0.0, 1.0)).clamp(0.45, 1.0)
}

pub fn from_stroke(stroke: &Stroke, seed: u64, index: usize) -> Option<Motif> {
    let sp: BrushSpec = spec_for(stroke);
    sp.instrument?;
    let mut rng = Rng::from_parts(&[seed, index as u64, 0x6d6f_7469]);
    let n = note_count(stroke, &mut rng);
    let heights = stroke.heights(n);
    let vel = velocity(stroke);

    // Para un toque no hay contorno: la altura del punto da la nota y el resto
    // se improvisa cerca, determinista.
    let base_from_h =
        |h: f32| sp.lo_degree + (h.clamp(0.0, 1.0) * sp.span_degrees as f32).round() as i32;

    let mut notes = Vec::with_capacity(n);
    let mut t = 0.0;
    for (i, &h) in heights.iter().enumerate() {
        let mut deg = base_from_h(h);
        if stroke.points.len() < 2 && i > 0 {
            deg += [-1, 1, 2][rng.below(3)];
        }
        let beats = match sp.articulation {
            Articulation::Sustain => sp.note_beats * rng.range(0.9, 1.3),
            Articulation::Pluck => sp.note_beats,
            Articulation::Chord => sp.note_beats,
        };
        let at = if sp.articulation == Articulation::Chord {
            0.0
        } else {
            t
        };
        notes.push(Note {
            degree: deg,
            at,
            beats,
            vel: vel * (0.92 + 0.08 * rng.f32()),
        });
        if sp.articulation != Articulation::Chord {
            // en sustain las notas se encadenan; en pluck llevan su paso
            t += match sp.articulation {
                Articulation::Sustain => beats,
                _ => sp.note_beats,
            };
        }
    }
    let total = match sp.articulation {
        Articulation::Chord => sp.note_beats,
        _ => t,
    };
    Some(Motif {
        notes,
        beats: total,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{Brush, Point};

    #[test]
    fn rising_stroke_rises() {
        let st = Stroke::line(
            Brush::Kelp,
            vec![Point::new(-1.0, 0.0, 0.0), Point::new(1.0, 1.0, 0.0)],
            2000,
        );
        let m = from_stroke(&st, 1, 0).unwrap();
        assert!(m.notes.len() >= 2);
        assert!(m.notes.first().unwrap().degree < m.notes.last().unwrap().degree);
        assert!(m.notes.iter().all(|n| n.degree >= 0 && n.degree <= 6));
    }

    #[test]
    fn barco_has_no_motif() {
        let st = Stroke::tap(Brush::Barco, Point::new(0.0, 0.0, 0.0));
        assert!(from_stroke(&st, 1, 0).is_none());
    }

    #[test]
    fn chord_notes_are_simultaneous() {
        let st = Stroke::tap(Brush::Medusa, Point::new(0.0, 0.5, 0.0));
        let m = from_stroke(&st, 3, 0).unwrap();
        assert!(m.notes.iter().all(|n| n.at == 0.0));
    }
}
