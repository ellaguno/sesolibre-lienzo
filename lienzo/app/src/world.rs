//! El abismo vacío: cámara, niebla, fondo, suelo, rayos y partículas. Lo que se
//! ve antes del primer trazo, y lo que ya suena.

use crate::material::CardMaterial;
use crate::orbit::Orbit;
use crate::textures;
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use bevy::render::view::Tonemapping;

/// Tamaño del volumen en unidades de mundo. El core usa x,z ∈ [-1,1], y ∈ [0,1].
pub const HALF_W: f32 = 6.0;
pub const HEIGHT: f32 = 5.0;

pub fn to_world(p: lienzo_core::Point) -> Vec3 {
    Vec3::new(p.x * HALF_W, p.y * HEIGHT, p.z * HALF_W)
}

pub fn to_core(v: Vec3) -> lienzo_core::Point {
    lienzo_core::Point::new(
        (v.x / HALF_W).clamp(-1.0, 1.0),
        (v.y / HEIGHT).clamp(0.0, 1.0),
        (v.z / HALF_W).clamp(-1.0, 1.0),
    )
}

#[derive(Component)]
pub struct Particle {
    speed: f32,
    phase: f32,
}

#[derive(Component)]
pub struct Billboard;

/// Lo que solo existe bajo el agua (superficie, rayos, partículas): sobre el
/// papel se oculta del todo y sus sistemas no corren.
#[derive(Component)]
pub struct Submerged;

#[derive(Component)]
pub struct Ray {
    pub tilt: f32,
    pub phase: f32,
    pub width: f32,
    pub x: f32,
    pub z: f32,
    /// Dibujado con el pincel Haz: no se apaga en la vista de dibujo.
    pub drawn: bool,
}

#[derive(Resource)]
pub struct Common {
    pub quad: Handle<Mesh>,
    pub dot: Handle<CardMaterial>,
    /// Textura de acuarela para los trazos de color.
    pub blob: Handle<Image>,
    /// Texturas de la familia Luz.
    pub glow_tex: Handle<Image>,
    pub ray_tex: Handle<Image>,
    pub rainbow_tex: Handle<Image>,
    pub backdrop: Handle<CardMaterial>,
    pub sand: Handle<CardMaterial>,
    pub surface: Handle<CardMaterial>,
    pub ray: Handle<CardMaterial>,
    /// El borde del lienzo (solo visible sobre el papel).
    pub frame: Handle<CardMaterial>,
}

