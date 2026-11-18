//! `lienzo` — CLI del núcleo. Para escuchar la obra sin app.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use lienzo_core::{compose, director, render_offline, write_wav, EngineOptions, SampleBank, Scene};
use std::sync::Arc;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "lienzo", about = "Lienzo: de la escena al audio, sin UI")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Escribe la escena de demostración como JSON.
    Demo {
        #[arg(short, long, default_value = "demo.json")]
        out: PathBuf,
    },
    /// Muestra los motivos y el conteo de eventos de una escena.
    Describe {
        scene: PathBuf,
        #[arg(short, long, default_value_t = 180.0)]
        seconds: f64,
    },
    /// Renderiza una escena a WAV.
    Render {
        scene: PathBuf,
        #[arg(short, long, default_value = "obra.wav")]
        out: PathBuf,
        #[arg(short, long, default_value_t = 180.0)]
        seconds: f64,
        /// Carpeta `assets/audio` con el pack curado.
        #[arg(short, long, default_value = "../assets/audio")]
        assets: PathBuf,
        #[arg(long, default_value_t = 0.45)]
        wet: f32,
    },
}

fn load_scene(p: &PathBuf) -> Result<Scene> {
    let s = std::fs::read_to_string(p).with_context(|| format!("leyendo {}", p.display()))?;
    Scene::from_json(&s).with_context(|| format!("{} no es una escena válida", p.display()))
}

fn main() -> Result<()> {
    match Cli::parse().cmd {
        Cmd::Demo { out } => {
            std::fs::write(&out, Scene::demo().to_json())?;
            println!("escrito {}", out.display());
        }
        Cmd::Describe { scene, seconds } => {
            let sc = load_scene(&scene)?;
            let ev = compose(&sc, seconds);
            print!("{}", director::describe(&sc, &ev));
        }
        Cmd::Render { scene, out, seconds, assets, wet } => {
            let sc = load_scene(&scene)?;
            let t0 = std::time::Instant::now();
            let bank = Arc::new(SampleBank::load(&assets).context("cargando muestras")?);
            let ev = compose(&sc, seconds);
            let audio = render_offline(bank, &sc, seconds, EngineOptions { wet, ..Default::default() });
            write_wav(&out, &audio)?;
            println!(
                "{} · {:.0} s · {} eventos · {:.1} s de render",
                out.display(),
                seconds,
                ev.len(),
                t0.elapsed().as_secs_f32()
            );
        }
    }
    Ok(())
}
