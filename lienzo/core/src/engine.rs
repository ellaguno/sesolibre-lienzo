//! El motor en tiempo real. Mezcla por bloques los eventos del director con las
//! muestras del banco. Es EL renderer: el render offline lo usa de principio a
//! fin, y la app lo alimenta a la tarjeta de sonido bloque a bloque. Lo que se
//! oye en vivo y lo que se exporta es el mismo código.

use crate::brush::spec;
use crate::director::{compose, Event};
use crate::motif;
use crate::render::{
    db, depth_cutoff, envelope, pcm_f32, pitch, to_pcm, Pcm, SampleBank, Stereo, SR,
};
use crate::rng::Rng;
use crate::scene::Scene;
use crate::theory::tonality;
use std::sync::Arc;

/// Una nota (o sonido) en curso: su buffer ya remuestreado, envuelto y filtrado.
struct Voice {
    buf: Arc<Pcm>,
    start: usize,
    gl: f32,
    gr: f32,
}

struct OnePole {
    coef: f32,
    l: f32,
    r: f32,
}

impl OnePole {
    fn new(cutoff_hz: f32) -> Self {
        let x = (-2.0 * std::f32::consts::PI * cutoff_hz / SR as f32).exp();
        OnePole {
            coef: 1.0 - x,
            l: 0.0,
            r: 0.0,
        }
    }

    #[inline]
    fn tick(&mut self, s: [f32; 2]) -> [f32; 2] {
        self.l += self.coef * (s[0] - self.l);
        self.r += self.coef * (s[1] - self.r);
        [self.l, self.r]
    }
}

struct Comb {
    buf: Vec<f32>,
    i: usize,
    lp: f32,
    fb: f32,
}

impl Comb {
    #[inline]
    fn tick(&mut self, x: f32) -> f32 {
        let y = self.buf[self.i];
        self.lp += 0.35 * (y - self.lp);
        self.buf[self.i] = x + self.lp * self.fb;
        self.i = (self.i + 1) % self.buf.len();
        y
    }
}

struct AllPass {
    buf: Vec<f32>,
    i: usize,
}

impl AllPass {
    #[inline]
    fn tick(&mut self, x: f32) -> f32 {
        let y = self.buf[self.i];
        self.buf[self.i] = x + y * 0.5;
        self.i = (self.i + 1) % self.buf.len();
        y - x * 0.5
    }
}

/// Reverb de Schroeder con estado, un canal.
struct ReverbChannel {
    combs: Vec<Comb>,
    allpass: Vec<AllPass>,
}

impl ReverbChannel {
    fn new(comb_delays: &[usize], seconds: f32) -> Self {
        let fb = 0.84f32.powf(1.0 / (seconds / 2.0).max(0.2));
        let scale = |d: usize| d * SR as usize / 44100;
        ReverbChannel {
            combs: comb_delays
                .iter()
                .map(|&d| Comb {
                    buf: vec![0.0; scale(d)],
                    i: 0,
                    lp: 0.0,
                    fb,
                })
                .collect(),
            allpass: [225usize, 556]
                .iter()
                .map(|&d| AllPass {
                    buf: vec![0.0; scale(d)],
                    i: 0,
                })
                .collect(),
        }
    }

    #[inline]
    fn tick(&mut self, x: f32) -> f32 {
        let mut acc = 0.0;
        for c in &mut self.combs {
            acc += c.tick(x);
        }
        for a in &mut self.allpass {
            acc = a.tick(acc);
        }
        acc * 0.25
    }
}

#[derive(Clone, Debug)]
pub struct EngineOptions {
    pub wet: f32,
    pub master_lowpass_hz: f32,
    /// Ganancia de salida antes del limitador. El render offline normaliza
    /// después; en vivo esto es lo que hay.
    pub output_db: f32,
}

impl Default for EngineOptions {
    fn default() -> Self {
        EngineOptions {
            wet: 0.45,
            master_lowpass_hz: 7000.0,
            output_db: 0.0,
        }
    }
}

