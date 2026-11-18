//! Lo tonal. Una escala por mundo y un ciclo lento de acordes; todo lo que
//! suena con altura pasa por aquí, y por eso nada puede sonar mal.

/// Re pentatónica mayor: D E F# A B. Grados 0..5 por octava.
pub const PENTA: [i32; 5] = [0, 2, 4, 7, 9];
/// Grado 0 = D3.
pub const ROOT_MIDI: i32 = 50;
pub const DEGREES_PER_OCTAVE: i32 = 5;

pub fn degree_to_midi(deg: i32) -> i32 {
    let oct = deg.div_euclid(DEGREES_PER_OCTAVE);
    let cls = deg.rem_euclid(DEGREES_PER_OCTAVE) as usize;
    ROOT_MIDI + 12 * oct + PENTA[cls]
}

pub fn degree_class(deg: i32) -> i32 {
    deg.rem_euclid(DEGREES_PER_OCTAVE)
}

/// Un acorde como conjunto de clases de grado.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Chord {
    pub name: &'static str,
    pub classes: [i32; 3],
}

impl Chord {
    pub fn contains(&self, deg: i32) -> bool {
        self.classes.contains(&degree_class(deg))
    }

    /// El grado más cercano a `deg` que pertenece al acorde (empate: hacia abajo).
    pub fn nearest(&self, deg: i32) -> i32 {
        if self.contains(deg) {
            return deg;
        }
        for d in 1..DEGREES_PER_OCTAVE {
            if self.contains(deg - d) {
                return deg - d;
            }
            if self.contains(deg + d) {
                return deg + d;
            }
        }
        deg
    }
}

/// El ciclo armónico del abismo. Cada acorde dura `CHORD_BEATS` pulsos.
/// D (0 2 3 = D F# A) · Bm (4 0 2 = B D F#) · Asus (3 0 1 = A D E) · D
pub const CYCLE: [Chord; 4] = [
    Chord { name: "D", classes: [0, 2, 3] },
    Chord { name: "Bm", classes: [4, 0, 2] },
    Chord { name: "Asus", classes: [3, 0, 1] },
    Chord { name: "D", classes: [0, 2, 3] },
];
pub const CHORD_BEATS: f64 = 32.0;

pub fn chord_at(beat: f64) -> Chord {
    let cycle_len = CHORD_BEATS * CYCLE.len() as f64;
    let pos = beat.rem_euclid(cycle_len);
    CYCLE[(pos / CHORD_BEATS) as usize % CYCLE.len()]
}

/// Grados del acorde vigente, en el registro del colchón (una voz por clase).
pub fn pad_voicing(beat: f64) -> Vec<i32> {
    let ch = chord_at(beat);
    // Quintas abiertas: bajo en D2 y luego el acorde repartido hacia arriba.
    let mut out = vec![];
    for (i, &cls) in ch.classes.iter().enumerate() {
        let base = -5 + 5 * i as i32; // una octava por voz
        out.push(base + cls);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn midi_mapping() {
        assert_eq!(degree_to_midi(0), 50); // D3
        assert_eq!(degree_to_midi(3), 57); // A3
        assert_eq!(degree_to_midi(5), 62); // D4
        assert_eq!(degree_to_midi(-5), 38); // D2
        assert_eq!(degree_to_midi(-1), 47); // B2
    }

    #[test]
    fn nearest_chord_tone() {
        let d = CYCLE[0];
        assert_eq!(d.nearest(0), 0);
        assert_eq!(d.nearest(1), 0);
        assert_eq!(d.nearest(4), 3);
        assert_eq!(d.nearest(6), 5);
    }

    #[test]
    fn cycle_wraps() {
        assert_eq!(chord_at(0.0).name, "D");
        assert_eq!(chord_at(33.0).name, "Bm");
        assert_eq!(chord_at(128.0 + 70.0).name, "Asus");
    }
}
