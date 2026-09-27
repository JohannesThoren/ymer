//! Prefabs: bygg en entitet med barn, spara den som prefab, instansiera
//! den flera gånger. Samma funktioner som knapparna i editorn anropar.
//!
//!     cargo run -p ymer-editor --example prefab_demo

use ymer_core::{Color, EntityName, GlobalTransform, Input, MeshInstance, Time, Vec3};
use ymer_editor::EditorState;
use ymer_editor::chrome::Chrome;
use ymer_render::Renderer;
use ymer_runtime::prelude::*;
use ymer_runtime::ui_backend::UiBackend;
use ymer_scene::{Scene, TypeRegistry, register_builtin_types};
use ymer_ui::FontAtlas;

const WIDTH: u32 = 1600;
const HEIGHT: u32 = 900;
const PROJECTS: &str = "projects";

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    let handle = ymer_editor::project::list(PROJECTS)
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("inget projekt – kör launcher_demo först"))?;

    let mut renderer = pollster::block_on(Renderer::new_offscreen(WIDTH, HEIGHT))?;
    let assets = ymer_render::Assets::new(&mut renderer);
    let cube = ymer_core::BUILTIN_CUBE;

    let mut registry = TypeRegistry::new();
    register_builtin_types(&mut registry);

    let mut world = World::new();
    world.insert_resource(Time::new());
    world.insert_resource(Input::default());
    world.insert_resource(ymer_core::ConsoleLog::default());
    Scene::load(handle.start_scene())?.spawn_into(&mut world, &registry)?;

    // Flytta kameran så att hela raden syns.
    {
        let mut query = world.query::<(&Camera, &mut Transform)>();
        if let Some((_, mut transform)) = query.iter_mut(&mut world).next() {
            *transform =
                Transform::from_xyz(0.0, 5.5, 13.0).looking_at(Vec3::new(0.0, 1.2, 0.0), Vec3::Y);
        }
    }

    // --- bygg ett torn med två barn --------------------------------------
    let root = world
        .spawn((
            EntityName::new("Torn"),
            Transform::from_xyz(-6.0, 0.5, 0.0),
            GlobalTransform::default(),
            MeshInstance::new(cube, Color::rgb(0.85, 0.45, 0.3)),
        ))
        .id();

    for (index, (height, scale, color)) in [
        (1.1, 0.65, Color::rgb(0.95, 0.75, 0.3)),
        (1.9, 0.4, Color::rgb(0.4, 0.85, 0.95)),
    ]
    .into_iter()
    .enumerate()
    {
        world.spawn((
            EntityName::new(format!("Torn-del {index}")),
            Transform::from_xyz(0.0, height, 0.0).with_scale(Vec3::splat(scale)),
            GlobalTransform::default(),
            MeshInstance::new(cube, color),
            ChildOf(root),
        ));
    }
    propagate_transforms(&mut world);

    // --- spara som prefab -------------------------------------------------
    let prefab = Scene::from_subtree(&mut world, &registry, root);
    let path = handle.path("prefabs").join("Torn.ron");
    prefab.save(&path)?;
    println!(
        "sparade prefabs/Torn.ron med {} entiteter",
        prefab.entities.len()
    );

    // --- instansiera den fyra gånger --------------------------------------
    for index in 1..=4 {
        let scene = Scene::load(&path)?;
        let mapping = scene.spawn_into(&mut world, &registry)?;
        let instance_root = mapping[&0];

        // Roten flyttas efter instansieringen – barnen följer med via hierarkin.
        if let Some(mut transform) = world.get_mut::<Transform>(instance_root) {
            transform.translation = Vec3::new(-6.0 + index as f32 * 3.0, 0.5, 0.0);
        }
    }
    propagate_transforms(&mut world);

    let towers = {
        let mut query = world.query::<&EntityName>();
        query.iter(&world).filter(|name| name.0 == "Torn").count()
    };
    println!(
        "{towers} torn i scenen, {} entiteter totalt",
        world.iter_entities().count()
    );

    // --- rendera editorn med prefabsmappen öppen --------------------------
    let mut state = EditorState::default();
    state.open_project(handle.clone());
    state.selected = Some(root);
    if let Some(browser) = state.browser.as_mut() {
        browser.enter(&handle.path("prefabs"));
    }

    let atlas =
        FontAtlas::from_font_bytes(std::fs::read("assets/fonts/DejaVuSans.ttf")?, 1024, 1024)
            .map_err(|err| anyhow::anyhow!(err))?;
    let mut ui = UiBackend::new(atlas, &mut renderer, &assets);
    let mut chrome = Chrome::new();
    let window = ymer_ui::Rect::new(0.0, 0.0, WIDTH as f32, HEIGHT as f32);

    // Två frames: den första bygger trädet, den andra ritar det färdigt.
    let mut commands = ymer_ui::DrawList::default();
    for _ in 0..2 {
        commands = chrome
            .frame(
                &ymer_ui::Input::default(),
                window,
                &mut state,
                &mut world,
                &registry,
                ui.atlas(),
            )
            .list;
    }

    let mut list = build_render_list(&mut world, renderer.aspect_ratio(), &assets);
    if let Some(selected) = state.selected {
        let camera = {
            let mut query = world.query::<(&Camera, &GlobalTransform)>();
            query
                .iter(&world)
                .next()
                .map(|(_, g)| g.translation())
                .unwrap_or_default()
        };
        list.overlay_items = ymer_editor::gizmo::gizmo_items(
            &mut world,
            selected,
            camera,
            assets.mesh(ymer_core::BUILTIN_CUBE),
            None,
        );
    }
    list.ui_items = ui.build(&commands, &assets);
    ui.upload(&mut renderer);
    renderer.render(&list)?;

    let output = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "prefab_demo.png".to_string());
    image::RgbaImage::from_raw(WIDTH, HEIGHT, renderer.capture_rgba()?)
        .ok_or_else(|| anyhow::anyhow!("fel bildstorlek"))?
        .save(&output)?;
    println!("skrev {output}");
    Ok(())
}
