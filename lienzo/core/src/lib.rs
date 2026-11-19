//! Lienzo — el núcleo. Sin UI, sin motor gráfico.
//!
//! escena (trazos) → motivos → director (eventos) → render (audio)
//!
//! Todo determinista: la misma escena produce siempre los mismos eventos y el
//! mismo audio.

pub mod brush;
pub mod director;
pub mod engine;
pub mod motif;
pub mod render;
pub mod rng;
pub mod scene;
pub mod theory;
pub mod variation;

pub use director::{compose, Event};
pub use engine::{render_offline, Engine, EngineOptions};
pub use render::{write_wav, AssetSource, FsSource, SampleBank, SR};
pub use scene::{Brush, Point, Scene, Stroke, Water};
pub use theory::{tonality, Tonality};