pub fn setup(
    flags: Res<crate::DebugFlags>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<CardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let mut cam = commands.spawn((
        Camera3d::default(),
        // sin multimuestreo: las cartas son quads con bordes suaves y el
        // relleno a 2 Mpx en el teléfono es lo que más cuesta
        Msaa::Off,
        Transform::from_xyz(0.0, 3.0, 15.0).looking_at(Vec3::new(0.0, 2.2, 0.0), Vec3::Y),
        Tonemapping::AcesFitted,
        Orbit::default(),
    ));
    // en GLES (Mali) el bloom oscurece la imagen a la mitad: se omite en Android
    if !flags.has("no_bloom") && (!cfg!(target_os = "android") || flags.has("bloom")) {
        cam.insert(Bloom {
            intensity: 0.18,
            ..Bloom::NATURAL
        });
    }
    // refracción: la imagen entera ondula despacio (ver `refraction`). La pila
    // de efectos de Bevy pinta negro en GLES (Mali), así que en Android no va.
    if !cfg!(target_os = "android") {
        cam.insert((
            bevy::post_process::effect_stack::LensDistortion {
                intensity: 0.0,
                ..default()
            },
            bevy::post_process::effect_stack::ChromaticAberration {
                intensity: 0.004,
                ..default()
            },
            bevy::post_process::effect_stack::Vignette {
                intensity: 0.25,
                ..default()
            },
        ));
    }
    let lk = crate::water::look(lienzo_core::Water::Abismo);
    commands.insert_resource(ClearColor(lk.clear));

    let quad = meshes.add(Rectangle::new(1.0, 1.0));
    let caustic_img = textures::caustic_tile();
    if let Ok(path) = std::env::var("LIENZO_DUMP_CAUSTIC") {
        if let Some(data) = caustic_img.data.as_ref() {
            let w = caustic_img.width();
            let h = caustic_img.height();
            let mut ppm = format!("P6 {w} {h} 255\n").into_bytes();
            for px in data.chunks(4) {
                ppm.extend_from_slice(&px[..3]);
            }
            let _ = std::fs::write(path, ppm);
        }
    }
    let _ = crate::material::CAUSTIC_TEX.set(images.add(caustic_img));

    // fondo: una esfera que envuelve todo, vista desde dentro, sin niebla.
    // Gire a donde gire la cámara, siempre hay agua.
    let backdrop = materials.add(CardMaterial::backdrop(
        images.add(textures::soft_dot()),
        Color::srgb(lk.top[0], lk.top[1], lk.top[2]),
    ));
    if !flags.has("no_backdrop") {
        commands.spawn((
        Mesh3d(meshes.add(Sphere::new(400.0).mesh().uv(48, 24))),
        MeshMaterial3d(backdrop.clone()),
        // centrada a la altura de la mirada: el horizonte del fondo coincide
        // con el borde del suelo, y la niebla los funde.
        Transform::from_xyz(0.0, 2.0, 0.0),
        ));
    }

    // suelo
    // la arena es gris: el tinte (arena del agua o papel) lo pone water::apply
    let mut sand_mat = CardMaterial::opaque(images.add(textures::sand([1.0, 1.0, 1.0])));
    sand_mat.color = Color::srgb(lk.sand[0], lk.sand[1], lk.sand[2]).into();
    let sand = materials.add(sand_mat);
    // el borde del lienzo: cuatro listones finos sobre el papel
    let frame = materials.add(CardMaterial::new(
        images.add(textures::white()),
        Color::srgba(0.45, 0.40, 0.34, 0.0),
        bevy::prelude::AlphaMode::Blend,
        false,
    ));
    for (x, z, w, h) in [
        (0.0, -HALF_W, 2.0 * HALF_W + 0.08, 0.08),
        (0.0, HALF_W, 2.0 * HALF_W + 0.08, 0.08),
        (-HALF_W, 0.0, 0.08, 2.0 * HALF_W),
        (HALF_W, 0.0, 0.08, 2.0 * HALF_W),
    ] {
        commands.spawn((
            Mesh3d(quad.clone()),
            MeshMaterial3d(frame.clone()),
            Transform::from_xyz(x, 0.04, z)
                .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2))
                .with_scale(Vec3::new(w, h, 1.0)),
        ));
    }
    // la superficie, vista desde abajo
    let surface = materials.add(CardMaterial::surface(
        images.add(textures::soft_dot()),
        Color::srgb(lk.sky[0], lk.sky[1], lk.sky[2]),
    ));
    if !flags.has("no_surface") {
        commands.spawn((
            Mesh3d(meshes.add(Plane3d::default().mesh().size(190.0, 190.0))),
            MeshMaterial3d(surface.clone()),
            Transform::from_xyz(0.0, HEIGHT, 0.0),
            Submerged,
        ));
    }
    // el suelo: una sola malla esculpible que llega hasta el fondo
    if flags.has("no_terrain") {
        commands.spawn((
            Mesh3d(meshes.add(Plane3d::default().mesh().size(190.0, 190.0))),
            MeshMaterial3d(sand.clone()),
            Transform::from_xyz(0.0, 0.0, -6.0),
        ));
    } else {
        commands.spawn((
            Mesh3d(meshes.add(crate::terrain::build_mesh(&lienzo_core::Scene::new(0)))),
            MeshMaterial3d(sand.clone()),
            Transform::IDENTITY,
            crate::terrain::Terrain,
        ));
    }

    // rayos de luz desde la superficie
    let ray_tex = images.add(textures::ray());
    let mut ray_mat = CardMaterial::additive(ray_tex.clone(), 0.16);
    ray_mat.extra.z = -1.0;
    let ray = materials.add(ray_mat);
    let mut seed = 17u32;
    let mut rnd = move || {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        (seed >> 8) as f32 / (1u32 << 24) as f32
    };
    // Rayos de luz: nacen en la superficie, siempre de cara a la cámara, y se
    // funden por los lados y por abajo. Nunca se ven de canto ni cortados.
    for _ in 0..(if flags.has("no_rays") { 0 } else { 12 }) {
        let x = -12.0 + 24.0 * rnd();
        let z = -14.0 + 14.0 * rnd();
        let w = 1.0 + 2.5 * rnd();
        // todos se inclinan hacia el mismo lado: un solo sol
        let tilt = 0.14 + 0.10 * rnd();
        let h = 14.0;
        commands.spawn((
            Mesh3d(quad.clone()),
            MeshMaterial3d(ray.clone()),
            Transform::from_xyz(x, HEIGHT - h * 0.5, z).with_scale(Vec3::new(w, h, 1.0)),
            Ray {
                tilt,
                phase: rnd() * 6.28,
                width: w,
                x,
                z,
                drawn: false,
            },
            Submerged,
        ));
    }

    // partículas en suspensión
    let dot = materials.add(CardMaterial::additive(
        images.add(textures::soft_dot()),
        0.35,
    ));
    for _ in 0..350 {
        let s = 0.03 + 0.07 * rnd();
        commands.spawn((
            Mesh3d(quad.clone()),
            MeshMaterial3d(dot.clone()),
            Transform::from_xyz(
                -10.0 + 20.0 * rnd(),
                0.2 + 7.0 * rnd(),
                -12.0 + 18.0 * rnd(),
            )
            .with_scale(Vec3::splat(s)),
            Particle {
                speed: 0.05 + 0.12 * rnd(),
                phase: rnd() * 6.28,
            },
            Billboard,
            Submerged,
        ));
    }
    commands.insert_resource(Common {
        quad,
        dot,
        blob: images.add(textures::blob()),
        glow_tex: images.add(textures::soft_dot()),
        ray_tex,
        rainbow_tex: images.add(textures::rainbow()),
        backdrop,
        sand,
        surface,
        ray,
        frame,
    });
}