/// Un ambiente en bucle: cuándo toca programar la siguiente pasada.
struct AmbienceLoop {
    name: &'static str,
    pieces: Vec<(Arc<Pcm>, f32, f32)>, // (buffer ya remuestreado, ganancia, paneo)
    next_t: Vec<f64>,
    gain: f32,
}

pub struct Engine {
    bank: Arc<SampleBank>,
    opts: EngineOptions,
    events: Vec<Event>,
    next_event: usize,
    voices: Vec<Voice>,
    ambiences: Vec<AmbienceLoop>,
    /// Muestras transcurridas desde el inicio.
    pub clock: usize,
    master_lp: OnePole,
    rev_l: ReverbChannel,
    rev_r: ReverbChannel,
    rng: Rng,
    seed: u64,
    /// Hasta qué segundo hay eventos compuestos.
    horizon: f64,
    scene: Scene,
    /// Eventos inmediatos (respuesta al gesto), fuera del plan del director.
    pending: Vec<Event>,
}

impl Engine {
    pub fn new(bank: Arc<SampleBank>, scene: Scene, opts: EngineOptions) -> Self {
        let mut e = Engine {
            bank,
            opts: opts.clone(),
            events: vec![],
            next_event: 0,
            voices: vec![],
            ambiences: vec![],
            clock: 0,
            master_lp: OnePole::new(opts.master_lowpass_hz),
            rev_l: ReverbChannel::new(&[1557, 1617, 1491, 1422], 4.0),
            rev_r: ReverbChannel::new(&[1601, 1667, 1523, 1456], 4.0),
            rng: Rng::from_parts(&[scene.seed, 0x7265_6e64]),
            seed: scene.seed,
            horizon: 0.0,
            scene: scene.clone(),
            pending: vec![],
        };
        e.set_scene(scene);
        e
    }

    pub fn now_seconds(&self) -> f64 {
        self.clock as f64 / SR as f64
    }

    pub fn scene(&self) -> &Scene {
        &self.scene
    }

    /// Cambia la obra en caliente. Lo que ya sonó se queda; lo que viene se
    /// recompone desde ahora. Determinista: misma escena + mismo instante de
    /// cambio = mismo resultado.
    pub fn set_scene(&mut self, scene: Scene) {
        self.opts.wet = tonality(scene.water).wet;
        self.scene = scene;
        self.rng = Rng::from_parts(&[self.seed, 0x7265_6e64, self.clock as u64]);
        self.horizon = 0.0;
        self.events.clear();
        self.next_event = 0;
        let now = self.now_seconds();
        self.extend(now + 120.0);
        // Ambiente: bucles que se construyen UNA vez (remuestrear 60 s de agua
        // pesa, y esto corre en el hilo de audio). Al cambiar la obra solo
        // cambia su ganancia; el agua no se reinicia con cada trazo.
        let all = compose(&self.scene, 1.0);
        for ev in all {
            if let Event::Ambience { name, gain } = ev {
                if let Some(existing) = self.ambiences.iter_mut().find(|a| a.name == name) {
                    existing.gain = gain;
                    continue;
                }
                if let Some(src) = self.bank.ambience(name) {
                    let mut pieces = vec![];
                    let mut next_t = vec![];
                    for (semis, g, pan, offset) in [(0.0, 1.0, -0.2, 0.0), (-3.0, 0.7, 0.3, 7.0)] {
                        let s = envelope(pitch(src, semis), 3.0, None, 3.0);
                        pieces.push((Arc::new(to_pcm(&s)), g * db(-6.0), pan));
                        next_t.push(now + offset);
                    }
                    self.ambiences.push(AmbienceLoop {
                        name,
                        pieces,
                        next_t,
                        gain,
                    });
                }
            }
        }
    }

