// Textura × color, niebla exponencial cuadrática por distancia a la cámara,
// sin luces. Ver material.rs.

import bevy_pbr::render::forward_io::VertexOutput;
import bevy_pbr::render::mesh_view_bindings::view;

struct CardMaterial {
    color: vec4<f32>,
    fog_color: vec4<f32>,
    params: vec4<f32>,
}

@group(3) @binding(0) var<uniform> material: CardMaterial;
@group(3) @binding(1) var card_texture: texture_2d<f32>;
@group(3) @binding(2) var card_sampler: sampler;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    var c = material.color * textureSample(card_texture, card_sampler, in.uv);
    if material.params.y > 0.5 {
        let d = distance(in.world_position.xyz, view.world_position) * material.params.x;
        let f = 1.0 - exp(-d * d);
        c = vec4<f32>(mix(c.rgb, material.fog_color.rgb, f), c.a);
    }
    if material.params.z > 0.5 {
        c = vec4<f32>(c.rgb * c.a, c.a);
    }
    return c;
}