/// Sensación de estar bajo el agua: la lente respira y se ladea con un oleaje
/// lento, más cuanto más clara es el agua y más cerca de la superficie está la
/// cámara; en la vista de dibujo, casi nada (es un papel).
pub fn refraction(
    time: Res<Time>,
    obra: Res<crate::Obra>,
    st: Res<crate::draw::DrawState>,
    mut q: Query<
        (
            &Transform,
            &mut bevy::post_process::effect_stack::LensDistortion,
            &mut bevy::post_process::effect_stack::ChromaticAberration,
        ),
        With<Camera3d>,
    >,
) {
    let Ok((tf, mut lens, mut chroma)) = q.single_mut() else {
        return; // en Android no hay efectos
    };
    let t = time.elapsed_secs();
    let clarity = 1.0 - crate::water::look(obra.scene.water).fog_density / 0.05;
    let near_surface = ((tf.translation.y - 1.0) / (HEIGHT - 1.0)).clamp(0.0, 1.0);
    let drawing = if st.view == crate::draw::View::Dibujo {
        0.25
    } else {
        1.0
    };
    let k = (0.35 + 0.65 * clarity.clamp(0.0, 1.0)) * (0.5 + 0.5 * near_surface) * drawing;
    let w1 = (t * 0.23).sin();
    let w2 = (t * 0.17 + 1.3).sin();
    lens.intensity = 0.035 * k * (0.6 + 0.4 * w1);
    lens.center = Vec2::new(0.5 + 0.03 * k * w2, 0.5 + 0.02 * k * (t * 0.29 + 0.4).sin());
    lens.multiplier = Vec2::new(1.0 + 0.15 * k * w2, 1.0 - 0.15 * k * w1);
    chroma.intensity = 0.0025 + 0.004 * k;
}

