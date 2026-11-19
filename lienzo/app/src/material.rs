//! El único material de la app: textura × color, con niebla por distancia y
//! sin iluminación. Es nuestro en vez del `StandardMaterial` de Bevy porque el
//! shader PBR exige extensiones (sombras con LOD, storage buffers) que GLES en
//! Mali no ofrece, y porque no necesitamos luces: el teatro de papel es plano.

use bevy::asset::AssetPath;
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{Material, MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, RenderPipelineDescriptor, SpecializedMeshPipelineError,
};
use bevy::shader::ShaderRef;

pub const FOG_COLOR: Color = Color::srgb(0.05, 0.16, 0.26);
pub const FOG_DENSITY: f32 = 0.045;

#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub struct CardMaterial {
    #[uniform(0)]
    pub color: LinearRgba,
    #[uniform(0)]
    pub fog_color: LinearRgba,
    /// x: densidad de niebla · y: niebla activa · z: premultiplicar alfa (modo
    /// aditivo) · w: fuerza de cáusticas propia del material (0 = ninguna).
    #[uniform(0)]
    pub params: Vec4,
    /// x: multiplicador de cáusticas del agua · y: escala espacial de las
    /// cáusticas · z: 1 = es el fondo (degradado por altura: `color` arriba,
    /// `fog_color` en el horizonte) · w: sin uso.
    #[uniform(0)]
    pub extra: Vec4,
    #[texture(1)]
    #[sampler(2)]
    pub texture: Handle<Image>,
    pub alpha: AlphaMode,
}

impl CardMaterial {
    fn new(texture: Handle<Image>, color: Color, alpha: AlphaMode, fog: bool) -> Self {
        Self::with_caustics(texture, color, alpha, fog, 0.0)
    }

    fn with_caustics(
        texture: Handle<Image>,
        color: Color,
        alpha: AlphaMode,
        fog: bool,
        caustics: f32,
    ) -> Self {
        let premul = if matches!(alpha, AlphaMode::Add) {
            1.0
        } else {
            0.0
        };
        CardMaterial {
            color: color.into(),
            fog_color: FOG_COLOR.into(),
            params: Vec4::new(FOG_DENSITY, if fog { 1.0 } else { 0.0 }, premul, caustics),
            extra: Vec4::new(1.0, 0.9, 0.0, 0.0),
            texture,
            alpha,
        }
    }

    /// Sprite recortado: mezcla por alfa, con niebla y cáusticas suaves.
    pub fn sprite(texture: Handle<Image>) -> Self {
        Self::with_caustics(texture, Color::WHITE, AlphaMode::Blend, true, 0.35)
    }

    /// Opaco con niebla y cáusticas marcadas (suelo); se sombrea por la
    /// pendiente (extra.w) para que el relieve se lea.
    pub fn opaque(texture: Handle<Image>) -> Self {
        let mut m = Self::with_caustics(texture, Color::WHITE, AlphaMode::Opaque, true, 0.9);
        m.extra.w = 1.0;
        m
    }

    /// El fondo: degradado por altura calculado en el shader. `color` es el
    /// agua junto a la superficie; `fog_color`, el horizonte.
    pub fn backdrop(texture: Handle<Image>, top: Color) -> Self {
        let mut m = Self::new(texture, top, AlphaMode::Opaque, false);
        m.extra.z = 1.0;
        m
    }

    /// La superficie del mar vista desde abajo: `color` es el cielo que entra
    /// por la ventana de Snell; fuera de ella se refleja el agua (fog_color).
    pub fn surface(texture: Handle<Image>, sky: Color) -> Self {
        let mut m = Self::new(texture, sky, AlphaMode::Opaque, true);
        m.extra.z = 2.0;
        m
    }

    /// Acuarela: mezcla por alfa, teñida de un color, con niebla y cáusticas suaves.
    pub fn watercolor(texture: Handle<Image>, color: Color) -> Self {
        Self::with_caustics(texture, color, AlphaMode::Blend, true, 0.25)
    }

    /// Aditivo (luz, partículas), con niebla.
    pub fn additive(texture: Handle<Image>, alpha: f32) -> Self {
        Self::new(
            texture,
            Color::srgba(1.0, 1.0, 1.0, alpha),
            AlphaMode::Add,
            true,
        )
    }

    /// Luz de color (farol, haz, arcoíris): aditiva y teñida.
    pub fn glow(texture: Handle<Image>, rgb: [f32; 3], alpha: f32) -> Self {
        Self::new(
            texture,
            Color::srgba(rgb[0], rgb[1], rgb[2], alpha),
            AlphaMode::Add,
            true,
        )
    }
}

/// Con la bandera de depuración `min_shader` se usa la variante mínima
/// (`card_min.wesl`), para bisecar problemas de GLES en el teléfono.
pub static MIN_SHADER: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

impl Material for CardMaterial {
    fn fragment_shader() -> ShaderRef {
        let minimal = *MIN_SHADER.get().unwrap_or(&false);
        let path = if minimal {
            bevy::asset::embedded_path!("card_min.wesl")
        } else {
            bevy::asset::embedded_path!("card.wesl")
        };
        ShaderRef::Path(AssetPath::from_path_buf(path).with_source("embedded"))
    }

    fn alpha_mode(&self) -> AlphaMode {
        self.alpha
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // las cartas se ven por los dos lados
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}
