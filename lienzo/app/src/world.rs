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

#[derive(Component)]
pub struct Ray {
    tilt: f32,
    phase: f32,
}

#[derive(Resource)]
pub struct Common {
    pub quad: Handle<Mesh>,
    pub dot: Handle<CardMaterial>,
    pub backdrop: Handle<CardMaterial>,
    pub sand: Handle<CardMaterial>,
    pub ray: Handle<CardMaterial>,
}

pub fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<CardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 3.0, 15.0).looking_at(Vec3::new(0.0, 2.2, 0.0), Vec3::Y),
        Tonemapping::AcesFitted,
        Bloom {
            intensity: 0.18,
            ..Bloom::NATURAL
        },
        Orbit::default(),
    ));
    let lk = crate::water::look(lienzo_core::Water::Abismo);
    commands.insert_resource(ClearColor(lk.clear));

    let quad = meshes.add(Rectangle::new(1.0, 1.0));

    // fondo: una esfera que envuelve todo, vista desde dentro, sin niebla.
    // Gire a donde gire la cámara, siempre hay agua.
    let backdrop = materials.add(CardMaterial::backdrop(
        images.add(textures::backdrop(lk.top, lk.bottom)),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(90.0).mesh().uv(48, 24))),
        MeshMaterial3d(backdrop.clone()),
        // centrada a la altura de la mirada: el horizonte del fondo coincide
        // con el borde del suelo, y la niebla los funde.
        Transform::from_xyz(0.0, 2.0, 0.0),
    ));

    // suelo
    let sand = materials.add(CardMaterial::opaque(images.add(textures::sand(lk.sand))));
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(140.0, 140.0))),
        MeshMaterial3d(sand.clone()),
        Transform::from_xyz(0.0, 0.0, -6.0),
    ));

    // rayos de luz desde la superficie
    let ray = materials.add(CardMaterial::additive(images.add(textures::ray()), 0.55));
    let mut seed = 17u32;
    let mut rnd = move || {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        (seed >> 8) as f32 / (1u32 << 24) as f32
    };
    for _ in 0..9 {
        let x = -9.0 + 18.0 * rnd();
        let z = -12.0 + 8.0 * rnd();
        let w = 1.2 + 2.2 * rnd();
        let tilt = -0.25 + 0.35 * rnd();
        commands.spawn((
            Mesh3d(quad.clone()),
            MeshMaterial3d(ray.clone()),
            Transform::from_xyz(x, 9.0, z)
                .with_rotation(Quat::from_rotation_z(tilt))
                .with_scale(Vec3::new(w, 20.0, 1.0)),
            Ray {
                tilt,
                phase: rnd() * 6.28,
            },
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
        ));
    }
    commands.insert_resource(Common {
        quad,
        dot,
        backdrop,
        sand,
        ray,
    });
}

/// Los rayos oscilan despacio, como la luz que atraviesa un oleaje lento.
pub fn sway_rays(time: Res<Time>, mut q: Query<(&mut Transform, &Ray)>) {
    let t = time.elapsed_secs();
    for (mut tf, r) in q.iter_mut() {
        let a = r.tilt + 0.06 * (t * 0.17 + r.phase).sin();
        tf.rotation = Quat::from_rotation_z(a);
        tf.scale.x = tf.scale.x.signum() * tf.scale.x.abs().max(0.1);
        tf.translation.x += 0.02 * (t * 0.11 + r.phase).cos() * time.delta_secs();
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
pub fn billboards(
    cam: Query<&Transform, (With<Camera3d>, Without<Billboard>)>,
    mut q: Query<&mut Transform, With<Billboard>>,
) {
    let Ok(cam) = cam.single() else { return };
    for mut tf in q.iter_mut() {
        let to_cam = cam.translation - tf.translation;
        let yaw = to_cam.x.atan2(to_cam.z);
        let keep = tf.rotation.to_euler(EulerRot::YXZ).2;
        tf.rotation = Quat::from_euler(EulerRot::YXZ, yaw, 0.0, keep);
    }
}
