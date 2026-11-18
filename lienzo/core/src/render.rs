//! Render offline: eventos + muestras → audio estéreo. Es la referencia: lo que
//! la app toque en vivo debe sonar igual que esto.

use crate::brush::Instrument;
use crate::director::Event;
use crate::rng::Rng;
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
    let mut r = hound::WavReader::open(path).map_err(|e| RenderError::Io(path.into(), e.to_string()))?;
    let spec = r.spec();
    if spec.channels != 2 || spec.sample_rate != SR {
        return Err(RenderError::Format(path.into(), spec.channels, spec.sample_rate));
    }
    let scale = 1.0 / (1u32 << (spec.bits_per_sample - 1)) as f32;
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Int => r.samples::<i32>().map(|s| s.unwrap_or(0) as f32 * scale).collect(),
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
                    if let Some(midi) = p.file_stem().and_then(|s| s.to_str()).and_then(|s| s.parse::<i32>().ok()) {
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
                    ambience.insert(p.file_stem().unwrap().to_string_lossy().into_owned(), read_wav(&p)?);
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
                        events.entry(name.to_string()).or_default().push(read_wav(&p)?);
                    }
                }
            }
        }
        Ok(SampleBank { instruments, ambience, events })
    }

    fn nearest(&self, inst: Instrument, midi: i32) -> (i32, &Stereo) {
        let notes = &self.instruments[&inst];
        let (m, s) = notes.iter().min_by_key(|(m, _)| (m - midi).abs()).unwrap();
        (*m, s)
    }
}

// ---- DSP mínimo ----

fn db(x: f32) -> f32 {
    10f32.powf(x / 20.0)
}

