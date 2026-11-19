//! Lienzo — la app. Bevy para ver y tocar; el core para componer y sonar.

mod audio;
mod cards;
mod draw;
mod gallery;
mod input;
mod material;
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

/// En Android los assets viven dentro del APK y se leen con el AssetManager.
#[cfg(target_os = "android")]
pub struct ApkSource;

#[cfg(target_os = "android")]
impl lienzo_core::AssetSource for ApkSource {
    fn read(&self, rel: &str) -> Result<Vec<u8>, String> {
        use std::io::Read;
        let am = bevy::android::ANDROID_APP
            .get()
            .ok_or("ANDROID_APP no inicializado")?
            .asset_manager();
        let path = std::ffi::CString::new(format!("audio/{rel}")).map_err(|e| e.to_string())?;
        let mut asset = am
            .open(&path)
            .ok_or_else(|| format!("no está en el APK: audio/{rel}"))?;
        let mut bytes = Vec::new();
        asset.read_to_end(&mut bytes).map_err(|e| e.to_string())?;
        Ok(bytes)
    }
}

/// Raíz del proyecto: la carpeta que contiene `assets/manifest.toml`.
#[cfg(not(target_os = "android"))]
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

/// Ajustes del renderer. En Android se usa GLES: el backend Vulkan de wgpu
/// aborta en los drivers Mali (pool de descriptores agotado en un camino que
/// wgpu considera imposible), y pasa igual con los *binding arrays* apagados.
fn render_plugin() -> bevy::render::RenderPlugin {
    use bevy::render::settings::{Backends, WgpuLimits, WgpuSettings};
    let mut settings = WgpuSettings::default();
    if cfg!(target_os = "android") {
        settings.backends = Some(Backends::GL);
        // Sin storage buffers (GLES no los ofrece en el vértice): Bevy pasa a
        // uniformes, como en WebGL2. Bevy solo lo hace solo en wasm, y con la
        // prioridad por defecto ignora `limits`; `constrained_limits` sí acota.
        let mut limits = WgpuLimits::downlevel_webgl2_defaults();
        limits.max_texture_dimension_2d = 8192;
        settings.constrained_limits = Some(limits);
    }
    bevy::render::RenderPlugin {
        render_creation: settings.into(),
        ..default()
    }
}

#[bevy_main]
pub fn main() {
    if let Err(e) = run() {
        eprintln!("lienzo: {e:#}");
    }
    // En Android la actividad puede destruirse y volver a crearse en el mismo
    // proceso, y winit no permite un segundo bucle de eventos: se sale del
    // proceso para que la siguiente apertura arranque limpia.
    #[cfg(target_os = "android")]
    std::process::exit(0);
}

pub fn run() -> anyhow::Result<()> {
    #[cfg(not(target_os = "android"))]
    let (asset_plugin, bank) = {
        let assets_dir = project_root().join("assets");
        let bank = SampleBank::load(&assets_dir.join("audio"))?;
        (
            AssetPlugin {
                file_path: assets_dir.to_string_lossy().into_owned(),
                ..default()
            },
            bank,
        )
    };
    #[cfg(target_os = "android")]
    let (asset_plugin, bank) = (AssetPlugin::default(), SampleBank::load_from(&ApkSource)?);
    let bank = Arc::new(bank);
    let path = std::env::args().nth(1).map(PathBuf::from);
    let scene = match &path {
        Some(p) if p.exists() => Scene::from_json(&std::fs::read_to_string(p)?)?,
        Some(_) => Scene::new(7),
        None => Scene::demo(),
    };

    let link = audio::start(bank.clone(), scene.clone())?;

    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Lienzo".into(),
                    resolution: WindowResolution::new(1280, 800),
                    ..default()
                }),
                ..default()
            })
            .set(asset_plugin)
            .set(render_plugin()),
    );
    bevy::asset::embedded_asset!(app, "card.wesl");
    // Obra contemplativa, no juego: ~30 fps en el teléfono y 60 en escritorio
    // bastan, y responde al instante a cualquier gesto. En segundo plano, casi
    // nada. Esto es lo que más batería y CPU ahorra.
    app.insert_resource(bevy::winit::WinitSettings {
        focused_mode: bevy::winit::UpdateMode::reactive(std::time::Duration::from_millis(
            if cfg!(target_os = "android") { 33 } else { 16 },
        )),
        unfocused_mode: bevy::winit::UpdateMode::reactive_low_power(std::time::Duration::from_millis(250)),
    });
    app.add_plugins(bevy::pbr::MaterialPlugin::<material::CardMaterial>::default())
        .insert_resource(Obra {
            scene,
            dirty: false,
            new_stroke: None,
            path,
        })
        .init_resource::<draw::DrawState>()
        .init_resource::<input::Pointer>()
        .init_resource::<gallery::Gallery>()
        .init_resource::<gallery::Pending>()
        .init_resource::<gallery::Exporting>()
        .insert_resource(gallery::Bank(bank))
        .insert_resource(link)
        .init_resource::<cards::SpriteMaterials>()
        .add_systems(
            Startup,
            (
                ui::load_font,
                world::setup,
                cards::spawn_scene,
                ui::setup,
                gallery::setup,
            )
                .chain(),
        )
        .add_observer(ui::on_click)
        .add_observer(gallery::on_click)
        .add_systems(
            Update,
            (
                input::gather,
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
