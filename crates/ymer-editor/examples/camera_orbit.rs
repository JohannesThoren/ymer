//! Verifierar editorns vykamera: att den roterar vyn utan att röra
//! scenens kamera, och att borttagning tar barnen med sig.
//!
//!     cargo run -p ymer-editor --example camera_orbit

use bevy_ecs::prelude::*;
use ymer_core::{
    Camera, Color, EntityName, GlobalTransform, MeshInstance, Transform, Vec3, propagate_transforms,
};
use ymer_editor::view::EditorCamera;
use ymer_render::{Assets, Renderer};
use ymer_runtime::build_render_list_with_view;
use ymer_scene::{TypeRegistry, register_builtin_types};

const WIDTH: u32 = 640;
const HEIGHT: u32 = 360;

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    let mut renderer = pollster::block_on(Renderer::new_offscreen(WIDTH, HEIGHT))?;
    let assets = Assets::new(&mut renderer);

    let mut registry = TypeRegistry::new();
    register_builtin_types(&mut registry);

    let mut world = World::new();

    // Scenens kamera – den som INTE ska röras av vyrotationen.
    let scene_camera_transform = Transform::from_xyz(0.0, 3.0, 9.0);
    let scene_camera = world
        .spawn((
            EntityName::new("Scenkamera"),
            scene_camera_transform,
            GlobalTransform::default(),
            Camera::default(),
        ))
        .id();

    // Förälder med två barn, för att testa att borttagning tar med sig barn.
    let parent = world
        .spawn((
            EntityName::new("Förälder"),
            Transform::from_xyz(0.0, 0.5, 0.0),
            GlobalTransform::default(),
            MeshInstance::new(ymer_core::BUILTIN_CUBE, Color::rgb(0.9, 0.5, 0.3)),
        ))
        .id();
    for i in 0..2 {
        world.spawn((
            EntityName::new(format!("Barn {i}")),
            Transform::from_xyz(-1.0 + i as f32 * 2.0, 1.2, 0.0),
            GlobalTransform::default(),
            MeshInstance::new(ymer_core::BUILTIN_CUBE, Color::rgb(0.4, 0.8, 0.9)),
            ChildOf(parent),
        ));
    }
    world.spawn((
        EntityName::new("Granne"),
        Transform::from_xyz(3.0, 0.5, 0.0),
        GlobalTransform::default(),
        MeshInstance::new(ymer_core::BUILTIN_CUBE, Color::rgb(0.6, 0.6, 0.7)),
    ));
    propagate_transforms(&mut world);

    let before = world.iter_entities().count();

    // --- rotera vyn ------------------------------------------------------
    let mut camera = EditorCamera::default();
    let start = camera.position();
    camera.orbit(300.0, 60.0);
    camera.zoom(1.5);
    let after_orbit = camera.position();

    anyhow::ensure!(start != after_orbit, "orbit flyttade inte kameran");

    // Scenens kamera ska vara orörd – det är hela poängen med en egen vy.
    let scene_camera_now = world.get::<Transform>(scene_camera).unwrap();
    anyhow::ensure!(
        scene_camera_now.translation == scene_camera_transform.translation,
        "vyrotationen rörde scenens kamera: {:?}",
        scene_camera_now.translation
    );
    println!("vyn roterad: {start:?} -> {after_orbit:?}");
    println!("scenens kamera orörd: {:?}", scene_camera_now.translation);

    // Rendera genom editorvyn.
    let aspect = renderer.aspect_ratio();
    let list = build_render_list_with_view(
        &mut world,
        aspect,
        &assets,
        Some((camera.view_proj(aspect), camera.position())),
    );
    renderer.render(&list)?;
    std::fs::write(
        std::env::temp_dir().join("orbit.raw"),
        renderer.capture_rgba()?,
    )?;

    // --- ta bort föräldern ----------------------------------------------
    world.try_despawn(parent)?;
    let after = world.iter_entities().count();
    println!("entiteter före: {before}, efter borttagning: {after}");
    anyhow::ensure!(
        before - after == 3,
        "borttagning tog {} entiteter, väntade 3 (förälder + två barn)",
        before - after
    );

    // F-tangenten: rama in något.
    camera.focus_on(Vec3::new(3.0, 0.5, 0.0), 0.87);
    println!("fokus: {:?}, avstånd {:.2}", camera.focus, camera.distance);

    println!("\nkontroller: OK");
    Ok(())
}