/// Los rayos oscilan despacio, como la luz que atraviesa un oleaje lento.
pub fn sway_rays(
    time: Res<Time>,
    st: Res<crate::draw::DrawState>,
    mut fade: Local<f32>,
    cam: Query<&Transform, (With<Camera3d>, Without<Ray>)>,
    mut q: Query<(&mut Transform, &Ray)>,
) {
    let Ok(cam) = cam.single() else { return };
    let t = time.elapsed_secs();
    // en la vista de dibujo (el papel) no hay rayos: se funden al cambiar
    let goal = if st.view == crate::draw::View::Dibujo { 0.0 } else { 1.0 };
    *fade += (goal - *fade) * (1.0 - (-2.5 * time.delta_secs()).exp());
    for (mut tf, r) in q.iter_mut() {
        // de cara a la cámara (solo giro en Y), con su inclinación propia que
        // respira con el oleaje
        let to_cam = cam.translation - tf.translation;
        let yaw = to_cam.x.atan2(to_cam.z);
        let a = r.tilt + 0.05 * (t * 0.17 + r.phase).sin();
        tf.rotation = Quat::from_euler(EulerRot::YXZ, yaw, 0.0, a);
        // el ancho parpadea despacio: la luz que concentra el oleaje
        let flicker = 0.75 + 0.35 * (0.5 + 0.5 * (t * 0.43 + r.phase).sin()) * (0.5 + 0.5 * (t * 0.29 + r.phase * 1.7).cos());
        let f = if r.drawn { 1.0 } else { (*fade).max(0.001) };
        tf.scale.x = r.width * flicker * f;
        tf.translation.x = r.x + 0.25 * (t * 0.11 + r.phase).sin();
        tf.translation.z = r.z;
    }
}

pub fn drift_particles(time: Res<Time>, mut q: Query<(&mut Transform, &Particle)>) {
    let t = time.elapsed_secs();
    for (mut tf, p) in q.iter_mut() {
        tf.translation.y += p.speed * time.delta_secs();
        tf.translation.x += (t * 0.3 + p.phase).sin() * 0.02 * time.delta_secs();
        if tf.translation.y > 7.5 {
            tf.translation.y = 0.2;
        }
    }
}

/// Las cartas miran a la cámara girando solo sobre Y: siempre de pie.
/// Las cartas miran a la cámara. Buceando giran solo sobre Y (siempre de
/// pie); desde arriba se tumban hacia la cámara para leerse como dibujos en
/// el papel. Entre ambas, se mezcla según la inclinación: la transición es
/// continua.
pub fn billboards(
    cam: Query<(&Transform, &Orbit), Without<Billboard>>,
    mut q: Query<&mut Transform, With<Billboard>>,
) {
    let Ok((cam, orbit)) = cam.single() else {
        return;
    };
    // la inclinación se mide en la cámara real (en contemplación la órbita
    // no se actualiza y las cartas quedaban tumbadas)
    let _ = orbit;
    let pitch = (-cam.forward().y).clamp(-1.0, 1.0).asin();
    let f = ((pitch - 0.55) / (crate::orbit::TOP_PITCH - 0.55)).clamp(0.0, 1.0);
    let f = f * f * (3.0 - 2.0 * f);
    for mut tf in q.iter_mut() {
        let to_cam = cam.translation - tf.translation;
        let yaw = to_cam.x.atan2(to_cam.z);
        let keep = tf.rotation.to_euler(EulerRot::YXZ).2;
        let upright = Quat::from_euler(EulerRot::YXZ, yaw, 0.0, keep);
        // tumbada sobre el papel (mirando al cielo), girada hacia la cámara en
        // el plano, con el balanceo propio de la carta: desde arriba se lee
        // como un dibujo pegado en el lienzo
        let flat = Quat::from_euler(EulerRot::YXZ, yaw, -std::f32::consts::FRAC_PI_2, keep);
        tf.rotation = upright.slerp(flat, f);
    }
}
