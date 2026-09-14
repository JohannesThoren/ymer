//! Bevisar att typregistret och scenformatet är förlustfria:
//! bygger demo-scenen, sparar den som RON, laddar den i en tom värld,
//! animerar båda världarna lika långt och jämför pixlarna.
//!
//!     cargo run -p ymer-runtime --example scene_roundtrip

use std::time::Duration;

use ymer_render::Renderer;
use ymer_runtime::prelude::*;
use ymer_runtime::{Update, demo, init_logging};
use ymer_scene::{Scene, TypeRegistry, register_builtin_types};

const WIDTH: u32 = 1280;
const HEIGHT: u32 = 720;
const FRAMES: u32 = 75;

fn main() -> anyhow::Result<()> {
    init_logging();

    let mut renderer = pollster::block_on(Renderer::new_offscreen(WIDTH, HEIGHT))?;
    let assets = ymer_render::Assets::new(&mut renderer);

    let mut registry = TypeRegistry::new();
    register_builtin_types(&mut registry);
    // Spelets egna komponenter registreras på exakt samma sätt.
    registry.register::<demo::Spin>("Spin");
    println!(
        "registrerade typer: {:?}",
        registry.names().collect::<Vec<_>>()
    );

    // --- värld A: byggd i kod -------------------------------------------
    let mut original = World::new();
    original.insert_resource(Time::new());
    demo::setup(&mut original);

    let scene = Scene::from_world(&mut original, &registry);
    let scene_path = std::env::temp_dir().join("demo.scene.ron");
    scene.save(&scene_path)?;
    println!(
        "sparade {} entiteter till {}",
        scene.entities.len(),
        scene_path.display()
    );

    // --- värld B: laddad från fil ---------------------------------------
    let loaded_scene = Scene::load(&scene_path)?;
    let mut loaded = World::new();
    loaded.insert_resource(Time::new());
    loaded_scene.spawn_into(&mut loaded, &registry)?;

    let pixels_original = simulate_and_render(&mut original, &mut renderer, &assets)?;
    let pixels_loaded = simulate_and_render(&mut loaded, &mut renderer, &assets)?;

    if pixels_original == pixels_loaded {
        println!("round-trip OK: bilderna är bitidentiska");
    } else {
        let differing = pixels_original
            .iter()
            .zip(&pixels_loaded)
            .filter(|(a, b)| a != b)
            .count();
        anyhow::bail!("round-trip trasig: {differing} bytes skiljer");
    }

    // --- redigering utan att känna till typerna ---------------------------
    // Precis det editorn gör: läs komponenten som RON-text, ändra texten,
    // skriv tillbaka. Ingen rad här nämner Transform eller MeshInstance
    // som Rust-typer – allt går genom registret på namn.
    let edits = edit_through_registry(&mut loaded, &registry)?;
    println!("redigerade {edits} komponenter enbart via registret");

    let pixels_edited = simulate_and_render(&mut loaded, &mut renderer, &assets)?;
    std::fs::write(std::env::temp_dir().join("frame.raw"), &pixels_edited)?;
    println!("skrev /tmp/frame.raw ({} bytes)", pixels_edited.len());

    Ok(())
}

/// Läser, textredigerar och skriver tillbaka komponenter via typregistret.
fn edit_through_registry(world: &mut World, registry: &TypeRegistry) -> anyhow::Result<usize> {
    let name_type = registry.get("Name").expect("Name är registrerad");
    let transform_type = registry.get("Transform").expect("Transform är registrerad");
    let mesh_type = registry
        .get("MeshInstance")
        .expect("MeshInstance är registrerad");

    let entities: Vec<Entity> = world.iter_entities().map(|entity| entity.id()).collect();
    let mut edited = 0;

    for entity in entities {
        let Some(name) = name_type.read(world, entity) else {
            continue;
        };
        let name = name.get_ron().to_string();

        if name.contains("Cube") {
            // Varannan kub i rutnätet blir större.
            let coords: Vec<i32> = name
                .trim_matches(|c: char| !c.is_ascii_digit() && c != '-' && c != ',')
                .split(',')
                .filter_map(|part| part.trim().parse().ok())
                .collect();
            if coords.len() == 2
                && (coords[0] + coords[1]).rem_euclid(2) == 0
                && let Some(transform) = transform_type.read(world, entity)
            {
                let patched = transform
                    .get_ron()
                    .replace("scale:(1.0,1.0,1.0)", "scale:(1.6,1.6,1.6)");
                let raw = ron::value::RawValue::from_boxed_ron(patched.into_boxed_str())
                    .map_err(|err| anyhow::anyhow!("{err}"))?;
                transform_type.write(world, entity, &raw)?;
                edited += 1;
            }
        }

        if name.contains("Ground")
            && let Some(mesh) = mesh_type.read(world, entity)
        {
            let patched = mesh.get_ron().replace(
                "color:(r:0.05,g:0.055,b:0.075,a:1.0)",
                "color:(r:0.16,g:0.11,b:0.09,a:1.0)",
            );
            let raw = ron::value::RawValue::from_boxed_ron(patched.into_boxed_str())
                .map_err(|err| anyhow::anyhow!("{err}"))?;
            mesh_type.write(world, entity, &raw)?;
            edited += 1;
        }
    }

    Ok(edited)
}

fn simulate_and_render(
    world: &mut World,
    renderer: &mut Renderer,
    assets: &ymer_render::Assets,
) -> anyhow::Result<Vec<u8>> {
    let mut schedule = Schedule::new(Update);
    schedule.add_systems((demo::spin_system, propagate_transforms).chain());

    for _ in 0..FRAMES {
        world
            .resource_mut::<Time>()
            .advance_by(Duration::from_millis(16));
        schedule.run(world);
    }

    let list = build_render_list(world, renderer.aspect_ratio(), assets);
    renderer.render(&list)?;
    renderer.capture_rgba()
}
