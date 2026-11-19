//! Render offline: eventos + muestras → audio estéreo. Es la referencia: lo que
//! la app toque en vivo debe sonar igual que esto.

use crate::brush::Instrument;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub const SR: u32 = 48_000;

pub type Stereo = Vec<[f32; 2]>;
/// Muestras almacenadas: 16 bits, la mitad de memoria que f32. Se mezcla en f32.
pub type Pcm = Vec<[i16; 2]>;

pub fn to_pcm(s: &Stereo) -> Pcm {
    s.iter()
        .map(|f| {
            [
                (f[0].clamp(-1.0, 1.0) * 32767.0) as i16,
                (f[1].clamp(-1.0, 1.0) * 32767.0) as i16,
            ]
        })
        .collect()
}

#[inline]
pub fn pcm_f32(s: [i16; 2]) -> [f32; 2] {
    [s[0] as f32 / 32768.0, s[1] as f32 / 32768.0]
}

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("no se pudo leer {0}: {1}")]
    Io(PathBuf, String),
    #[error("faltan muestras para {0:?} en {1}; corre tools/curate_samples.py")]
    MissingInstrument(Instrument, PathBuf),
    #[error("{0} debe ser estéreo a {SR} Hz (es {1} ch, {2} Hz)")]
    Format(PathBuf, u16, u32),
}

/// De dónde salen los archivos del pack: disco, APK, red… Solo hace falta
/// leer bytes por ruta relativa a `assets/audio`; el índice `pack.txt` evita
/// tener que listar carpetas.
pub trait AssetSource: Sync {
    fn read(&self, rel: &str) -> Result<Vec<u8>, String>;
}

/// Carpeta en disco.
pub struct FsSource(pub PathBuf);

impl AssetSource for FsSource {
    fn read(&self, rel: &str) -> Result<Vec<u8>, String> {
        std::fs::read(self.0.join(rel)).map_err(|e| e.to_string())
    }
}

/// Decodifica un Ogg Vorbis (el formato del pack) a estéreo 48 kHz.
fn read_ogg_bytes(path: &Path, bytes: Vec<u8>) -> Result<Stereo, RenderError> {
    let mut r = lewton::inside_ogg::OggStreamReader::new(std::io::Cursor::new(bytes))
        .map_err(|e| RenderError::Io(path.into(), format!("{e:?}")))?;
    let ch = r.ident_hdr.audio_channels as u16;
    let sr = r.ident_hdr.audio_sample_rate;
    if ch != 2 || sr != SR {
        return Err(RenderError::Format(path.into(), ch, sr));
    }
    let mut out: Stereo = Vec::new();
    while let Some(pkt) = r
        .read_dec_packet_itl()
        .map_err(|e| RenderError::Io(path.into(), format!("{e:?}")))?
    {
        out.extend(
            pkt.chunks_exact(2)
                .map(|c| [c[0] as f32 / 32768.0, c[1] as f32 / 32768.0]),
        );
    }
    Ok(out)
}

/// Lee `.ogg` o `.wav` según la extensión, desde bytes ya cargados.
pub fn decode(path: &Path, bytes: Vec<u8>) -> Result<Stereo, RenderError> {
    match path.extension().and_then(|e| e.to_str()) {
        Some("ogg") => read_ogg_bytes(path, bytes),
        _ => read_wav_bytes(path, bytes),
    }
}

/// Lee un archivo de disco (`.ogg` o `.wav`).
pub fn read_audio(path: &Path) -> Result<Stereo, RenderError> {
    let bytes = std::fs::read(path).map_err(|e| RenderError::Io(path.into(), e.to_string()))?;
    decode(path, bytes)
}

fn read_wav_bytes(path: &Path, bytes: Vec<u8>) -> Result<Stereo, RenderError> {
    let mut r = hound::WavReader::new(std::io::Cursor::new(bytes))
        .map_err(|e| RenderError::Io(path.into(), e.to_string()))?;
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
    instruments: HashMap<Instrument, Vec<(i32, Pcm)>>,
    ambience: HashMap<String, Pcm>,
    events: HashMap<String, Vec<Pcm>>,
}

