//! Renderar banan utan fönster och skriver en PNG.
//!
//!     cargo run -p pussel --bin render --release -- bild.png [drag]
//!
//! Dragen är samma tangenter som i spelet, t.ex. `DDD` för tre steg åt
//! höger, så bilden kan visa banan både före och efter en lösning.

use ymer_core::{Camera, ConsoleLog, EntityName, GlobalTransform, Input, Time, Transform, Vec3};
use ymer_runtime::prelude::*;
use ymer_runtime::{ScriptHost, build_render_list};
use ymer_scene::{TypeRegistry, register_builtin_types};

const WIDTH: u32 = 960;
const HEIGHT: u32 = 640;
const PROJECT: &str = "projects/pussel";

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    let output = std::env::args().nth(1).unwrap_or("pussel.png".to_string());
    let moves = std::env::args().nth(2).unwrap_or_default();

    let mut registry = TypeRegistry::new();
    register_builtin_types(&mut registry);

    let mut world = World::new();
    world.insert_resource(Time::new());
    world.insert_resource(Input::default());
    world.insert_resource(ConsoleLog::default());
    pussel::build(&mut world);

    world.spawn((
        EntityName::new("Main Camera"),
        // Högt och snett: på en bred bana skymmer den närmaste muren annars
        // bortre raden, och det ser ut som ett renderingsfel.
        Transform::from_xyz(4.0, 9.5, 7.5).looking_at(Vec3::new(4.0, 0.0, 2.0), Vec3::Y),
        GlobalTransform::default(),
        Camera::default(),
    ));

    let mut host = ScriptHost::new(
        std::fs::read("assets/script_host.wasm")?,
        format!("{PROJECT}/scripts"),
    );
    for key in moves.chars() {
        let name = format!("Key{}", key.to_ascii_uppercase());
        world.resource_mut::<Input>().press(&name);
        host.tick(&mut world, &registry, 1.0 / 60.0)?;
        world.resource_mut::<Input>().end_frame();
        world.resource_mut::<Input>().release(&name);
        host.tick(&mut world, &registry, 1.0 / 60.0)?;
        world.resource_mut::<Input>().end_frame();
    }

    let mut renderer = pollster::block_on(ymer_render::Renderer::new_offscreen(WIDTH, HEIGHT))?;
    let mut assets = ymer_render::Assets::new(&mut renderer);

    // Modellerna måste registreras om vid varje start: mesh-handtag är
    // körtidsdata, men namnen i banan är beständiga.
    let root = std::path::Path::new(PROJECT);
    let mut loaded = 0;
    for entry in std::fs::read_dir(root.join("models"))? {
        let path = entry?.path();
        if path.extension().and_then(|e| e.to_str()) != Some("glb") {
            continue;
        }
        let name = format!("models/{}", path.file_name().unwrap().to_string_lossy());
        match ymer_render::import_gltf(&mut renderer, &mut assets, &path, &name) {
            Ok(asset) => {
                loaded += 1;
                log::info!("{name}: {:?} / {:?}", asset.mesh_names, asset.texture_names);
            }
            Err(err) => log::warn!("{name}: {err:#}"),
        }
    }
    println!("{loaded} modeller laddade");

    let mut schedule = Schedule::new(Update);
    schedule.add_systems(propagate_transforms);
    schedule.run(&mut world);

    let list = build_render_list(&mut world, renderer.aspect_ratio(), &assets);
    renderer.render(&list)?;

    let pixels = renderer.capture_rgba()?;
    let buffer = image::RgbaImage::from_raw(WIDTH, HEIGHT, pixels)
        .ok_or_else(|| anyhow::anyhow!("fel bildstorlek"))?;
    buffer.save(&output)?;
    println!("{} objekt -> {output}", list.items.len());
    Ok(())
}