    /// Compone eventos hasta `until` segundos. El director es determinista y
    /// barato; se recompone entero y se conservan solo los futuros.
    fn extend(&mut self, until: f64) {
        if until <= self.horizon {
            return;
        }
        let now = self.now_seconds();
        let all = compose(&self.scene, until);
        let mut future: Vec<Event> = all
            .into_iter()
            .filter(|e| !matches!(e, Event::Ambience { .. }) && e.time() >= now.max(self.horizon))
            .collect();
        future.sort_by(|a, b| a.time().partial_cmp(&b.time()).unwrap());
        self.events.extend(future);
        self.horizon = until;
    }

    fn start_event(&mut self, ev: &Event) {
        let (buf, t, gain, pan) = match ev {
            Event::Note {
                t,
                instrument,
                midi,
                vel,
                dur,
                pan,
                depth,
                ..
            } => {
                let (near, src) = self.bank.nearest(*instrument, *midi);
                let (attack, release) = instrument.envelope();
                let mut buf = pitch(src, (*midi - near) as f32);
                let hold = if release > 1.0 && attack < 0.1 {
                    None
                } else {
                    Some(*dur)
                };
                buf = envelope(buf, attack, hold, release);
                let mut lp = OnePole::new(depth_cutoff(*depth));
                for s in buf.iter_mut() {
                    *s = lp.tick(*s);
                }
                let g =
                    vel * db(instrument.gain_db()) * db(-6.0) * (1.0 - 0.25 * (-*depth).max(0.0));
                (buf, *t, g, *pan)
            }
            Event::Natural {
                t,
                name,
                gain,
                pan,
                depth,
                ..
            } => {
                let Some(clips) = self.bank.event_clips(name) else {
                    return;
                };
                let clip = &clips[self.rng.below(clips.len())];
                let mut buf: Stereo = clip.iter().map(|s| pcm_f32(*s)).collect();
                let mut lp = OnePole::new(depth_cutoff(*depth).min(2500.0));
                for s in buf.iter_mut() {
                    *s = lp.tick(*s);
                }
                (buf, *t, gain * db(-4.0), *pan)
            }
            Event::Ambience { .. } => return,
        };
        self.push_voice(Arc::new(to_pcm(&buf)), t, gain, pan);
    }

    fn push_voice(&mut self, buf: Arc<Pcm>, t: f64, gain: f32, pan: f32) {
        let theta = (pan.clamp(-1.0, 1.0) + 1.0) * std::f32::consts::FRAC_PI_4;
        self.voices.push(Voice {
            buf,
            start: (t * SR as f64) as usize,
            gl: gain * theta.cos(),
            gr: gain * theta.sin(),
        });
    }

