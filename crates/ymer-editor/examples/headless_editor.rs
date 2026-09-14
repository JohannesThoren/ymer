//! Renderar hela editorn – scen plus egui-paneler – till en textur.
//! Låter oss se UI:t utan skärm och funkar som regressionstest.
//!
//!     cargo run -p ymer-editor --example headless_editor

use std::time::Duration;

use ymer_core::Script;
use ymer_core::{GlobalTransform, Vec3};
use ymer_editor::gizmo;
use ymer_editor::{EditorState, EguiOverlay, PlayMode, run_ui};
use ymer_render::Renderer;
use ymer_runtime::demo;
use ymer_runtime::prelude::*;
use ymer_scene::{TypeRegistry, register_builtin_types};

const PIXELS_PER_POINT: f32 = 1.4;
const WIDTH: u32 = 1600;
const HEIGHT: u32 = 900;

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    let mut renderer = pollster::block_on(Renderer::new_offscreen(WIDTH, HEIGHT))?;
    let assets = ymer_render::Assets::new(&mut renderer);

    let mut registry = TypeRegistry::new();
    register_builtin_types(&mut registry);
    registry.register::<demo::Spin>("Spin");

    let mut world = World::new();
    world.insert_resource(Time::new());
    demo::setup(&mut world);

    // Kuberna kör sin logik som skript istället för som Rust-system.
    let spinners: Vec<Entity> = {
        let mut query = world.query_filtered::<Entity, With<demo::Spin>>();
        query.iter(&world).collect()
    };
    for entity in &spinners {
        world.entity_mut(*entity).remove::<demo::Spin>();
        world.entity_mut(*entity).insert(Script::new("paint.ts"));
    }

    // Tryck spela: ögonblicksbild tas, skripten laddas, scenen börjar leva.
    let mut play = PlayMode::new("assets/script_host.wasm", "assets/scripts")?;
    let loaded = play.start(&mut world, &registry)?;
    println!("play startad, {loaded} skript laddade");

    let before = world.iter_entities().count();
    for _ in 0..75 {
        world
            .resource_mut::<Time>()
            .advance_by(Duration::from_millis(16));
        play.tick(&mut world, &registry, 1.0 / 60.0)?;
        propagate_transforms(&mut world);
    }
    let during = world.iter_entities().count();
    println!("entiteter före play: {before}, under körning: {during}");

    // Gizmot ritas av vanliga kuber i overlay-passet.
    let gizmo_mesh = assets.mesh(ymer_core::BUILTIN_CUBE);

    // --- plocka en entitet med en simulerad musklick ----------------------
    // Projicera en känd kub till skärmen, skjut en stråle genom den pixeln
    // och se om plockningen hittar tillbaka till samma entitet.
    let aspect = renderer.aspect_ratio();
    let view_proj = gizmo::camera_view_proj(&mut world, aspect).expect("kamera saknas");

    let target = world
        .iter_entities()
        .find(|entity| {
            entity
                .get::<ymer_core::EntityName>()
                .is_some_and(|n| n.0 == "Cube 0,0")
        })
        .map(|entity| entity.id())
        .expect("Cube 0,0 saknas");

    let target_position = world.get::<GlobalTransform>(target).unwrap().translation();
    let clip = view_proj * target_position.extend(1.0);
    let click = (
        (clip.x / clip.w * 0.5 + 0.5) * WIDTH as f32,
        (0.5 - clip.y / clip.w * 0.5) * HEIGHT as f32,
    );

    let ray = gizmo::screen_ray(view_proj, click, (WIDTH, HEIGHT));
    let picked = gizmo::pick(&mut world, &renderer.meshes, &assets, &ray);
    // Under körning rör sig kuberna, så strålen kan mycket väl träffa en
    // annan kub närmare kameran – det är korrekt beteende, inte ett fel.
    println!(
        "klick på ({:.0}, {:.0}) plockade {:?} (siktade på {:?})",
        click.0, click.1, picked, target
    );

    let selected = picked;

    // --- dra i Y-handtaget -----------------------------------------------
    let camera = {
        let mut query = world.query::<(&ymer_core::Camera, &GlobalTransform)>();
        query
            .iter(&world)
            .next()
            .map(|(_, g)| g.translation())
            .unwrap()
    };

    let before = world.get::<Transform>(target).unwrap().translation;

    // Sikta mitt på Y-armen, greppa, och dra pekaren 90 pixlar uppåt.
    let handle = target_position + Vec3::Y * (target_position - camera).length() * 0.16 * 0.6;
    let handle_clip = view_proj * handle.extend(1.0);
    let grab = (
        (handle_clip.x / handle_clip.w * 0.5 + 0.5) * WIDTH as f32,
        (0.5 - handle_clip.y / handle_clip.w * 0.5) * HEIGHT as f32,
    );

    let grab_ray = gizmo::screen_ray(view_proj, grab, (WIDTH, HEIGHT));
    let drag = gizmo::begin_drag(&mut world, target, &grab_ray, camera);
    println!("grepp: {drag:?}");

    if let Some(drag) = drag {
        let moved_ray = gizmo::screen_ray(view_proj, (grab.0, grab.1 - 90.0), (WIDTH, HEIGHT));
        gizmo::update_drag(&mut world, &drag, &moved_ray);
        propagate_transforms(&mut world);
    }

    let after = world.get::<Transform>(target).unwrap().translation;
    println!("flyttad från {before:?} till {after:?}");

    let ctx = egui::Context::default();
    ctx.set_pixels_per_point(PIXELS_PER_POINT);
    ctx.set_visuals(egui::Visuals::dark());

    let mut state = EditorState::default();
    state.playing = true;
    state.selected = selected;
    state.playing = true;

    // Första framen mäter layouten, andra ritar den färdigt. Båda måste
    // lämnas till overlayen – en TexturesDelta som slängs oanvänd panikar.
    let mut overlay = EguiOverlay::new(renderer.device(), renderer.output_format());
    let points = egui::vec2(
        WIDTH as f32 / PIXELS_PER_POINT,
        HEIGHT as f32 / PIXELS_PER_POINT,
    );
    for _ in 0..2 {
        let raw_input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, points)),
            ..Default::default()
        };
        let output = run_ui(&ctx, raw_input, &mut state, &mut world, &registry);
        overlay.accept(&ctx, output, (WIDTH, HEIGHT));
    }

    let mut list = build_render_list(&mut world, renderer.aspect_ratio(), &assets);
    if let Some(selected) = selected {
        list.overlay_items = gizmo::gizmo_items(&mut world, selected, camera, gizmo_mesh, None);
    }
    renderer.render_with_overlay(&list, Some(&mut overlay))?;

    let pixels = renderer.capture_rgba()?;
    std::fs::write(std::env::temp_dir().join("frame.raw"), &pixels)?;

    // Stopp: allt skripten hann göra rullas tillbaka.
    play.stop(&mut world, &registry)?;
    println!("efter stopp: {} entiteter", world.iter_entities().count());
    println!(
        "skrev {} ({WIDTH}x{HEIGHT})",
        std::env::temp_dir().join("frame.raw").display()
    );
    Ok(())
}
