//! Tema y variaciones. Cada vez que un motivo vuelve, el director le aplica una
//! o dos de estas operaciones, elegidas por semilla y número de aparición.

use crate::motif::{Motif, Note};
use crate::rng::Rng;
use crate::theory::Chord;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Op {
    /// Mueve todo `n` grados dentro de la escala.
    Transpose(i32),
    /// Espejo alrededor de la primera nota.
    Invert,
    /// Al revés.
    Retrograde,
    /// Más lento (>1) o más rápido (<1).
    Augment(f64),
    /// Quita notas al azar, conservando la primera.
    Thin(f32),
    /// Repite cada nota más suave, un poco después.
    Echo,
    /// Una octava arriba o abajo.
    Octave(i32),
}

pub fn apply(m: &Motif, op: Op, rng: &mut Rng) -> Motif {
    match op {
        Op::Transpose(n) => Motif {
            notes: m
                .notes
                .iter()
                .map(|x| Note {
                    degree: x.degree + n,
                    ..x.clone()
                })
                .collect(),
            beats: m.beats,
        },
        Op::Octave(n) => apply(m, Op::Transpose(5 * n), rng),
        Op::Invert => {
            let axis = m.notes.first().map(|n| n.degree).unwrap_or(0);
            Motif {
                notes: m
                    .notes
                    .iter()
                    .map(|x| Note {
                        degree: axis - (x.degree - axis),
                        ..x.clone()
                    })
                    .collect(),
                beats: m.beats,
            }
        }
        Op::Retrograde => {
            let mut notes: Vec<Note> = m
                .notes
                .iter()
                .map(|x| Note {
                    at: (m.beats - x.at - x.beats).max(0.0),
                    ..x.clone()
                })
                .collect();
            notes.sort_by(|a, b| a.at.partial_cmp(&b.at).unwrap());
            Motif {
                notes,
                beats: m.beats,
            }
        }
        Op::Augment(f) => Motif {
            notes: m
                .notes
                .iter()
                .map(|x| Note {
                    at: x.at * f,
                    beats: x.beats * f,
                    ..x.clone()
                })
                .collect(),
            beats: m.beats * f,
        },
        Op::Thin(p) => Motif {
            notes: m
                .notes
                .iter()
                .enumerate()
                .filter(|(i, _)| *i == 0 || !rng.chance(p as f64))
                .map(|(_, x)| x.clone())
                .collect(),
            beats: m.beats,
        },
        Op::Echo => {
            let mut notes = m.notes.clone();
            for x in &m.notes {
                notes.push(Note {
                    at: x.at + 0.75,
                    vel: x.vel * 0.45,
                    ..x.clone()
                });
            }
            notes.sort_by(|a, b| a.at.partial_cmp(&b.at).unwrap());
            Motif {
                notes,
                beats: m.beats + 0.75,
            }
        }
    }
}

/// Qué variación toca en la aparición `k` de un motivo. La primera vez suena
/// tal cual; después, una operación; más adelante, a veces dos.
pub fn choose(k: u32, rng: &mut Rng) -> Vec<Op> {
    if k == 0 {
        return vec![];
    }
    let pool = [
        Op::Transpose(2),
        Op::Transpose(-2),
        Op::Transpose(1),
        Op::Invert,
        Op::Retrograde,
        Op::Augment(1.5),
        Op::Augment(0.75),
        Op::Thin(0.35),
        Op::Echo,
        Op::Octave(1),
        Op::Octave(-1),
    ];
    let mut ops = vec![*rng.pick(&pool)];
    if k >= 3 && rng.chance(0.4) {
        let second = *rng.pick(&pool);
        if second != ops[0] {
            ops.push(second);
        }
    }
    ops
}

/// Reajusta al acorde vigente: las notas en tiempo fuerte van al tono de
/// acorde más cercano; las demás se quedan (son de paso).
pub fn fit_to_chord(m: &Motif, chord: Chord) -> Motif {
    Motif {
        notes: m
            .notes
            .iter()
            .map(|x| {
                let strong = (x.at / 1.0).fract().abs() < 1e-6 && (x.at as i64) % 2 == 0;
                if strong || x.beats >= 2.0 {
                    Note {
                        degree: chord.nearest(x.degree),
                        ..x.clone()
                    }
                } else {
                    x.clone()
                }
            })
            .collect(),
        beats: m.beats,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m() -> Motif {
        Motif {
            notes: vec![
                Note {
                    degree: 0,
                    at: 0.0,
                    beats: 1.0,
                    vel: 0.8,
                },
                Note {
                    degree: 2,
                    at: 1.0,
                    beats: 1.0,
                    vel: 0.8,
                },
                Note {
                    degree: 4,
                    at: 2.0,
                    beats: 1.0,
                    vel: 0.8,
                },
            ],
            beats: 3.0,
        }
    }

    #[test]
    fn invert_mirrors() {
        let mut r = Rng::new(1);
        let v = apply(&m(), Op::Invert, &mut r);
        assert_eq!(
            v.notes.iter().map(|n| n.degree).collect::<Vec<_>>(),
            vec![0, -2, -4]
        );
    }

    #[test]
    fn retrograde_reverses() {
        let mut r = Rng::new(1);
        let v = apply(&m(), Op::Retrograde, &mut r);
        assert_eq!(
            v.notes.iter().map(|n| n.degree).collect::<Vec<_>>(),
            vec![4, 2, 0]
        );
    }

    #[test]
    fn first_appearance_untouched() {
        let mut r = Rng::new(1);
        assert!(choose(0, &mut r).is_empty());
        assert!(!choose(1, &mut r).is_empty());
    }
}
