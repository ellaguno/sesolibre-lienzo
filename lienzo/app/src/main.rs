//! Lienzo — la app. Bevy para ver y tocar; el core para componer y sonar.

mod audio;
mod cards;
mod draw;
mod gallery;
mod orbit;
mod textures;
mod ui;
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
    /// Índice del trazo recién añadido, para que suene al instante.
    pub new_stroke: Option<usize>,
    pub path: Option<PathBuf>,
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
    let path = std::env::args().nth(1).map(PathBuf::from);
    let scene = match &path {
        Some(p) if p.exists() => Scene::from_json(&std::fs::read_to_string(p)?)?,
        Some(_) => Scene::new(7),
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
                .set(AssetPlugin {
                    file_path: assets_dir.to_string_lossy().into_owned(),
                    ..default()
                }),
        )
        .insert_resource(Obra {
            scene,
            dirty: false,
            new_stroke: None,
            path,
        })
        .init_resource::<draw::DrawState>()
        .init_resource::<gallery::Gallery>()
        .init_resource::<gallery::Pending>()
        .insert_resource(link)
        .init_resource::<cards::SpriteMaterials>()
        .add_systems(
            Startup,
            (world::setup, cards::spawn_scene, ui::setup, gallery::setup).chain(),
        )
        .add_observer(ui::on_click)
        .add_observer(gallery::on_click)
        .add_systems(
            Update,
            (
                gallery::keys,
                gallery::apply,
                gallery::refresh,
                draw::keys,
                draw::draw,
                draw::touch,
                ui::refresh,
                orbit::control,
                world::drift_particles,
                cards::animate,
                world::billboards,
                draw::title,
                sync_audio,
            )
                .chain(),
        )
        .run();
    Ok(())
}

fn sync_audio(mut obra: ResMut<Obra>, link: Res<audio::AudioLink>) {
    if obra.dirty {
        link.set_scene(obra.scene.clone(), obra.new_stroke.take());
        obra.dirty = false;
    }
}