impl SampleBank {
    /// `dir` es `assets/audio` en disco.
    pub fn load(dir: &Path) -> Result<Self, RenderError> {
        Self::load_from(&FsSource(dir.to_path_buf()))
    }

    /// Carga el pack desde cualquier fuente, guiado por `pack.txt`.
    pub fn load_from(src: &dyn AssetSource) -> Result<Self, RenderError> {
        let index = src
            .read("pack.txt")
            .map_err(|e| RenderError::Io("pack.txt".into(), e))?;
        let index = String::from_utf8_lossy(&index);
        let paths: Vec<&str> = index
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .collect();

        // Se decodifica en paralelo: ~90 archivos Vorbis y en un teléfono el
        // arranque se nota.
        let decoded: Vec<Result<(String, Pcm), RenderError>> = std::thread::scope(|sc| {
            let workers = 8;
            let chunks: Vec<Vec<&str>> = (0..workers)
                .map(|w| paths.iter().copied().skip(w).step_by(workers).collect())
                .collect();
            let handles: Vec<_> = chunks
                .into_iter()
                .map(|chunk| {
                    sc.spawn(move || {
                        chunk
                            .into_iter()
                            .map(|rel| {
                                let bytes =
                                    src.read(rel).map_err(|e| RenderError::Io(rel.into(), e))?;
                                Ok((rel.to_string(), to_pcm(&decode(Path::new(rel), bytes)?)))
                            })
                            .collect::<Vec<_>>()
                    })
                })
                .collect();
            handles
                .into_iter()
                .flat_map(|h| h.join().expect("hilo de decodificación"))
                .collect()
        });

        let mut instruments: HashMap<Instrument, Vec<(i32, Pcm)>> = HashMap::new();
        let mut ambience = HashMap::new();
        let mut events: HashMap<String, Vec<Pcm>> = HashMap::new();
        let mut decoded: Vec<(String, Pcm)> = decoded.into_iter().collect::<Result<_, _>>()?;
        decoded.sort_by(|a, b| a.0.cmp(&b.0));
        for (rel, audio) in decoded {
            let p = Path::new(&rel);
            let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            let mut parts = rel.split('/');
            match parts.next() {
                Some("instruments") => {
                    let dir = parts.next().unwrap_or("");
                    if let (Some(inst), Ok(midi)) = (
                        Instrument::ALL.iter().find(|i| i.dir() == dir),
                        stem.parse::<i32>(),
                    ) {
                        instruments.entry(*inst).or_default().push((midi, audio));
                    }
                }
                Some("ambience") => {
                    ambience.insert(stem.to_string(), audio);
                }
                Some("events") => {
                    if let Some((name, _)) = stem.rsplit_once('_') {
                        events.entry(name.to_string()).or_default().push(audio);
                    }
                }
                _ => {}
            }
        }
        for inst in Instrument::ALL {
            match instruments.get_mut(&inst) {
                Some(notes) if !notes.is_empty() => notes.sort_by_key(|(m, _)| *m),
                _ => {
                    return Err(RenderError::MissingInstrument(
                        inst,
                        PathBuf::from("instruments").join(inst.dir()),
                    ))
                }
            }
        }
        Ok(SampleBank {
            instruments,
            ambience,
            events,
        })
    }

    pub fn ambience(&self, name: &str) -> Option<&Pcm> {
        self.ambience.get(name)
    }

    pub fn event_clips(&self, name: &str) -> Option<&Vec<Pcm>> {
        self.events.get(name)
    }

    pub fn nearest(&self, inst: Instrument, midi: i32) -> (i32, &Pcm) {
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
pub fn pitch(src: &Pcm, semis: f32) -> Stereo {
    if semis.abs() < 1e-4 {
        return src.iter().map(|s| pcm_f32(*s)).collect();
    }
    let ratio = 2f32.powf(semis / 12.0);
    let n = (src.len() as f32 / ratio) as usize;
    (0..n)
        .map(|i| {
            let pos = i as f32 * ratio;
            let j = pos as usize;
            let f = pos - j as f32;
            let a = pcm_f32(src[j.min(src.len() - 1)]);
            let b = pcm_f32(src[(j + 1).min(src.len() - 1)]);
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
        let src: Pcm = (0..1000).map(|i| [i as i16, 0]).collect();
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
