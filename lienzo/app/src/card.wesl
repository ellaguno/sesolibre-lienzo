// Textura × color, niebla exponencial cuadrática por distancia a la cámara,
// sin luces. Ver material.rs.

import bevy_pbr::render::forward_io::{Vertex, VertexOutput, decompress_vertex};
import bevy_pbr::render::mesh_view_bindings::{view, globals};
import bevy_pbr::render::mesh_functions;
import bevy_pbr::render::view_transformations::position_world_to_clip;

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
@group(3) @binding(3) var caustic_texture: texture_2d<f32>;
@group(3) @binding(4) var caustic_sampler: sampler;

// Cáusticas reales a dos lecturas: la textura celular se desplaza en dos
// direcciones y se multiplica; el producto solo brilla donde ambas coinciden,
// que es como se mueven las redes de luz en un fondo de arena.
fn waves(p: vec2<f32>, t: f32) -> f32 {
    let a = sin(p.x * 0.9 + t * 0.35 + p.y * 0.3);
    let b = sin(p.y * 1.3 - t * 0.27 + p.x * 0.5);
    let c = sin((p.x - p.y) * 0.6 + t * 0.19);
    return (a + b + c) / 3.0;
}

fn caustic_tex(p0: vec2<f32>, t: f32) -> f32 {
    let k = material.extra.y * 0.21;
    // la red se ondula con el oleaje: corrientes, no un estampado fijo
    let p = p0 + 0.6 * vec2<f32>(waves(p0 * 0.45, t * 0.7), waves(p0 * 0.45 + vec2<f32>(7.0, 3.0), t * 0.6));
    // fract: la repetición no depende del sampler (en GLES se ignoraba)
    let a = textureSample(caustic_texture, caustic_sampler, fract(p * k + vec2<f32>(t * 0.013, t * 0.021))).r;
    // la segunda lectura va girada y a otra escala: nada de cuadrícula
    let q = vec2<f32>(p.x * 0.83 - p.y * 0.56, p.x * 0.56 + p.y * 0.83);
    let b = textureSample(caustic_texture, caustic_sampler, fract(q * k * 1.61 + vec2<f32>(-t * 0.017, t * 0.009) + vec2<f32>(0.37, 0.61))).r;
    return min(a, b) * 1.4;
}

// Corrientes: un campo lento de mundo que mece las cartas (más arriba, más;
// la base queda anclada). Ni el relieve, ni la superficie, ni el fondo, ni
// los rayos: solo lo que flota o crece.
fn current(p: vec3<f32>, t: f32) -> vec3<f32> {
    let a = sin(p.x * 0.55 + t * 0.37 + p.z * 0.3) + 0.5 * sin(p.z * 1.1 - t * 0.23);
    let b = cos(p.z * 0.6 + t * 0.31 + p.x * 0.4) + 0.5 * cos(p.x * 0.9 + t * 0.19);
    let c = sin((p.x + p.z) * 0.4 + t * 0.27);
    return vec3<f32>(a, 0.35 * c, b) * 0.09;
}

