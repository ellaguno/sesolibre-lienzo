//! Catálogo de pinceles: qué instrumento, en qué registro, con qué articulación
//! y qué sonido natural trae cada uno. Es dato; añadir un pincel no toca el
//! director ni el render.

use crate::scene::{Brush, Stroke};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Instrument {
    CelloSus,
    CelloPizz,
    ViolaSus,
    ViolinSus,
    ViolinHarm,
    Harp,
    Vibes,
    FluteSus,
    Tuba,
    Marimba,
}

impl Instrument {
    /// Carpeta en `assets/audio/instruments/`.
    pub fn dir(&self) -> &'static str {
        match self {
            Instrument::CelloSus => "cello_sus",
            Instrument::CelloPizz => "cello_pizz",
            Instrument::ViolaSus => "viola_sus",
            Instrument::ViolinSus => "violin_sus",
            Instrument::ViolinHarm => "violin_harm",
            Instrument::Harp => "harp",
            Instrument::Vibes => "vibes",
            Instrument::FluteSus => "flute_sus",
            Instrument::Tuba => "tuba",
            Instrument::Marimba => "marimba",
        }
    }

    pub const ALL: [Instrument; 10] = [
        Instrument::CelloSus,
        Instrument::CelloPizz,
        Instrument::ViolaSus,
        Instrument::ViolinSus,
        Instrument::ViolinHarm,
        Instrument::Harp,
        Instrument::Vibes,
        Instrument::FluteSus,
        Instrument::Tuba,
        Instrument::Marimba,
    ];

    /// Ataque y cola del envolvente, en segundos.
    pub fn envelope(&self) -> (f64, f64) {
        match self {
            Instrument::CelloSus | Instrument::ViolaSus | Instrument::ViolinSus => (0.35, 1.2),
            Instrument::ViolinHarm | Instrument::FluteSus => (0.25, 1.0),
            Instrument::Tuba => (0.3, 1.5),
            Instrument::Harp | Instrument::Vibes | Instrument::Marimba | Instrument::CelloPizz => {
                (0.005, 1.5)
            }
        }
    }

    /// Nivel de mezcla relativo, en dB. Las muestras ya vienen a RMS igual.
    pub fn gain_db(&self) -> f32 {
        match self {
            Instrument::Harp => -2.0,
            Instrument::Vibes => -5.0,
            Instrument::Marimba => -4.0,
            Instrument::ViolinHarm => -6.0,
            Instrument::Tuba => -3.0,
            _ => 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Articulation {
    /// Notas largas, ligadas.
    Sustain,
    /// Notas cortas que suenan y decaen solas.
    Pluck,
    /// Las notas del motivo suenan juntas, como acorde.
    Chord,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Attach {
    Floor,
    Water,
}

#[derive(Clone, Debug)]
pub struct BrushSpec {
    pub instrument: Option<Instrument>,
    pub articulation: Articulation,
    pub attach: Attach,
    /// Registro en grados de la escala: el trazo se mapea a [lo, lo + span].
    pub lo_degree: i32,
    pub span_degrees: i32,
    /// Duración base de cada nota, en pulsos.
    pub note_beats: f64,
    /// Separación entre apariciones del motivo, en segundos (mín, máx).
    pub interval_s: (f64, f64),
    /// Sonido natural que el elemento emite de vez en cuando.
    pub natural: Option<&'static str>,
    pub natural_interval_s: (f64, f64),
    /// Cuánto contribuye a la cama de burbujas (0..1).
    pub bubbles: f32,
}

/// La especificación de un trazo concreto: igual que `spec`, salvo que un
/// trazo de color elige instrumento por su tono: cálidos = cuerdas (viola),
/// verdes = flauta, fríos = armónicos de violín (cristal).
pub fn spec_for(stroke: &Stroke) -> BrushSpec {
    let mut sp = spec(stroke.brush);
    if stroke.brush == Brush::Color {
        if let Some(h) = stroke.hue() {
            sp.instrument = Some(if !(60.0..300.0).contains(&h) {
                Instrument::ViolaSus
            } else if h < 170.0 {
                Instrument::FluteSus
            } else {
                Instrument::ViolinHarm
            });
            // los fríos van más agudos
            if h >= 170.0 && h < 300.0 {
                sp.lo_degree += 5;
            }
        }
    }
    sp
}

pub fn spec(b: Brush) -> BrushSpec {
    match b {
        Brush::Kelp => BrushSpec {
            instrument: Some(Instrument::CelloSus),
            articulation: Articulation::Sustain,
            attach: Attach::Floor,
            lo_degree: 0,
            span_degrees: 6,
            note_beats: 2.0,
            interval_s: (18.0, 30.0),
            natural: None,
            natural_interval_s: (0.0, 0.0),
            bubbles: 0.1,
        },
        Brush::Coral => BrushSpec {
            instrument: Some(Instrument::Marimba),
            articulation: Articulation::Pluck,
            attach: Attach::Floor,
            lo_degree: 5,
            span_degrees: 7,
            note_beats: 1.0,
            interval_s: (14.0, 24.0),
            natural: None,
            natural_interval_s: (0.0, 0.0),
            bubbles: 0.0,
        },
        Brush::Anemona => BrushSpec {
            instrument: Some(Instrument::Vibes),
            articulation: Articulation::Pluck,
            attach: Attach::Floor,
            lo_degree: 10,
            span_degrees: 6,
            note_beats: 2.0,
            interval_s: (10.0, 20.0),
            natural: None,
            natural_interval_s: (0.0, 0.0),
            bubbles: 0.0,
        },
        Brush::Cardumen => BrushSpec {
            instrument: Some(Instrument::Harp),
            articulation: Articulation::Pluck,
            attach: Attach::Water,
            lo_degree: 5,
            span_degrees: 8,
            note_beats: 0.5,
            interval_s: (12.0, 22.0),
            natural: None,
            natural_interval_s: (0.0, 0.0),
            bubbles: 0.25,
        },
        Brush::Medusa => BrushSpec {
            instrument: Some(Instrument::ViolinSus),
            articulation: Articulation::Chord,
            attach: Attach::Water,
            lo_degree: 8,
            span_degrees: 6,
            note_beats: 4.0,
            interval_s: (20.0, 35.0),
            natural: None,
            natural_interval_s: (0.0, 0.0),
            bubbles: 0.0,
        },
        Brush::Burbujas => BrushSpec {
            instrument: Some(Instrument::Vibes),
            articulation: Articulation::Pluck,
            attach: Attach::Water,
            lo_degree: 14,
            span_degrees: 6,
            note_beats: 0.25,
            interval_s: (8.0, 16.0),
            natural: None,
            natural_interval_s: (0.0, 0.0),
            bubbles: 1.0,
        },
        Brush::Ballena => BrushSpec {
            instrument: Some(Instrument::Tuba),
            articulation: Articulation::Sustain,
            attach: Attach::Water,
            lo_degree: -12,
            span_degrees: 5,
            note_beats: 3.0,
            interval_s: (40.0, 70.0),
            natural: Some("whale"),
            natural_interval_s: (35.0, 60.0),
            bubbles: 0.0,
        },
        // colchón: notas largas y ligadas, pocas, en el registro medio
        Brush::Color => BrushSpec {
            instrument: Some(Instrument::ViolaSus),
            articulation: Articulation::Sustain,
            attach: Attach::Water,
            lo_degree: 3,
            span_degrees: 6,
            note_beats: 4.0,
            interval_s: (24.0, 40.0),
            natural: None,
            natural_interval_s: (0.0, 0.0),
            bubbles: 0.0,
        },
        // Luz: lo agudo y brillante. El farol, notas sueltas de vibráfono;
        // el haz, un arpegio de arpa que recorre el rayo; el arcoíris, un acorde
        // de armónicos que se abre despacio.
        Brush::Farol => BrushSpec {
            instrument: Some(Instrument::Vibes),
            articulation: Articulation::Pluck,
            attach: Attach::Water,
            lo_degree: 12,
            span_degrees: 5,
            note_beats: 1.5,
            interval_s: (10.0, 20.0),
            natural: None,
            natural_interval_s: (0.0, 0.0),
            bubbles: 0.0,
        },
        Brush::Haz => BrushSpec {
            instrument: Some(Instrument::Harp),
            articulation: Articulation::Pluck,
            attach: Attach::Water,
            lo_degree: 10,
            span_degrees: 8,
            note_beats: 0.5,
            interval_s: (16.0, 30.0),
            natural: None,
            natural_interval_s: (0.0, 0.0),
            bubbles: 0.0,
        },
        Brush::Arcoiris => BrushSpec {
            instrument: Some(Instrument::ViolinHarm),
            articulation: Articulation::Chord,
            attach: Attach::Water,
            lo_degree: 12,
            span_degrees: 5,
            note_beats: 6.0,
            interval_s: (30.0, 50.0),
            natural: None,
            natural_interval_s: (0.0, 0.0),
            bubbles: 0.0,
        },
        // Relieve: el registro grave. Una cresta es un bordón de tuba; una fosa,
        // un cello en la octava baja, aún más espaciado.
        Brush::Elevar => BrushSpec {
            instrument: Some(Instrument::Tuba),
            articulation: Articulation::Sustain,
            attach: Attach::Floor,
            lo_degree: -11,
            span_degrees: 3,
            note_beats: 6.0,
            interval_s: (30.0, 50.0),
            natural: None,
            natural_interval_s: (0.0, 0.0),
            bubbles: 0.0,
        },
        Brush::Hundir => BrushSpec {
            instrument: Some(Instrument::CelloSus),
            articulation: Articulation::Sustain,
            attach: Attach::Floor,
            lo_degree: -9,
            span_degrees: 3,
            note_beats: 8.0,
            interval_s: (35.0, 55.0),
            natural: None,
            natural_interval_s: (0.0, 0.0),
            bubbles: 0.0,
        },
        Brush::Barco => BrushSpec {
            instrument: None,
            articulation: Articulation::Sustain,
            attach: Attach::Floor,
            lo_degree: 0,
            span_degrees: 0,
            note_beats: 0.0,
            interval_s: (0.0, 0.0),
            natural: Some("creak"),
            natural_interval_s: (25.0, 45.0),
            bubbles: 0.0,
        },
    }
}
