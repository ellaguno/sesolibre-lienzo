//! El director. Convierte la escena en una línea de tiempo de eventos: decide
//! cuándo vuelve cada motivo, con qué variación, cómo respira la pieza por
//! secciones y cuándo actúan los elementos naturales.
//!
//! No sabe de muestras ni de mezcla: produce eventos. El render los toca.

use crate::brush::{spec_for, Attach, Instrument};
use crate::motif::{self, Motif};
use crate::rng::{hash, Rng};
use crate::scene::{Brush, Scene, Stroke};
use crate::theory::{chord_at, pad_voicing, tonality, Tonality};
use crate::variation;

/// Tempo de referencia (Abismo). El real lo pone el agua: `theory::tonality`.
pub const BPM: f64 = 56.0;
/// Duración de la forma completa (intro → crecimiento → meseta → adelgazamiento).
pub const FORM_SECONDS: f64 = 300.0;

#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    Note {
        t: f64,
        instrument: Instrument,
        midi: i32,
        vel: f32,
        dur: f64,
        pan: f32,
        /// -1 lejos .. 1 cerca
        depth: f32,
        /// Índice del trazo que lo produce (None: colchón).
        stroke: Option<usize>,
    },
    Natural {
        t: f64,
        name: &'static str,
        gain: f32,
        pan: f32,
        depth: f32,
        stroke: usize,
    },
    Ambience {
        name: &'static str,
        gain: f32,
    },
}

impl Event {
    pub fn time(&self) -> f64 {
        match self {
            Event::Note { t, .. } | Event::Natural { t, .. } => *t,
            Event::Ambience { .. } => 0.0,
        }
    }
}

pub fn beat_seconds() -> f64 {
    60.0 / BPM
}

pub fn beat_seconds_for(scene: &Scene) -> f64 {
    tonality(scene.water).beat_seconds()
}

/// Densidad 0.35..1 a lo largo de la forma. Intro callada, crecimiento,
/// meseta, adelgazamiento; y vuelve a empezar.
pub fn density(t: f64) -> f64 {
    let u = (t / FORM_SECONDS).rem_euclid(1.0);
    let d = if u < 0.15 {
        0.35 + 0.3 * (u / 0.15)
    } else if u < 0.45 {
        0.65 + 0.35 * ((u - 0.15) / 0.30)
    } else if u < 0.70 {
        1.0
    } else {
        1.0 - 0.6 * ((u - 0.70) / 0.30)
    };
    d.clamp(0.35, 1.0)
}

fn pan_of(s: &Stroke) -> f32 {
    s.centroid().x.clamp(-1.0, 1.0) * 0.8
}

fn depth_of(s: &Stroke) -> f32 {
    s.centroid().z.clamp(-1.0, 1.0)
}

fn beats_to_seconds(ton: &Tonality, b: f64) -> f64 {
    b * ton.beat_seconds()
}

fn schedule_motif(
    out: &mut Vec<Event>,
    ton: &Tonality,
    m: &Motif,
    t0: f64,
    instrument: Instrument,
    pan: f32,
    depth: f32,
    stroke: usize,
    gain: f32,
) {
    let chord = chord_at(t0 / ton.beat_seconds());
    let fitted = variation::fit_to_chord(m, chord);
    for n in &fitted.notes {
        out.push(Event::Note {
            t: t0 + beats_to_seconds(ton, n.at),
            instrument,
            midi: ton.degree_to_midi(n.degree),
            vel: (n.vel * gain).clamp(0.0, 1.0),
            dur: beats_to_seconds(ton, n.beats),
            pan,
            depth,
            stroke: Some(stroke),
        });
    }
}

fn compose_stroke(out: &mut Vec<Event>, scene: &Scene, i: usize, seconds: f64) {
    let ton = tonality(scene.water);
    let s = &scene.strokes[i];
    let sp = spec_for(s);
    let seed = scene.seed;
    let pan = pan_of(s);
    let depth = depth_of(s);

    if let (Some(inst), Some(base)) = (sp.instrument, motif::from_stroke(s, seed, i)) {
        let mut rng = Rng::from_parts(&[seed, i as u64, 0x6469_7265]);
        // fase inicial: no todos entran a la vez
        let mut t = rng.range(0.5, sp.interval_s.0 * 0.6);
        let mut k: u32 = 0;
        while t < seconds {
            let d = density(t);
            let feel = 0.85 * ton.feel;
            if rng.chance(d * feel) {
                let mut m = base.clone();
                for op in variation::choose(k, &mut rng) {
                    m = variation::apply(&m, op, &mut rng);
                }
                // que la variación no se salga del registro del pincel
                let lo = sp.lo_degree - 3;
                let hi = sp.lo_degree + sp.span_degrees + 3;
                if m.lowest() < lo {
                    m = variation::apply(&m, variation::Op::Octave(1), &mut rng);
                } else if m.highest() > hi {
                    m = variation::apply(&m, variation::Op::Octave(-1), &mut rng);
                }
                let gain = 0.75 + 0.25 * d as f32;
                schedule_motif(out, &ton, &m, t, inst, pan, depth, i, gain);
                k += 1;
            }
            let interval = rng.range(sp.interval_s.0, sp.interval_s.1);
            t += interval * (1.45 - 0.6 * d);
        }
    }

    if let Some(name) = sp.natural {
        let mut rng = Rng::from_parts(&[seed, i as u64, 0x6e61_7475]);
        let mut t = rng.range(3.0, sp.natural_interval_s.0);
        while t < seconds {
            out.push(Event::Natural {
                t,
                name,
                gain: 0.7 + 0.3 * rng.f32(),
                pan,
                depth,
                stroke: i,
            });
            t += rng.range(sp.natural_interval_s.0, sp.natural_interval_s.1);
        }
    }
}

