//! Hot reload: skriptet byts ut mitt under körning, scenen står kvar.
//!
//! Renderar två bilder – före och efter omladdningen – med exakt samma
//! värld emellan. Skillnaden kommer enbart från den nya .ts-filen.
//!
//!     cargo run -p ymer-editor --example hot_reload

use std::time::Duration;

use ymer_core::{Color, EntityName, GlobalTransform, MeshInstance, Script, Vec3};
use ymer_editor::PlayMode;
use ymer_render::Renderer;
use ymer_runtime::prelude::*;
use ymer_scene::{TypeRegistry, register_builtin_types};

const WIDTH: u32 = 1280;
const HEIGHT: u32 = 720;
const SCRIPT: &str = "hot_demo.ts";

/// Version 1: lugn våg, kalla färger.
const VERSION_1: &str = r#"
let elapsed = 0;

function update(dt: number, entities: Entity[]): void {
  elapsed += dt;
  for (const e of entities) {
    const t = e.Transform;
    if (!t) continue;
    t.translation[1] = 1.0 + Math.sin(elapsed * 2 - e.i * 0.5) * 0.6;
    engine.rotate(t.rotation, 0, 1, 0, dt * 0.6);
    const mesh = e.MeshInstance;
    if (mesh) {
      mesh.color.r = 0.2;
      mesh.color.g = 0.45 + e.i * 0.05;
      mesh.color.b = 0.95;
    }
  }
}
"#;

/// Version 2: samma scen, helt annan känsla.
const VERSION_2: &str = r#"
let elapsed = 0;

function update(dt: number, entities: Entity[]): void {
  elapsed += dt;
  for (const e of entities) {
    const t = e.Transform;
    if (!t) continue;
    // Spiral istället för våg.
    const phase = elapsed * 3 + e.i * 0.9;
    t.translation[1] = 2.2 + Math.sin(phase) * 1.6;
    t.scale[0] = t.scale[1] = t.scale[2] = 0.7 + Math.cos(phase) * 0.35;
    engine.rotate(t.rotation, 0.4, 1, 0.2, dt * 3.0);
    const mesh = e.MeshInstance;
    if (mesh) {
      mesh.color.r = 0.98;
      mesh.color.g = 0.35 + Math.sin(phase) * 0.3;
      mesh.color.b = 0.15;
    }
  }
}
"#;

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    let path = std::path::Path::new("assets/scripts").join(SCRIPT);
    std::fs::write(&path, VERSION_1)?;

    let mut renderer = pollster::block_on(Renderer::new_offscreen(WIDTH, HEIGHT))?;
    let assets = ymer_render::Assets::new(&mut renderer);
    let mut registry = TypeRegistry::new();
    register_builtin_types(&mut registry);

    let cube = ymer_core::BUILTIN_CUBE;
    let ground = ymer_core::BUILTIN_PLANE;

    let mut world = World::new();
    world.insert_resource(Time::new());
    world.spawn((
        EntityName::new("Main Camera"),
        Transform::from_xyz(0.0, 5.0, 12.0).looking_at(Vec3::new(0.0, 1.5, 0.0), Vec3::Y),
        GlobalTransform::default(),
        Camera::default(),
    ));
    world.spawn((
        EntityName::new("Ground"),
        Transform::from_xyz(0.0, -0.5, 0.0),
        GlobalTransform::default(),
        MeshInstance::new(ground, Color::rgb(0.06, 0.065, 0.085)),
    ));
    for index in 0..7 {
        world.spawn((
            EntityName::new(format!("Kub {index}")),
            Transform::from_xyz(-6.0 + index as f32 * 2.0, 0.0, 0.0),
            GlobalTransform::default(),
            MeshInstance::new(cube, Color::rgb(0.5, 0.5, 0.5)),
            Script::new(SCRIPT),
        ));
    }

    let mut play = PlayMode::new("assets/script_host.wasm", "assets/scripts")?;
    play.start(&mut world, &registry)?;

    let run = |play: &mut PlayMode, world: &mut World, frames: u32| -> anyhow::Result<()> {
        for _ in 0..frames {
            world
                .resource_mut::<Time>()
                .advance_by(Duration::from_micros(16_667));
            play.tick(world, &registry, 1.0 / 60.0)?;
            propagate_transforms(world);
        }
        Ok(())
    };

    run(&mut play, &mut world, 45)?;
    let mut list = build_render_list(&mut world, renderer.aspect_ratio(), &assets);
    renderer.render(&list)?;
    std::fs::write(
        std::env::temp_dir().join("before.raw"),
        renderer.capture_rgba()?,
    )?;
    println!("bild 1 renderad med version 1");

    // Spara om filen – precis vad som händer när du trycker ctrl+s.
    std::fs::write(&path, VERSION_2)?;
    let reloaded = play.poll_reloads(true);
    for result in &reloaded {
        match result {
            Ok(name) => println!("laddade om {name} utan att röra scenen"),
            Err(err) => println!("omladdning misslyckades: {err}"),
        }
    }
    anyhow::ensure!(!reloaded.is_empty(), "ingen omladdning upptäcktes");

    run(&mut play, &mut world, 45)?;
    list = build_render_list(&mut world, renderer.aspect_ratio(), &assets);
    renderer.render(&list)?;
    std::fs::write(
        std::env::temp_dir().join("after.raw"),
        renderer.capture_rgba()?,
    )?;
    println!("bild 2 renderad med version 2");

    std::fs::remove_file(&path)?;
    Ok(())
}
