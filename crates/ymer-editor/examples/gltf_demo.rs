//! Importerar en glTF-fil, sparar den som prefab, instansierar den, och
//! renderar resultatet.
//!
//!     cargo run -p ymer-editor --example gltf_demo -- modell.glb

use ymer_core::{Color, EntityName, GlobalTransform, Input, MeshInstance, Time, Vec3};
use ymer_editor::gltf_import;
use ymer_render::{Assets, Renderer};
use ymer_runtime::prelude::*;
use ymer_scene::{Scene, TypeRegistry, register_builtin_types};

const WIDTH: u32 = 1280;
const HEIGHT: u32 = 720;

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let source = std::env::args().nth(1).unwrap_or_else(|| {
        std::env::temp_dir()
            .join("robot.glb")
            .to_string_lossy()
            .into_owned()
    });
    let source = std::path::Path::new(&source);

    let project_root = &std::env::temp_dir().join("gltf_demo_project");
    for folder in ["models", "prefabs"] {
        std::fs::create_dir_all(project_root.join(folder))?;
    }
    let target = project_root
        .join("models")
        .join(source.file_name().unwrap());
    std::fs::copy(source, &target)?;

    let mut renderer = pollster::block_on(Renderer::new_offscreen(WIDTH, HEIGHT))?;
    let mut assets = Assets::new(&mut renderer);

    let prefab_path =
        gltf_import::import_as_prefab(&mut renderer, &mut assets, project_root, &target)?;
    println!("prefab sparad: {}", prefab_path.display());
    println!("meshnamn: {:?}", assets.mesh_names().collect::<Vec<_>>());
    println!(
        "texturnamn: {:?}",
        assets.texture_names().collect::<Vec<_>>()
    );

    let mut registry = TypeRegistry::new();
    register_builtin_types(&mut registry);

    let mut world = World::new();
    world.insert_resource(Time::new());
    world.insert_resource(Input::default());
    world.insert_resource(ymer_core::ConsoleLog::default());

    world.spawn((
        EntityName::new("Main Camera"),
        Transform::from_xyz(2.5, 2.2, 4.0).looking_at(Vec3::new(0.0, 0.9, 0.0), Vec3::Y),
        GlobalTransform::default(),
        Camera::default(),
    ));
    world.spawn((
        EntityName::new("Ground"),
        Transform::from_xyz(0.0, -0.5, 0.0),
        GlobalTransform::default(),
        MeshInstance::new(ymer_core::BUILTIN_PLANE, Color::rgb(0.2, 0.2, 0.24)),
    ));

    // Instansiera prefaben tre gånger, precis som man skulle göra genom
    // att dubbelklicka den i filutforskaren.
    for i in 0..3 {
        let scene = Scene::load(&prefab_path)?;
        let mapping = scene.spawn_into(&mut world, &registry)?;
        let root = mapping[&0];
        if let Some(mut transform) = world.get_mut::<Transform>(root) {
            transform.translation.x = -2.5 + i as f32 * 2.5;
        }
    }
    propagate_transforms(&mut world);

    println!("entiteter: {}", world.iter_entities().count());

    let list = build_render_list(&mut world, renderer.aspect_ratio(), &assets);
    println!("ritar {} objekt", list.items.len());
    renderer.render(&list)?;
    std::fs::write(
        std::env::temp_dir().join("frame.raw"),
        renderer.capture_rgba()?,
    )?;
    println!("skrev {}", std::env::temp_dir().join("frame.raw").display());
    Ok(())
}
