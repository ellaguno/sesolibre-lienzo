//! Lienzo — el núcleo. Sin UI, sin motor gráfico.
//!
//! escena (trazos) → motivos → director (eventos) → render (audio)
//!
//! Todo determinista: la misma escena produce siempre los mismos eventos y el
//! mismo audio.

pub mod brush;
pub mod director;
pub mod motif;
pub mod render;
pub mod rng;
pub mod scene;
pub mod theory;
pub mod variation;

pub use director::{compose, Event};
pub use render::{render, write_wav, RenderOptions, SampleBank};
pub use scene::{Brush, Point, Scene, Stroke};
