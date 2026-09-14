//! 2D: ortografisk kamera, sprites, spritesheet-animation och alfa.
//!
//!     cargo run -p ymer-runtime --example sprites2d

use std::time::Duration;

use bevy_ecs::prelude::*;
use ymer_core::{
    Camera, Color, ConsoleLog, EntityName, GlobalTransform, Input, Sprite, SpriteAnimation, Time,
    Transform, propagate_transforms,
};
use ymer_render::{Assets, Renderer};
use ymer_runtime::{Update, build_render_list};

const WIDTH: u32 = 1280;
const HEIGHT: u32 = 720;

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    let mut renderer = pollster::block_on(Renderer::new_offscreen(WIDTH, HEIGHT))?;
    let mut assets = Assets::new(&mut renderer);

    let root = &std::env::temp_dir().join("2d");
    let loaded = assets.load_textures(&mut renderer, root, root);
    println!("{loaded} texturer laddade");

    let mut world = World::new();
    world.insert_resource(Time::new());
    world.insert_resource(Input::default());
    world.insert_resource(ConsoleLog::default());

    // Ortografisk kamera: 12 världsenheter vertikalt i bild. Positionen på
    // +Z tittar rakt in i XY-planet, som är där sprites lever.
    world.spawn((
        EntityName::new("Camera2D"),
        Transform::from_xyz(0.0, 0.0, 10.0),
        GlobalTransform::default(),
        Camera::orthographic_2d(12.0),
    ));

    // Mark: samma textur upprepad, ingen animation.
    for x in -8..=8 {
        world.spawn((
            EntityName::new(format!("Mark {x}")),
            Transform::from_xyz(x as f32 * 1.5, -4.0, 0.0),
            GlobalTransform::default(),
            Sprite::new("mark.png", 1.5, 1.5),
        ));
    }

    // Moln med alfa – ligger bakom gubbarna i Z för att visa att
    // sorteringen bakifrån-och-fram fungerar.
    for (i, (x, y)) in [(-4.5, 3.2), (1.5, 4.0), (5.0, 2.6)]
        .into_iter()
        .enumerate()
    {
        world.spawn((
            EntityName::new(format!("Moln {i}")),
            Transform::from_xyz(x, y, -1.0),
            GlobalTransform::default(),
            Sprite::new("moln.png", 4.0, 2.0),
        ));
    }

    // Tre animerade gubbar i olika takt, en spegelvänd.
    for (i, (x, fps, flip)) in [(-4.0, 8.0, false), (0.0, 3.0, false), (4.0, 12.0, true)]
        .into_iter()
        .enumerate()
    {
        let mut sprite = Sprite::new("gubbe.png", 2.0, 2.0).with_grid(4, 2);
        sprite.flip_x = flip;

        world.spawn((
            EntityName::new(format!("Gubbe {i}")),
            Transform::from_xyz(x, -2.0, 0.0),
            GlobalTransform::default(),
            sprite,
            // frames: 4 = bara översta raden (gå-cykeln), inte hela arket.
            SpriteAnimation {
                fps,
                frames: 4,
                playing: true,
                timer: 0.0,
            },
        ));
    }

    // En färgtonad sprite: color multipliceras med texturen.
    let mut tinted = Sprite::new("gubbe.png", 2.0, 2.0).with_grid(4, 2);
    tinted.index = 4; // första rutan på rad två
    world.spawn((
        EntityName::new("Tonad"),
        Transform::from_xyz(-7.0, -2.0, 0.0),
        GlobalTransform::default(),
        tinted.with_color(Color::rgb(0.6, 1.0, 0.6)),
    ));

    let mut schedule = Schedule::new(Update);
    schedule.add_systems((ymer_core::animate_sprites, propagate_transforms).chain());

    // Stega fram så att animationen hamnar mitt i gå-cykeln.
    for _ in 0..20 {
        world
            .resource_mut::<Time>()
            .advance_by(Duration::from_millis(16));
        schedule.run(&mut world);
    }

    let indices: Vec<u32> = {
        let mut query = world.query::<(&EntityName, &Sprite)>();
        let mut found: Vec<(String, u32)> = query
            .iter(&world)
            .map(|(n, s)| (n.0.clone(), s.index))
            .filter(|(n, _)| n.starts_with("Gubbe"))
            .collect();
        found.sort();
        found.into_iter().map(|(_, i)| i).collect()
    };
    println!("gubbarnas rutindex efter 20 frames: {indices:?}");

    let list = build_render_list(&mut world, renderer.aspect_ratio(), &assets);
    println!(
        "ogenomskinliga: {}, sprites: {}",
        list.items.len(),
        list.sprite_items.len()
    );

    renderer.render(&list)?;
    std::fs::write(
        std::env::temp_dir().join("frame.raw"),
        renderer.capture_rgba()?,
    )?;
    println!("skrev {}", std::env::temp_dir().join("frame.raw").display());
    Ok(())
}
