//! Salida de audio: el `Engine` del core corre en el hilo de la tarjeta de
//! sonido. La app no mezcla nada; solo le manda escenas.

use bevy::prelude::*;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use lienzo_core::{Engine, EngineOptions, SampleBank, Scene, SR};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};

#[derive(Resource)]
pub struct AudioLink {
    tx: Sender<Scene>,
    engine: Arc<Mutex<Engine>>,
}

impl AudioLink {
    pub fn set_scene(&self, scene: Scene) {
        let _ = self.tx.send(scene);
    }

    pub fn now_seconds(&self) -> f64 {
        self.engine.lock().map(|e| e.now_seconds()).unwrap_or(0.0)
    }

    pub fn voices(&self) -> usize {
        self.engine.lock().map(|e| e.active_voices()).unwrap_or(0)
    }
}

/// Arranca el hilo de audio. El `Stream` de cpal no es `Send`, así que vive en
/// su propio hilo y nunca sale de ahí.
pub fn start(bank: Arc<SampleBank>, scene: Scene) -> anyhow::Result<AudioLink> {
    let engine = Arc::new(Mutex::new(Engine::new(bank, scene, EngineOptions { output_db: -3.0, ..Default::default() })));
    let (tx, rx) = channel::<Scene>();
    let (ready_tx, ready_rx) = channel::<anyhow::Result<()>>();
    let eng = engine.clone();

    std::thread::Builder::new().name("lienzo-audio".into()).spawn(move || {
        let host = cpal::default_host();
        let Some(device) = host.default_output_device() else {
            let _ = ready_tx.send(Err(anyhow::anyhow!("no hay dispositivo de salida de audio")));
            return;
        };
        let cfg = match device.supported_output_configs() {
            Ok(it) => it
                .filter(|c| c.channels() == 2 && c.sample_format() == cpal::SampleFormat::F32)
                .find(|c| c.min_sample_rate() <= SR && SR <= c.max_sample_rate())
                .map(|c| c.with_sample_rate(SR).config()),
            Err(_) => None,
        };
        let Some(config) = cfg else {
            let _ = ready_tx.send(Err(anyhow::anyhow!(
                "la tarjeta de sonido no ofrece estéreo f32 a {} Hz",
                SR
            )));
            return;
        };
        let mut scratch: Vec<[f32; 2]> = Vec::new();
        let stream = device.build_output_stream(
            config,
            move |data: &mut [f32], _| {
                let frames = data.len() / 2;
                scratch.resize(frames, [0.0; 2]);
                if let Ok(mut e) = eng.lock() {
                    while let Ok(scene) = rx.try_recv() {
                        e.set_scene(scene);
                    }
                    e.process(&mut scratch);
                }
                for (i, f) in scratch.iter().enumerate() {
                    data[2 * i] = f[0];
                    data[2 * i + 1] = f[1];
                }
            },
            |err| error!("audio: {err}"),
            None,
        );
        match stream {
            Ok(s) => {
                if let Err(e) = s.play() {
                    let _ = ready_tx.send(Err(e.into()));
                    return;
                }
                let _ = ready_tx.send(Ok(()));
                // el stream vive mientras viva este hilo
                loop {
                    std::thread::park();
                }
            }
            Err(e) => {
                let _ = ready_tx.send(Err(e.into()));
            }
        }
    })?;

    ready_rx.recv()??;
    Ok(AudioLink { tx, engine })
}
