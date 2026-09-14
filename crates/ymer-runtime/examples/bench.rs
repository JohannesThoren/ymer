//! Mäter var tiden faktiskt går i en stor scen.
//!
//!     cargo run --release -p ymer-runtime --example bench

use std::time::{Duration, Instant};

use ymer_core::{Color, EntityName, GlobalTransform, Input, MeshInstance, Script, Time, Vec3};
use ymer_render::{Assets, Renderer};
use ymer_runtime::prelude::*;
use ymer_scene::{Scene, TypeRegistry, clear_scene, register_builtin_types};
use ymer_script::{ScriptRuntime, run_script_general, run_script_system};

const ENTITIES: usize = 5000;
const FRAMES: u32 = 60;
const WIDTH: u32 = 640;
const HEIGHT: u32 = 360;

fn bench(label: &str, iterations: u32, mut body: impl FnMut()) -> Duration {
    let start = Instant::now();
    for _ in 0..iterations {
        body();
    }
    let total = start.elapsed();
    let each = total / iterations;
    println!("{label:<34} {:>9.3} ms/frame", each.as_secs_f64() * 1000.0);
    each
}

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    let mut renderer = pollster::block_on(Renderer::new_offscreen(WIDTH, HEIGHT))?;
    let assets = Assets::new(&mut renderer);

    let mut registry = TypeRegistry::new();
    register_builtin_types(&mut registry);

    let mut world = World::new();
    world.insert_resource(Time::new());
    world.insert_resource(Input::default());
    world.insert_resource(ymer_core::ConsoleLog::default());

    world.spawn((
        EntityName::new("Main Camera"),
        Transform::from_xyz(0.0, 60.0, 90.0).looking_at(Vec3::ZERO, Vec3::Y),
        GlobalTransform::default(),
        Camera::default(),
    ));

    let side = (ENTITIES as f32).sqrt() as i32;
    for index in 0..ENTITIES {
        let x = (index as i32 % side) as f32 - side as f32 * 0.5;
        let z = (index as i32 / side) as f32 - side as f32 * 0.5;
        world.spawn((
            EntityName::new(format!("Kub {index}")),
            Transform::from_xyz(x * 1.5, 0.0, z * 1.5),
            GlobalTransform::default(),
            MeshInstance::new(ymer_core::BUILTIN_CUBE, Color::rgb(0.5, 0.6, 0.9)),
            Script::new("spin.ts"),
        ));
    }
    println!("{} entiteter\n", world.iter_entities().count());

    bench("propagate_transforms", FRAMES, || {
        propagate_transforms(&mut world);
    });

    bench("build_render_list", FRAMES, || {
        let list = build_render_list(&mut world, renderer.aspect_ratio(), &assets);
        std::hint::black_box(list);
    });

    let list = build_render_list(&mut world, renderer.aspect_ratio(), &assets);
    bench("render (sortering + draw calls)", FRAMES, || {
        renderer.render(&list).expect("render");
    });

    // Det editorn gör varje frame för hierarkin och vid varje scenrensning.
    bench("registerskanning (hierarki)", FRAMES, || {
        let entities: Vec<Entity> = world.iter_entities().map(|e| e.id()).collect();
        let count = entities
            .iter()
            .filter(|entity| {
                registry
                    .iter()
                    .any(|component| component.read(&world, **entity).is_some())
            })
            .count();
        std::hint::black_box(count);
    });

    bench("Scene::from_world", 5, || {
        let scene = Scene::from_world(&mut world, &registry);
        std::hint::black_box(scene);
    });

    // Skriptvägarna.
    let wasm = std::fs::read("assets/script_host.wasm")?;
    let source = std::fs::read_to_string("assets/scripts/spin.ts")?;
    let mut runtime = ScriptRuntime::new(&wasm)?;
    runtime.load_typescript(&source, "spin.ts")?;

    bench("run_script_system (platt buffert)", 20, || {
        run_script_system(&mut world, &mut runtime, "spin.ts", 1.0 / 60.0).expect("script");
    });

    let mut general = ScriptRuntime::new(&wasm)?;
    general.load_typescript(
        &std::fs::read_to_string("assets/scripts/paint.ts")?,
        "paint.ts",
    )?;
    {
        let entities: Vec<Entity> = {
            let mut query = world.query_filtered::<Entity, With<Script>>();
            query.iter(&world).collect()
        };
        for entity in entities {
            world.entity_mut(entity).insert(Script::new("paint.ts"));
        }
    }
    bench("run_script_general (JSON)", 5, || {
        run_script_general(&mut world, &mut general, &registry, "paint.ts", 1.0 / 60.0)
            .expect("script");
    });

    let removed = clear_scene(&mut world, &registry);
    println!("\nclear_scene tog bort {removed} entiteter");
    Ok(())
}
