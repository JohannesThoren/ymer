//! Fysik: gravitation, kollisionsupplösning och grounded-flaggan.
//!
//!     cargo run -p ymer-runtime --example physics

use std::time::Duration;

use bevy_ecs::prelude::*;
use ymer_core::{
    Camera, Collider, Color, ConsoleLog, EntityName, GlobalTransform, Gravity, Input, MeshInstance,
    RigidBody, Time, Transform, Vec3, propagate_transforms, step_physics,
};
use ymer_render::{Assets, Renderer};
use ymer_runtime::{Update, build_render_list};

const WIDTH: u32 = 1280;
const HEIGHT: u32 = 720;

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    let mut renderer = pollster::block_on(Renderer::new_offscreen(WIDTH, HEIGHT))?;
    let assets = Assets::new(&mut renderer);

    let mut world = World::new();
    world.insert_resource(Time::new());
    world.insert_resource(Input::default());
    world.insert_resource(ConsoleLog::default());
    world.insert_resource(Gravity::default());

    world.spawn((
        EntityName::new("Kamera"),
        Transform::from_xyz(0.0, 4.0, 16.0).looking_at(Vec3::new(0.0, 1.5, 0.0), Vec3::Y),
        GlobalTransform::default(),
        Camera::default(),
    ));

    // Mark: statisk geometri, alltså Collider utan RigidBody.
    // Meshen skalas efter kollideraren – kuben är 1x1x1, så skalan blir
    // halva storleken gånger två. Annars visar bilden inte det som
    // faktiskt kolliderar.
    let ground_collider = Collider::new(12.0, 1.0, 4.0);
    let ground = Transform::from_xyz(0.0, -1.0, 0.0).with_scale(ground_collider.half_extents * 2.0);
    world.spawn((
        EntityName::new("Mark"),
        ground,
        GlobalTransform(ground.matrix()),
        MeshInstance::new(ymer_core::BUILTIN_CUBE, Color::rgb(0.25, 0.28, 0.32)),
        ground_collider,
    ));

    // Trappsteg i olika höjd – kroppar ska landa på rätt nivå.
    for (index, (x, height)) in [(-6.0, 0.5), (-2.0, 1.5), (2.0, 2.5)]
        .into_iter()
        .enumerate()
    {
        let collider = Collider::new(1.5, height, 2.0);
        let step =
            Transform::from_xyz(x, height - 1.0, 0.0).with_scale(collider.half_extents * 2.0);
        world.spawn((
            EntityName::new(format!("Trappsteg {index}")),
            step,
            GlobalTransform(step.matrix()),
            MeshInstance::new(ymer_core::BUILTIN_CUBE, Color::rgb(0.35, 0.4, 0.45)),
            collider,
        ));
    }

    // Fallande kroppar, en över varje trappsteg och en vid sidan.
    for (index, x) in [-6.0, -2.0, 2.0, 6.0].into_iter().enumerate() {
        world.spawn((
            EntityName::new(format!("Låda {index}")),
            Transform::from_xyz(x, 8.0, 0.0),
            GlobalTransform::default(),
            MeshInstance::new(
                ymer_core::BUILTIN_CUBE,
                Color::rgb(0.9, 0.45 + index as f32 * 0.1, 0.3),
            ),
            Collider::cube(0.5),
            RigidBody::new(),
        ));
    }

    let mut schedule = Schedule::new(Update);
    schedule.add_systems((step_physics, propagate_transforms).chain());

    // Två sekunder – gott om tid att falla och komma till vila.
    for _ in 0..120 {
        world
            .resource_mut::<Time>()
            .advance_by(Duration::from_micros(16_667));
        schedule.run(&mut world);
    }

    let mut query = world.query::<(&EntityName, &Transform, &RigidBody)>();
    let mut resting: Vec<(String, f32, bool)> = query
        .iter(&world)
        .map(|(name, t, body)| (name.0.clone(), t.translation.y, body.grounded))
        .collect();
    resting.sort_by(|a, b| a.0.cmp(&b.0));

    for (name, y, grounded) in &resting {
        println!("{name:10} y = {y:5.2}  grounded = {grounded}");
    }
    anyhow::ensure!(
        resting.iter().all(|(_, _, g)| *g),
        "någon kropp kom aldrig till vila"
    );

    let list = build_render_list(&mut world, renderer.aspect_ratio(), &assets);
    renderer.render(&list)?;
    std::fs::write(
        std::env::temp_dir().join("frame.raw"),
        renderer.capture_rgba()?,
    )?;
    println!("skrev {}", std::env::temp_dir().join("frame.raw").display());
    Ok(())
}