@vertex
fn vertex(v: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let vert = decompress_vertex(v, v.instance_index);
    let world_from_local = mesh_functions::get_world_from_local(v.instance_index);
    out.world_normal = mesh_functions::mesh_normal_local_to_world(vert.normal, v.instance_index);
    var wp = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(vert.position, 1.0));
    if material.extra.w < 0.5 && material.extra.z > -0.5 && material.extra.z < 0.5 {
        // la carta se mece con la corriente; la parte alta (uv.y = 0) más
        let sway = 1.0 - vert.uv.y;
        let root = world_from_local[3].xyz;
        wp = vec4<f32>(wp.xyz + current(root, globals.time) * sway, wp.w);
    }
    out.world_position = wp;
    out.position = position_world_to_clip(wp.xyz);
    out.uv = vert.uv;
    @if(VERTEX_OUTPUT_INSTANCE_INDEX)
    out.instance_index = v.instance_index;
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    if material.extra.z > 1.5 {
        // Desde arriba (vista de dibujo) la superficie no existe: el papel.
        if view.world_position.y > in.world_position.y + 0.05 {
            discard;
        }
        // Superficie desde abajo. Mirando casi a plomo se ve el cielo por la
        // ventana de Snell (~48°); más allá, el agua se refleja a sí misma.
        // El oleaje mueve el borde de la ventana y riza la luz.
        let to_frag = in.world_position.xyz - view.world_position;
        let dist = length(to_frag);
        let up = to_frag.y / max(dist, 0.001);
        let w = waves(in.world_position.xz, globals.time);
        let window = smoothstep(0.52 + 0.10 * w, 0.78 + 0.10 * w, up);
        let ripple = 0.92 + 0.10 * waves(in.world_position.xz * 2.3 + vec2<f32>(11.0, 5.0), globals.time * 1.3);
        // destellos: la red de cáusticas vista desde abajo, y el sol, un disco
        // suave algo descentrado hacia donde entran los rayos
        let glitter = caustic_tex(in.world_position.xz * 2.4, globals.time * 1.4) * 0.09;
        let sun_dir = normalize(vec3<f32>(0.18, 1.0, -0.22));
        let sun = pow(clamp(dot(to_frag / max(dist, 0.001), sun_dir), 0.0, 1.0), 180.0) * 1.6
            + pow(clamp(dot(to_frag / max(dist, 0.001), sun_dir), 0.0, 1.0), 12.0) * 0.25;
        let sky = material.color.rgb * (ripple + glitter + sun);
        let mirror = material.fog_color.rgb * (0.75 + 0.2 * w);
        var s = mix(mirror, sky, window);
        let d = dist * material.params.x * 0.7;
        let f = 1.0 - exp(-d * d);
        s = mix(s, material.fog_color.rgb, f);
        return vec4<f32>(s, 1.0);
    }
    if material.extra.z > 0.5 {
        // Fondo: del horizonte (a la altura de la mirada, ~2) a la superficie.
        let h = clamp((in.world_position.y - 2.0) / 45.0, 0.0, 1.0);
        let k = h * h * (3.0 - 2.0 * h);
        return vec4<f32>(mix(material.fog_color.rgb, material.color.rgb, k), 1.0);
    }
    // el relieve repite su arena sin fin (fract: el sampler no decide)
    var uv = in.uv;
    if material.extra.w > 0.5 {
        uv = fract(in.uv);
    }
    var c = material.color * textureSample(card_texture, card_sampler, uv);
    if material.extra.z < -0.5 {
        // rayo de luz: un velo que se desvanece cuando se mira de canto (al
        // mirar al fondo no se ven franjas, solo cáusticas) y que no tiene
        // bordes: nunca parece una pared
        let to_cam = view.world_position - in.world_position.xyz;
        let near = length(to_cam);
        let facing = abs(dot(normalize(in.world_normal), to_cam / max(near, 0.001)));
        // y se disuelve si está encima de la cámara: nunca una barra en la cara
        // y desde fuera del agua (vista de dibujo) no existen
        let under = 0.35 + 0.65 * smoothstep(5.0, 3.8, view.world_position.y);
        c = vec4<f32>(c.rgb, c.a * smoothstep(0.15, 0.7, facing) * smoothstep(1.5, 7.0, near) * under);
    }
    if material.extra.w > 0.5 {
        // relieve: las laderas que miran a la luz (arriba y un poco al frente)
        // se aclaran; las que se alejan, se oscurecen
        let n = normalize(in.world_normal);
        let lit = clamp(dot(n, normalize(vec3<f32>(0.25, 1.0, 0.35))), 0.0, 1.0);
        // y las fosas se oscurecen, las crestas se aclaran
        let depth = clamp(in.world_position.y / 1.6, -1.0, 1.0);
        c = vec4<f32>(c.rgb * (0.55 + 0.55 * lit) * (1.0 + 0.4 * depth), c.a);
    }
    let caustic_strength = material.params.w * material.extra.x;
    if caustic_strength > 0.001 {
        // más luz cerca de la superficie (y alta), casi nada en el abismo
        let height = clamp(in.world_position.y / 5.0, 0.0, 1.0);
        // y solo cerca: lejos la red se funde en un velo (y evita el moiré)
        let dc = distance(in.world_position.xyz, view.world_position) * 0.055;
        let near_c = exp(-dc * dc);
        let light = caustic_tex(in.world_position.xz, globals.time) * caustic_strength * (0.35 + 0.65 * height) * near_c;
        c = vec4<f32>(c.rgb * (1.0 + light) + vec3<f32>(0.6, 0.8, 0.85) * light * 0.25, c.a);
    }
    if material.params.y > 0.5 {
        let d = distance(in.world_position.xyz, view.world_position) * material.params.x;
        let f = 1.0 - exp(-d * d);
        if material.params.z > 0.5 {
            // la luz aditiva no se tiñe de niebla: se apaga con la distancia
            c = vec4<f32>(c.rgb * sqrt(1.0 - f), c.a);
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
