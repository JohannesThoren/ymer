//! Renderar staden med sitt gränssnitt, utan fönster, och skriver en PNG.
//!
//!     cargo run -p stad --release --bin render -- bild.png
//!
//! HUD:en ritas i samma render-pass som scenen, så det här är också sättet
//! att se att spelets UI och 3D-vyn faktiskt samsas – annars märks felet
//! först när spelet startas på en riktig maskin.

use bevy_ecs::prelude::*;
use stad::{Kind, Stadskassa, Val};
use ymer_core::{
    Camera, ConsoleLog, EntityName, GlobalTransform, Input, Screen, Time, Transform, Vec2, Vec3,
};
use ymer_render::{Assets, EguiOverlay, OFFSCREEN_FORMAT, Renderer};
use ymer_runtime::prelude::*;
use ymer_runtime::{ScriptHost, build_render_list};
use ymer_scene::{Scene, TypeRegistry, register_builtin_types};

const WIDTH: u32 = 1280;
const HEIGHT: u32 = 760;
const PROJECT: &str = "projects/stad";

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    let output = std::env::args().nth(1).unwrap_or("stad.png".to_string());

    let mut registry = TypeRegistry::new();
    register_builtin_types(&mut registry);
    stad::register_types(&mut registry);

    let mut world = World::new();
    world.insert_resource(Time::new());
    world.insert_resource(Input::default());
    world.insert_resource(ConsoleLog::default());
    world.insert_resource(Val::default());
    world.insert_resource(stad::ui::UiKommando::default());
    world.insert_resource(Screen {
        size: Vec2::new(WIDTH as f32, HEIGHT as f32),
    });

    // Samma karta som spelet laddar.
    Scene::load(format!("{PROJECT}/scenes/main.ron"))?
        .spawn_into(&mut world, &registry)
        .map_err(|err| anyhow::anyhow!("kunde inte ladda kartan: {err:#}"))?;

    world.spawn((
        EntityName::new("Main Camera"),
        Transform::from_xyz(0.0, 11.0, 11.0).looking_at(Vec3::ZERO, Vec3::Y),
        GlobalTransform::default(),
        Camera::default(),
    ));
    // En liten stad, byggd genom samma väg som ett klick tar.
    {
        let entity = stad::kassa_entity(&mut world).expect("stadshuset finns i scenen");
        world.get_mut::<Stadskassa>(entity).unwrap().guld = 2000.0;
    }
    let plan = [
        ((-2, -2), Kind::Hus),
        ((-1, -2), Kind::Hus),
        ((0, -2), Kind::Hus),
        ((-2, -1), Kind::Hus),
        ((1, -1), Kind::Butik),
        ((2, -1), Kind::Butik),
        ((-1, 0), Kind::Marknad),
        ((1, 1), Kind::Kontor),
        ((3, 2), Kind::Hus),
        ((-3, 1), Kind::Butik),
    ];
    for (tile, kind) in plan {
        if let Err(neka) = stad::bygg(&mut world, tile, kind) {
            log::warn!("{tile:?}: {}", neka.text());
        }
    }
    // Låt skriptet driva staden en stund, så att HUD:en visar liv.
    let mut host = ScriptHost::new(
        std::fs::read("assets/script_host.wasm")?,
        format!("{PROJECT}/scripts"),
    );
    for _ in 0..600 {
        host.tick(&mut world, &registry, 1.0 / 60.0)?;
        world.resource_mut::<Input>().end_frame();
    }
    {
        let mut val = world.resource_mut::<Val>();
        val.markerad = Some((1, 1));
        val.vald = Kind::Marknad;
        val.status = "Kontoret är bara halvbemannat – bygg fler hus.".to_string();
    }

    let mut renderer = pollster::block_on(Renderer::new_offscreen(WIDTH, HEIGHT))?;
    let mut assets = Assets::new(&mut renderer);

    let models = std::path::Path::new(PROJECT).join("models");
    for entry in std::fs::read_dir(&models)? {
        let path = entry?.path();
        if path.extension().and_then(|e| e.to_str()) != Some("glb") {
            continue;
        }
        let name = format!("models/{}", path.file_name().unwrap().to_string_lossy());
        if let Err(err) = ymer_render::import_gltf(&mut renderer, &mut assets, &path, &name) {
            log::warn!("{name}: {err:#}");
        }
    }

    let mut schedule = Schedule::new(Update);
    schedule.add_systems(propagate_transforms);
    schedule.run(&mut world);

    // egui utan winit: en råinput räcker för att köra en frame.
    let ctx = egui::Context::default();
    ctx.set_visuals(egui::Visuals::dark());
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::pos2(0.0, 0.0),
            egui::vec2(WIDTH as f32, HEIGHT as f32),
        )),
        ..Default::default()
    };

    let mut overlay = EguiOverlay::new(renderer.device(), OFFSCREEN_FORMAT);
    // Flera frames, inte en. Den första mäter panelerna och den andra
    // lägger ut dem, och alla måste lämnas till overlayen – typsnitts-
    // atlasen kommer i den första, och utan den ritas all text som
    // ingenting. Fönster tonar dessutom in, så med bara två frames blir
    // den markerade byggnadens panel halvgenomskinlig.
    for _ in 0..8 {
        let mut frame = input.clone();
        frame.predicted_dt = 1.0 / 20.0;
        let output = ctx.run_ui(frame, |ui| stad::ui::rita(ui, &mut world));
        overlay.accept(&ctx, output, (WIDTH, HEIGHT));
    }

    let list = build_render_list(&mut world, renderer.aspect_ratio(), &assets);
    renderer.render_with_overlay(&list, Some(&mut overlay))?;

    let pixels = renderer.capture_rgba()?;
    image::RgbaImage::from_raw(WIDTH, HEIGHT, pixels)
        .ok_or_else(|| anyhow::anyhow!("fel bildstorlek"))?
        .save(&output)?;

    let k = stad::kassa(&mut world);
    let antal = {
        let mut query = world.query::<&stad::Byggnad>();
        query.iter(&world).count()
    };
    println!(
        "{antal} byggnader, {:.0} invånare av {} platser, {:.1} guld/s -> {output}",
        k.invanare.floor(),
        k.platser,
        k.inkomst
    );
    Ok(())
}
