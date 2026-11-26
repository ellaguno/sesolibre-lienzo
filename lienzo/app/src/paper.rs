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

/// Sobre el papel (f ≈ 1) lo sumergido se oculta: no se dibuja ni se anima.
pub fn hide_submerged(
    paper: Res<Paper>,
    mut last: Local<Option<bool>>,
    mut q: Query<&mut Visibility, With<crate::world::Submerged>>,
) {
    let on_paper = paper.f > 0.985;
    if *last == Some(on_paper) {
        return;
    }
    *last = Some(on_paper);
    for mut v in q.iter_mut() {
        *v = if on_paper {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
}

/// Condición para los sistemas que solo importan bajo el agua.
pub fn underwater(paper: Res<Paper>) -> bool {
    paper.f < 0.985
}

/// Traza de rendimiento (LIENZO_TRACE o Android): fotogramas por segundo
/// cada 5 s, para medir en el teléfono por logcat.
pub fn fps_trace(time: Res<Time>, mut acc: Local<(f32, u32)>) {
    if !(cfg!(target_os = "android") || std::env::var("LIENZO_TRACE").is_ok()) {
        return;
    }
    acc.0 += time.delta_secs();
    acc.1 += 1;
    if acc.0 >= 5.0 {
        info!("fps {:.1}", acc.1 as f32 / acc.0);
        *acc = (0.0, 0);
    }
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