    /// Mezcla el siguiente bloque. Llamar desde el hilo de audio o en bucle
    /// para el render offline.
    pub fn process(&mut self, out: &mut [[f32; 2]]) {
        let n = out.len();
        let block_end = self.clock + n;
        let end_s = block_end as f64 / SR as f64;

        if end_s + 30.0 > self.horizon {
            self.extend(self.horizon + 120.0);
        }
        if !self.pending.is_empty() {
            let pend = std::mem::take(&mut self.pending);
            for ev in pend {
                if ev.time() < end_s {
                    self.start_event(&ev);
                } else {
                    self.pending.push(ev);
                }
            }
        }
        while self.next_event < self.events.len() && self.events[self.next_event].time() < end_s {
            let ev = self.events[self.next_event].clone();
            self.next_event += 1;
            self.start_event(&ev);
        }
        for a in 0..self.ambiences.len() {
            for p in 0..self.ambiences[a].pieces.len() {
                while self.ambiences[a].next_t[p] < end_s {
                    let (buf, g, pan) = {
                        let amb = &self.ambiences[a];
                        let (b, g, pan) = &amb.pieces[p];
                        (b.clone(), *g * amb.gain, *pan)
                    };
                    let t = self.ambiences[a].next_t[p];
                    let len_s = (buf.len() - 3 * SR as usize) as f64 / SR as f64;
                    self.push_voice(buf, t, g, pan);
                    self.ambiences[a].next_t[p] = t + len_s;
                }
            }
        }

        for s in out.iter_mut() {
            *s = [0.0, 0.0];
        }
        let clock = self.clock;
        self.voices.retain_mut(|v| {
            if v.start >= block_end {
                return true;
            }
            let from = v.start.max(clock);
            let off = from - v.start;
            let avail = v.buf.len().saturating_sub(off);
            let count = avail.min(block_end - from);
            let o = from - clock;
            for k in 0..count {
                let s = pcm_f32(v.buf[off + k]);
                out[o + k][0] += s[0] * v.gl;
                out[o + k][1] += s[1] * v.gr;
            }
            off + count < v.buf.len()
        });

        let og = db(self.opts.output_db);
        for s in out.iter_mut() {
            let d = self.master_lp.tick(*s);
            let wl = self.rev_l.tick(d[0]) * self.opts.wet;
            let wr = self.rev_r.tick(d[1]) * self.opts.wet;
            let l = (d[0] + wl) * 1.4 * og;
            let r = (d[1] + wr) * 1.4 * og;
            *s = [l.tanh() / 1.4, r.tanh() / 1.4];
        }
        self.clock = block_end;
    }

    /// Toca de inmediato el motivo de un trazo, tal cual, sin variación. Es la
    /// respuesta al gesto: desde el primer trazo suena algo nuevo.
    pub fn play_stroke_now(&mut self, index: usize) {
        let Some(stroke) = self.scene.strokes.get(index) else {
            return;
        };
        let sp = spec(stroke.brush);
        let now = self.now_seconds() + 0.05;
        let ton = tonality(self.scene.water);
        let beat = ton.beat_seconds();
        let c = stroke.centroid();
        if let (Some(inst), Some(m)) = (
            sp.instrument,
            motif::from_stroke(stroke, self.scene.seed, index),
        ) {
            for n in &m.notes {
                let ev = Event::Note {
                    t: now + n.at * beat,
                    instrument: inst,
                    midi: ton.degree_to_midi(n.degree),
                    vel: n.vel,
                    dur: n.beats * beat,
                    pan: c.x.clamp(-1.0, 1.0) * 0.8,
                    depth: c.z.clamp(-1.0, 1.0),
                    stroke: Some(index),
                };
                self.pending.push(ev);
            }
        }
        if let Some(name) = sp.natural {
            self.pending.push(Event::Natural {
                t: now,
                name,
                gain: 0.9,
                pan: c.x * 0.8,
                depth: c.z,
                stroke: index,
            });
        }
    }

    pub fn active_voices(&self) -> usize {
        self.voices.len()
    }
}

/// Render offline completo: el motor de principio a fin, normalizado a -1 dB
/// y con fundido final.
pub fn render_offline(
    bank: Arc<SampleBank>,
    scene: &Scene,
    seconds: f64,
    opts: EngineOptions,
) -> Stereo {
    let mut e = Engine::new(bank, scene.clone(), opts);
    let n = (seconds * SR as f64) as usize;
    let mut out = vec![[0.0f32; 2]; n];
    let block = 1024;
    let mut i = 0;
    while i < n {
        let end = (i + block).min(n);
        e.process(&mut out[i..end]);
        i = end;
    }
    let peak = out
        .iter()
        .fold(0.0f32, |m, s| m.max(s[0].abs()).max(s[1].abs()));
    let norm = db(-1.0) / peak.max(1e-6);
    let fade = (3.0 * SR as f64) as usize;
    for (i, s) in out.iter_mut().enumerate() {
        let g = norm
            * if i + fade > n {
                (n - i) as f32 / fade as f32
            } else {
                1.0
            };
        s[0] *= g;
        s[1] *= g;
    }
    out
}
