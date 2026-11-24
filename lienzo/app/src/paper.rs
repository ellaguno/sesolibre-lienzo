//! El papel: en la vista de dibujo el mundo es una hoja clara, sin niebla,
//! sin cáusticas ni rayos, con el borde del lienzo visible. Al bucear o
//! contemplar, el papel se disuelve en mar. `Paper.f` va de 1 (hoja) a 0
//! (agua) y `water::apply` tiñe el mundo según ese factor.

use crate::draw::{DrawState, View};
use bevy::prelude::*;

/// Color del papel (lino cálido).
pub const PAPER: [f32; 3] = [0.93, 0.90, 0.83];

#[derive(Resource, Default)]
pub struct Paper {
    pub f: f32,
}

pub fn ease(time: Res<Time>, st: Res<DrawState>, mut paper: ResMut<Paper>) {
    let target = if st.view == View::Dibujo && !st.contemplate {
        1.0
    } else {
        0.0
    };
    // volver al papel es rápido; disolverse en el mar, un viaje lento
    let rate = if target > 0.5 { 2.5 } else { 0.7 };
    let k = 1.0 - (-rate * time.delta_secs()).exp();
    let f = paper.f + (target - paper.f) * k;
    paper.f = if (f - target).abs() < 0.002 { target } else { f };
}

/// Mezcla lineal de dos colores RGB.
pub fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}
