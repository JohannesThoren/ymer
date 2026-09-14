//! Kör demo-scenen med gameplay skriven i TypeScript istället för Rust.
//! Motorn kompilerar .ts-filen själv (oxc) och kör den i QuickJS inuti wasm.
//!
//!     cargo run -p ymer-runtime --example typescript

use std::time::{Duration, Instant};

use ymer_core::Script;
use ymer_render::Renderer;
use ymer_runtime::demo;
use ymer_runtime::prelude::*;
use ymer_script::{ScriptRuntime, run_script_system};

const WIDTH: u32 = 1280;
const HEIGHT: u32 = 720;
const FRAMES: u32 = 75;
const SCRIPT: &str = "spin.ts";

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    let mut renderer = pollster::block_on(Renderer::new_offscreen(WIDTH, HEIGHT))?;
    let assets = ymer_render::Assets::new(&mut renderer);

    let mut world = World::new();
    world.insert_resource(Time::new());
    demo::setup(&mut world);

    // Byt ut Rust-systemet mot skriptet: bort med Spin, på med Script.
    let spinners: Vec<Entity> = {
        let mut query = world.query_filtered::<Entity, With<demo::Spin>>();
        query.iter(&world).collect()
    };
    for entity in &spinners {
        world.entity_mut(*entity).remove::<demo::Spin>();
        world.entity_mut(*entity).insert(Script::new(SCRIPT));
    }
    println!("{} entiteter kör nu {SCRIPT}", spinners.len());

    // Hela kedjan: .ts -> oxc strippar typerna -> QuickJS i wasm.
    let wasm = std::fs::read("assets/script_host.wasm")?;
    let source = std::fs::read_to_string(format!("assets/scripts/{SCRIPT}"))?;

    let compile_start = Instant::now();
    let mut runtime = ScriptRuntime::new(&wasm)?;
    runtime.load_typescript(&source, SCRIPT)?;
    println!("kompilerade och laddade på {:?}", compile_start.elapsed());

    let dt = 1.0 / 60.0;
    let run_start = Instant::now();
    for _ in 0..FRAMES {
        world
            .resource_mut::<Time>()
            .advance_by(Duration::from_micros(16_667));
        run_script_system(&mut world, &mut runtime, SCRIPT, dt)?;
        propagate_transforms(&mut world);
    }
    let elapsed = run_start.elapsed();
    println!(
        "{FRAMES} frames på {:?} ({:.3} ms/frame för {} entiteter)",
        elapsed,
        elapsed.as_secs_f64() * 1000.0 / FRAMES as f64,
        spinners.len()
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
