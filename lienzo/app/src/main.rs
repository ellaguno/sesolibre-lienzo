//! Lienzo — la app. Bevy para ver y tocar; el core para componer y sonar.

mod audio;
mod cards;
mod orbit;
mod textures;
mod world;

use bevy::prelude::*;
use bevy::window::WindowResolution;
use lienzo_core::{SampleBank, Scene};
use std::path::PathBuf;
use std::sync::Arc;

/// La obra abierta. Fuente de verdad en la app; se manda al audio al cambiar.
#[derive(Resource)]
pub struct Obra {
    pub scene: Scene,
    pub dirty: bool,
}

/// Raíz del proyecto: la carpeta que contiene `assets/manifest.toml`.
fn project_root() -> PathBuf {
    let mut d = std::env::current_dir().expect("cwd");
    loop {
        if d.join("assets").join("manifest.toml").exists() {
            return d;
        }
        if !d.pop() {
            panic!("no encuentro assets/manifest.toml subiendo desde el directorio actual");
        }
    }
}

fn main() -> anyhow::Result<()> {
    let root = project_root();
    let assets_dir = root.join("assets");
    let scene = match std::env::args().nth(1) {
        Some(p) => Scene::from_json(&std::fs::read_to_string(&p)?)?,
        None => Scene::demo(),
    };

    let bank = Arc::new(SampleBank::load(&assets_dir.join("audio"))?);
    let link = audio::start(bank, scene.clone())?;

    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Lienzo".into(),
                        resolution: WindowResolution::new(1280, 800),
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin { file_path: assets_dir.to_string_lossy().into_owned(), ..default() }),
        )
        .insert_resource(Obra { scene, dirty: false })
        .insert_resource(link)
        .init_resource::<cards::SpriteMaterials>()
        .add_systems(Startup, (world::setup, cards::spawn_scene).chain())
        .add_systems(Update, (orbit::control, world::drift_particles, cards::animate, world::billboards, sync_audio).chain())
        .run();
    Ok(())
}

fn sync_audio(mut obra: ResMut<Obra>, link: Res<audio::AudioLink>) {
    if obra.dirty {
        link.set_scene(obra.scene.clone());
        obra.dirty = false;
    }
}
