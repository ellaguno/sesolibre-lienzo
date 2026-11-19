// Textura × color, niebla exponencial cuadrática por distancia a la cámara,
// sin luces. Ver material.rs.

import bevy_pbr::render::forward_io::VertexOutput;
import bevy_pbr::render::mesh_view_bindings::{view, globals};

struct CardMaterial {
    color: vec4<f32>,
    fog_color: vec4<f32>,
    params: vec4<f32>,
    extra: vec4<f32>,
}

// Cáusticas: tres ondas que se cruzan y se mueven despacio; el cuadrado afina
// las líneas de luz. Se evalúa en el plano XZ del mundo, así que el suelo y las
// cartas comparten la misma red de luz.
fn caustics(p: vec3<f32>, t: f32) -> f32 {
    let k = material.extra.y;
    let a = sin(p.x * 1.9 * k + t * 0.60 + p.z * 0.7 * k);
    let b = sin(p.z * 2.3 * k - t * 0.45 + p.x * 0.4 * k);
    let c = sin((p.x + p.z) * 1.1 * k + t * 0.33);
    let v = a * b * c;
    return v * v;
}

@group(3) @binding(0) var<uniform> material: CardMaterial;
@group(3) @binding(1) var card_texture: texture_2d<f32>;
@group(3) @binding(2) var card_sampler: sampler;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    if material.extra.z > 0.5 {
        // Fondo: del horizonte (a la altura de la mirada, ~2) a la superficie.
        let h = clamp((in.world_position.y - 2.0) / 45.0, 0.0, 1.0);
        let k = h * h * (3.0 - 2.0 * h);
        return vec4<f32>(mix(material.fog_color.rgb, material.color.rgb, k), 1.0);
    }
    var c = material.color * textureSample(card_texture, card_sampler, in.uv);
    let caustic_strength = material.params.w * material.extra.x;
    if caustic_strength > 0.001 {
        // más luz cerca de la superficie (y alta), casi nada en el abismo
        let height = clamp(in.world_position.y / 5.0, 0.0, 1.0);
        let light = caustics(in.world_position.xyz, globals.time) * caustic_strength * (0.35 + 0.65 * height);
        c = vec4<f32>(c.rgb * (1.0 + light) + vec3<f32>(0.6, 0.8, 0.85) * light * 0.25, c.a);
    }
    if material.params.y > 0.5 {
        let d = distance(in.world_position.xyz, view.world_position) * material.params.x;
        let f = 1.0 - exp(-d * d);
        if material.params.z > 0.5 {
            // la luz aditiva no se tiñe de niebla: se apaga con la distancia
            c = vec4<f32>(c.rgb * (1.0 - f), c.a);
        } else {
            c = vec4<f32>(mix(c.rgb, material.fog_color.rgb, f), c.a);
        }
    }
    if material.params.z > 0.5 {
        // Aditivo de verdad: Bevy mezcla "Add" como premultiplicado
        // (destino × (1 − α)); con α = 0 el destino se conserva y la luz se suma.
        c = vec4<f32>(c.rgb * c.a, 0.0);
    }
    return c;
}