/// El colchón: el acorde vigente sostenido por cuerdas, renovado cada 6 s con
/// fundidos largos. Su nivel crece con la obra, pero nunca falta del todo.
fn compose_pad(out: &mut Vec<Event>, scene: &Scene, seconds: f64) {
    let ton = tonality(scene.water);
    let mut rng = Rng::from_parts(&[scene.seed, 0x7061_64]);
    let strokes = scene.strokes.len().min(6) as f32;
    let level = (0.25 + 0.75 * (strokes / 6.0)) * ton.pad_level;
    let mut t = 2.0;
    while t < seconds {
        let beat = t / ton.beat_seconds();
        for deg in pad_voicing(beat) {
            let midi = ton.degree_to_midi(deg);
            let inst = if midi < 55 {
                Instrument::CelloSus
            } else if midi < 67 {
                Instrument::ViolaSus
            } else {
                Instrument::ViolinSus
            };
            out.push(Event::Note {
                t: t + rng.range(0.0, 0.4),
                instrument: inst,
                midi,
                vel: (0.32 * level * density(t) as f32 * (0.9 + 0.1 * rng.f32())).clamp(0.05, 1.0),
                dur: 7.5,
                pan: rng.range(-0.5, 0.5) as f32,
                depth: -0.4,
                stroke: None,
            });
        }
        t += 6.0;
    }
}

fn compose_ambience(out: &mut Vec<Event>, scene: &Scene) {
    let bubbles: f32 = scene
        .strokes
        .iter()
        .map(|s| spec_for(s).bubbles)
        .sum::<f32>()
        .min(2.0)
        / 2.0;
    let near_surface = scene
        .strokes
        .iter()
        .filter(|s| s.centroid().y > 0.75)
        .count() as f32;
    out.push(Event::Ambience {
        name: "hydrophone_flow",
        gain: 1.0,
    });
    out.push(Event::Ambience {
        name: "bubble_flow",
        gain: 0.25 + 0.6 * bubbles,
    });
    out.push(Event::Ambience {
        name: "waves_rocks",
        gain: (0.15 + 0.2 * near_surface.min(3.0)).min(0.6),
    });
}

/// La obra como lista de eventos ordenada en el tiempo.
pub fn compose(scene: &Scene, seconds: f64) -> Vec<Event> {
    let mut out = Vec::new();
    compose_ambience(&mut out, scene);
    compose_pad(&mut out, scene, seconds);
    for i in 0..scene.strokes.len() {
        compose_stroke(&mut out, scene, i, seconds);
    }
    // límite de voces por instante: si una obra enorme apila demasiado, se
    // quitan las notas más suaves (determinista: mismo orden, mismo resultado)
    out.sort_by(|a, b| a.time().partial_cmp(&b.time()).unwrap());
    out
}

/// Resumen legible, para el CLI y para depurar sin oír.
pub fn describe(scene: &Scene, events: &[Event]) -> String {
    let mut s = format!(
        "{} trazos · semilla {} · {} eventos\n",
        scene.strokes.len(),
        scene.seed,
        events.len()
    );
    let notes = events
        .iter()
        .filter(|e| matches!(e, Event::Note { .. }))
        .count();
    let nat = events
        .iter()
        .filter(|e| matches!(e, Event::Natural { .. }))
        .count();
    s += &format!("  notas {notes} · naturales {nat}\n");
    for (i, st) in scene.strokes.iter().enumerate() {
        let sp = spec_for(st);
        let m = motif::from_stroke(st, scene.seed, i);
        s += &format!(
            "  [{i}] {:?} {} pts · {:.2} u · {} ms → {}\n",
            st.brush,
            st.points.len(),
            st.length(),
            st.gesture_ms,
            match (&m, sp.instrument) {
                (Some(m), Some(inst)) => format!(
                    "{:?}: {} notas, grados {:?}, {:.1} pulsos",
                    inst,
                    m.notes.len(),
                    m.notes.iter().map(|n| n.degree).collect::<Vec<_>>(),
                    m.beats
                ),
                _ => format!("sin instrumento ({:?})", sp.natural),
            }
        );
        let _ = Attach::Floor;
    }
    let _ = hash(&[0]);
    let _ = Brush::ALL;
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_compose() {
        let s = Scene::demo();
        let a = compose(&s, 120.0);
        let b = compose(&s, 120.0);
        assert_eq!(a, b);
        assert!(a.len() > 50);
    }

    #[test]
    fn empty_scene_still_sounds() {
        let s = Scene::new(1);
        let ev = compose(&s, 60.0);
        assert!(ev.iter().any(|e| matches!(e, Event::Ambience { .. })));
        assert!(ev.iter().any(|e| matches!(e, Event::Note { .. })));
    }

    #[test]
    fn density_breathes() {
        assert!(density(0.0) < density(FORM_SECONDS * 0.5));
        assert!(density(FORM_SECONDS * 0.95) < density(FORM_SECONDS * 0.5));
    }
}
