//! Gameplay med scenfrågor: spelaren plockar mynt via överlappstest och
//! markerar nästa mynt med en raycast. Allt i TypeScript.
//!
//!     cargo run -p ymer-runtime --example typescript_queries

use std::time::Duration;

use ymer_core::{Color, EntityName, GlobalTransform, Input, MeshInstance, Script, Vec3};
use ymer_render::Renderer;
use ymer_runtime::prelude::*;
use ymer_scene::{TypeRegistry, register_builtin_types};
use ymer_script::{ScriptRuntime, run_script_general};

const WIDTH: u32 = 1280;
const HEIGHT: u32 = 720;
const SCRIPT: &str = "collector.ts";

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    let mut renderer = pollster::block_on(Renderer::new_offscreen(WIDTH, HEIGHT))?;
    let assets = ymer_render::Assets::new(&mut renderer);

    let mut registry = TypeRegistry::new();
    register_builtin_types(&mut registry);

    let cube = ymer_core::BUILTIN_CUBE;
    let ground = ymer_core::BUILTIN_PLANE;

    let mut world = World::new();
    world.insert_resource(Time::new());
    world.insert_resource(Input::default());
    world.insert_resource(ymer_core::ConsoleLog::default());

    world.spawn((
        EntityName::new("Main Camera"),
        Transform::from_xyz(0.0, 4.5, 9.0).looking_at(Vec3::new(0.0, 1.0, -1.0), Vec3::Y),
        GlobalTransform::default(),
        Camera::default(),
    ));
    world.spawn((
        EntityName::new("Ground"),
        Transform::from_xyz(0.0, -0.5, 0.0),
        GlobalTransform::default(),
        MeshInstance::new(ground, Color::rgb(0.06, 0.065, 0.085)),
    ));
    world.spawn((
        EntityName::new("Player"),
        Transform::from_xyz(-5.0, 0.0, 2.0),
        GlobalTransform::default(),
        MeshInstance::new(cube, Color::rgb(0.35, 0.85, 0.95)),
        Script::new(SCRIPT),
    ));

    // En rad mynt som skriptet får leta upp själv.
    for index in 0..8 {
        world.spawn((
            EntityName::new(format!("Mynt {index}")),
            Transform::from_xyz(-4.0 + index as f32 * 1.6, 0.35, 2.0).with_scale(Vec3::splat(0.55)),
            GlobalTransform::default(),
            MeshInstance::new(cube, Color::rgb(0.95, 0.8, 0.25)),
        ));
    }

    let wasm = std::fs::read("assets/script_host.wasm")?;
    let source = std::fs::read_to_string(format!("assets/scripts/{SCRIPT}"))?;
    let mut runtime = ScriptRuntime::new(&wasm)?;
    runtime.load_typescript(&source, SCRIPT)?;

    // Ett inspelat inputmakro: gå höger, hoppa, fortsätt, gå framåt.
    let dt = 1.0 / 60.0;
    for frame in 0..120u32 {
        {
            let mut input = world.resource_mut::<Input>();
            match frame {
                0 => input.press("KeyD"),
                // Stanna halvvägs så att några mynt återstår i bilden.
                60 => input.release("KeyD"),
                // Hoppet sker sent, så stillbilden fångar spelaren i luften.
                108 => input.press("Space"),
                109 => input.release("Space"),
                _ => {}
            }
        }

        world
            .resource_mut::<Time>()
            .advance_by(Duration::from_micros(16_667));
        run_script_general(&mut world, &mut runtime, &registry, SCRIPT, dt)?;
        propagate_transforms(&mut world);
        world.resource_mut::<Input>().end_frame();
    }

    let remaining = {
        let mut query = world.query::<&EntityName>();
        query
            .iter(&world)
            .filter(|name| name.0.starts_with("Mynt"))
            .count()
    };
    println!("mynt kvar: {remaining} av 8");

    let player = {
        let mut query = world.query::<(&EntityName, &Transform)>();
        query
            .iter(&world)
            .find(|(name, _)| name.0 == "Player")
            .map(|(_, transform)| transform.translation)
    };
    println!("spelaren hamnade på {player:?}");
    println!("entiteter i världen: {}", world.iter_entities().count());

    let list = build_render_list(&mut world, renderer.aspect_ratio(), &assets);
    println!("ritar {} objekt", list.items.len());
    renderer.render(&list)?;
    std::fs::write(
        std::env::temp_dir().join("frame.raw"),
        renderer.capture_rgba()?,
    )?;
    Ok(())
}
