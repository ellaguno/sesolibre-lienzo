//! Render offline: eventos + muestras → audio estéreo. Es la referencia: lo que
//! la app toque en vivo debe sonar igual que esto.

use crate::brush::Instrument;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub const SR: u32 = 48_000;

pub type Stereo = Vec<[f32; 2]>;

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("no se pudo leer {0}: {1}")]
    Io(PathBuf, String),
    #[error("faltan muestras para {0:?} en {1}; corre tools/curate_samples.py")]
    MissingInstrument(Instrument, PathBuf),
    #[error("{0} debe ser estéreo a {SR} Hz (es {1} ch, {2} Hz)")]
    Format(PathBuf, u16, u32),
}

fn read_wav(path: &Path) -> Result<Stereo, RenderError> {
    let mut r =
        hound::WavReader::open(path).map_err(|e| RenderError::Io(path.into(), e.to_string()))?;
    let spec = r.spec();
    if spec.channels != 2 || spec.sample_rate != SR {
        return Err(RenderError::Format(
            path.into(),
            spec.channels,
            spec.sample_rate,
        ));
    }
    let scale = 1.0 / (1u32 << (spec.bits_per_sample - 1)) as f32;
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Int => r
            .samples::<i32>()
            .map(|s| s.unwrap_or(0) as f32 * scale)
            .collect(),
        hound::SampleFormat::Float => r.samples::<f32>().map(|s| s.unwrap_or(0.0)).collect(),
    };
    Ok(samples.chunks_exact(2).map(|c| [c[0], c[1]]).collect())
}

/// Las muestras curadas, en memoria.
pub struct SampleBank {
    instruments: HashMap<Instrument, Vec<(i32, Stereo)>>,
    ambience: HashMap<String, Stereo>,
    events: HashMap<String, Vec<Stereo>>,
}

impl SampleBank {
    /// `dir` es `assets/audio`.
    pub fn load(dir: &Path) -> Result<Self, RenderError> {
        let mut instruments = HashMap::new();
        for inst in Instrument::ALL {
            let d = dir.join("instruments").join(inst.dir());
            let mut notes = vec![];
            if let Ok(rd) = std::fs::read_dir(&d) {
                for e in rd.flatten() {
                    let p = e.path();
                    if let Some(midi) = p
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .and_then(|s| s.parse::<i32>().ok())
                    {
                        notes.push((midi, read_wav(&p)?));
                    }
                }
            }
            if notes.is_empty() {
                return Err(RenderError::MissingInstrument(inst, d));
            }
            notes.sort_by_key(|(m, _)| *m);
            instruments.insert(inst, notes);
        }
        let mut ambience = HashMap::new();
        if let Ok(rd) = std::fs::read_dir(dir.join("ambience")) {
            for e in rd.flatten() {
                let p = e.path();
                if p.extension().map(|x| x == "wav").unwrap_or(false) {
                    ambience.insert(
                        p.file_stem().unwrap().to_string_lossy().into_owned(),
                        read_wav(&p)?,
                    );
                }
            }
        }
        let mut events: HashMap<String, Vec<Stereo>> = HashMap::new();
        if let Ok(rd) = std::fs::read_dir(dir.join("events")) {
            let mut paths: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
            paths.sort();
            for p in paths {
                if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                    if let Some((name, _)) = stem.rsplit_once('_') {
                        events
                            .entry(name.to_string())
                            .or_default()
                            .push(read_wav(&p)?);
                    }
                }
            }
        }
        Ok(SampleBank {
            instruments,
            ambience,
            events,
        })
    }

    pub fn ambience(&self, name: &str) -> Option<&Stereo> {
        self.ambience.get(name)
    }

    pub fn event_clips(&self, name: &str) -> Option<&Vec<Stereo>> {
        self.events.get(name)
    }

    pub fn nearest(&self, inst: Instrument, midi: i32) -> (i32, &Stereo) {
        let notes = &self.instruments[&inst];
        let (m, s) = notes.iter().min_by_key(|(m, _)| (m - midi).abs()).unwrap();
        (*m, s)
    }
}

// ---- DSP mínimo ----

pub fn db(x: f32) -> f32 {
    10f32.powf(x / 20.0)
}

/// Remuestreo lineal por semitonos. Cambia la duración, como una cinta.
pub fn pitch(src: &Stereo, semis: f32) -> Stereo {
    if semis.abs() < 1e-4 {
        return src.clone();
    }
    let ratio = 2f32.powf(semis / 12.0);
    let n = (src.len() as f32 / ratio) as usize;
    (0..n)
        .map(|i| {
            let pos = i as f32 * ratio;
            let j = pos as usize;
            let f = pos - j as f32;
            let a = src[j.min(src.len() - 1)];
            let b = src[(j + 1).min(src.len() - 1)];
            [a[0] + (b[0] - a[0]) * f, a[1] + (b[1] - a[1]) * f]
        })
        .collect()
}

/// Envolvente: ataque, sostén hasta `hold`, cola `release`. Recorta el buffer.
pub fn envelope(mut buf: Stereo, attack: f64, hold: Option<f64>, release: f64) -> Stereo {
    let sr = SR as f64;
    if let Some(h) = hold {
        let n = ((h + release) * sr) as usize;
        buf.truncate(n.max(1));
    }
    let n = buf.len();
    let a = ((attack * sr) as usize).min(n);
    for i in 0..a {
        let g = i as f32 / a.max(1) as f32;
        buf[i][0] *= g;
        buf[i][1] *= g;
    }
    let r = ((release * sr) as usize).min(n);
    for i in 0..r {
        let g = 1.0 - i as f32 / r.max(1) as f32;
        let k = n - r + i;
        buf[k][0] *= g;
        buf[k][1] *= g;
    }
    buf
}

/// Frecuencia de corte según profundidad: lejos (-1) = 1.2 kHz · cerca (1) = 9 kHz.
pub fn depth_cutoff(depth: f32) -> f32 {
    let u = (depth + 1.0) / 2.0;
    1200.0 * (7.5f32).powf(u)
}

pub fn write_wav(path: &Path, audio: &Stereo) -> Result<(), RenderError> {
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: SR,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut w = hound::WavWriter::create(path, spec)
        .map_err(|e| RenderError::Io(path.into(), e.to_string()))?;
    for s in audio {
        for c in s {
            w.write_sample((c.clamp(-1.0, 1.0) * 32767.0) as i16)
                .map_err(|e| RenderError::Io(path.into(), e.to_string()))?;
        }
    }
    w.finalize()
        .map_err(|e| RenderError::Io(path.into(), e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pitch_changes_length() {
        let src: Stereo = (0..1000).map(|i| [i as f32, 0.0]).collect();
        assert_eq!(pitch(&src, 12.0).len(), 500);
        assert_eq!(pitch(&src, 0.0).len(), 1000);
    }

    #[test]
    fn envelope_truncates_and_fades() {
        let src: Stereo = vec![[1.0, 1.0]; SR as usize * 4];
        let e = envelope(src, 0.1, Some(1.0), 0.5);
        assert_eq!(e.len(), (1.5 * SR as f64) as usize);
        assert!(e[0][0] < 0.01);
        assert!((e[e.len() - 1][0]).abs() < 0.01);
        assert!((e[SR as usize][0] - 1.0).abs() < 1e-5);
    }
}