/// Remuestreo lineal por semitonos. Cambia la duración, como una cinta.
fn pitch(src: &Stereo, semis: f32) -> Stereo {
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
fn envelope(mut buf: Stereo, attack: f64, hold: Option<f64>, release: f64) -> Stereo {
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

/// Paso-bajo de un polo. La profundidad oscurece.
fn onepole(buf: &mut Stereo, cutoff_hz: f32) {
    let x = (-2.0 * std::f32::consts::PI * cutoff_hz / SR as f32).exp();
    let a = 1.0 - x;
    let mut l = 0.0;
    let mut r = 0.0;
    for s in buf.iter_mut() {
        l += a * (s[0] - l);
        r += a * (s[1] - r);
        *s = [l, r];
    }
}

fn depth_cutoff(depth: f32) -> f32 {
    // lejos (-1) = 1.2 kHz · cerca (1) = 9 kHz
    let u = (depth + 1.0) / 2.0;
    1200.0 * (7.5f32).powf(u)
}

fn mix_into(out: &mut Stereo, src: &Stereo, at: f64, gain: f32, pan: f32) {
    let i0 = (at * SR as f64) as usize;
    let theta = (pan.clamp(-1.0, 1.0) + 1.0) * std::f32::consts::FRAC_PI_4;
    let (gl, gr) = (gain * theta.cos(), gain * theta.sin());
    for (k, s) in src.iter().enumerate() {
        let i = i0 + k;
        if i >= out.len() {
            break;
        }
        out[i][0] += s[0] * gl;
        out[i][1] += s[1] * gr;
    }
}

/// Reverb de Schroeder: cuatro peines y dos paso-todo por canal, con tiempos
/// distintos por lado para que abra. Barata y suficiente para la referencia.
fn reverb(dry: &Stereo, wet_gain: f32, seconds: f32) -> Stereo {
    let combs_l = [1557usize, 1617, 1491, 1422];
    let combs_r = [1601usize, 1667, 1523, 1456];
    let fb = 0.84f32.powf(1.0 / (seconds / 2.0).max(0.2));
    let mut out = vec![[0.0f32; 2]; dry.len()];
    for (ch, combs) in [(0usize, combs_l), (1, combs_r)] {
        let mut acc = vec![0.0f32; dry.len()];
        for &d in &combs {
            let d = d * (SR as usize) / 44100;
            let mut buf = vec![0.0f32; d];
            let mut lp = 0.0f32;
            for i in 0..dry.len() {
                let y = buf[i % d];
                lp += 0.35 * (y - lp);
                buf[i % d] = dry[i][ch] + lp * fb;
                acc[i] += y;
            }
        }
        for &d in &[225usize, 556] {
            let d = d * (SR as usize) / 44100;
            let mut buf = vec![0.0f32; d];
            for i in 0..acc.len() {
                let y = buf[i % d];
                let x = acc[i];
                buf[i % d] = x + y * 0.5;
                acc[i] = y - x * 0.5;
            }
        }
        for i in 0..dry.len() {
            out[i][ch] = acc[i] * wet_gain * 0.25;
        }
    }
    out
}

pub struct RenderOptions {
    pub seconds: f64,
    pub wet: f32,
    pub master_lowpass_hz: f32,
}

impl Default for RenderOptions {
    fn default() -> Self {
        RenderOptions { seconds: 180.0, wet: 0.45, master_lowpass_hz: 7000.0 }
    }
}

pub fn render(events: &[Event], bank: &SampleBank, opts: &RenderOptions, seed: u64) -> Stereo {
    let n = (opts.seconds * SR as f64) as usize;
    let mut out: Stereo = vec![[0.0; 2]; n];
    let mut rng = Rng::from_parts(&[seed, 0x7265_6e64]);

    for ev in events {
        match ev {
            Event::Ambience { name, gain } => {
                if let Some(src) = bank.ambience.get(*name) {
                    // dos copias desfasadas a velocidades distintas, en bucle con fundido
                    for (semis, g, pan, offset) in [(0.0, 1.0, -0.2, 0.0), (-3.0, 0.7, 0.3, 7.0)] {
                        let s = pitch(src, semis);
                        let fade = (3.0 * SR as f64) as usize;
                        let mut t = offset;
                        while (t * SR as f64) < n as f64 {
                            let piece = envelope(s.clone(), 3.0, None, 3.0);
                            mix_into(&mut out, &piece, t, gain * g * db(-6.0), pan);
                            t += (s.len() - fade) as f64 / SR as f64;
                        }
                    }
                }
            }
            Event::Note { t, instrument, midi, vel, dur, pan, depth, .. } => {
                let (near, src) = bank.nearest(*instrument, *midi);
                let (attack, release) = instrument.envelope();
                let mut buf = pitch(src, (*midi - near) as f32);
                let hold = if release > 1.0 && attack < 0.1 { None } else { Some(*dur) };
                buf = envelope(buf, attack, hold, release);
                onepole(&mut buf, depth_cutoff(*depth));
                let g = vel * db(instrument.gain_db()) * db(-6.0) * (1.0 - 0.25 * (-*depth).max(0.0));
                mix_into(&mut out, &buf, *t, g, *pan);
            }
            Event::Natural { t, name, gain, pan, depth, .. } => {
                if let Some(clips) = bank.events.get(*name) {
                    let clip = &clips[rng.below(clips.len())];
                    let mut buf = clip.clone();
                    onepole(&mut buf, depth_cutoff(*depth).min(2500.0));
                    mix_into(&mut out, &buf, *t, gain * db(-4.0), *pan);
                }
            }
        }
    }

    onepole(&mut out, opts.master_lowpass_hz);
    let wet = reverb(&out, opts.wet, 4.0);
    let mut peak = 0.0f32;
    for i in 0..n {
        let l = (out[i][0] + wet[i][0]) * 1.4;
        let r = (out[i][1] + wet[i][1]) * 1.4;
        out[i] = [l.tanh() / 1.4, r.tanh() / 1.4];
        peak = peak.max(out[i][0].abs()).max(out[i][1].abs());
    }
    let norm = db(-1.0) / peak.max(1e-6);
    let fade = (3.0 * SR as f64) as usize;
    for i in 0..n {
        let g = norm * if i + fade > n { (n - i) as f32 / fade as f32 } else { 1.0 };
        out[i][0] *= g;
        out[i][1] *= g;
    }
    out
}

pub fn write_wav(path: &Path, audio: &Stereo) -> Result<(), RenderError> {
    let spec = hound::WavSpec { channels: 2, sample_rate: SR, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
    let mut w = hound::WavWriter::create(path, spec).map_err(|e| RenderError::Io(path.into(), e.to_string()))?;
    for s in audio {
        for c in s {
            w.write_sample((c.clamp(-1.0, 1.0) * 32767.0) as i16)
                .map_err(|e| RenderError::Io(path.into(), e.to_string()))?;
        }
    }
    w.finalize().map_err(|e| RenderError::Io(path.into(), e.to_string()))
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
