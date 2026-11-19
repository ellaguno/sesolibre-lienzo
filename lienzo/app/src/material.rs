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
    /// aditivo) · w: sin uso.
    #[uniform(0)]
    pub params: Vec4,
    #[texture(1)]
    #[sampler(2)]
    pub texture: Handle<Image>,
    pub alpha: AlphaMode,
}

impl CardMaterial {
    fn new(texture: Handle<Image>, color: Color, alpha: AlphaMode, fog: bool) -> Self {
        let premul = if matches!(alpha, AlphaMode::Add) { 1.0 } else { 0.0 };
        CardMaterial {
            color: color.into(),
            fog_color: FOG_COLOR.into(),
            params: Vec4::new(FOG_DENSITY, if fog { 1.0 } else { 0.0 }, premul, 0.0),
            texture,
            alpha,
        }
    }

    /// Sprite recortado: mezcla por alfa, con niebla.
    pub fn sprite(texture: Handle<Image>) -> Self {
        Self::new(texture, Color::WHITE, AlphaMode::Blend, true)
    }

    /// Opaco con niebla (suelo).
    pub fn opaque(texture: Handle<Image>) -> Self {
        Self::new(texture, Color::WHITE, AlphaMode::Opaque, true)
    }

    /// Opaco sin niebla (fondo).
    pub fn backdrop(texture: Handle<Image>) -> Self {
        Self::new(texture, Color::WHITE, AlphaMode::Opaque, false)
    }

    /// Aditivo (luz, partículas), con niebla.
    pub fn additive(texture: Handle<Image>, alpha: f32) -> Self {
        Self::new(texture, Color::srgba(1.0, 1.0, 1.0, alpha), AlphaMode::Add, true)
    }
}

impl Material for CardMaterial {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Path(
            AssetPath::from_path_buf(bevy::asset::embedded_path!("card.wesl")).with_source("embedded"),
        )
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
