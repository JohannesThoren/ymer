//! Renderar demo-scenen utan fönster och skriver rå RGBA till fil.
//! Deterministiskt: klockan stegas manuellt, så bilden blir identisk
//! varje körning. Bra som röktest i CI.
//!
//!     cargo run -p ymer-runtime --example headless

use std::time::Duration;

use ymer_render::Renderer;
use ymer_runtime::prelude::*;
use ymer_runtime::{Update, demo, init_logging};

const WIDTH: u32 = 1280;
const HEIGHT: u32 = 720;
const FRAMES: u32 = 75;

fn main() -> anyhow::Result<()> {
    init_logging();

    let mut renderer = pollster::block_on(Renderer::new_offscreen(WIDTH, HEIGHT))?;
    let assets = ymer_render::Assets::new(&mut renderer);

    let mut world = World::new();
    world.insert_resource(Time::new());
    demo::setup(&mut world);

    let mut schedule = Schedule::new(Update);
    schedule.add_systems((demo::spin_system, propagate_transforms).chain());

    // 75 steg à 16 ms = 1,2 sekunders animation.
    for _ in 0..FRAMES {
        world
            .resource_mut::<Time>()
            .advance_by(Duration::from_millis(16));
        schedule.run(&mut world);
    }

    let list = build_render_list(&mut world, renderer.aspect_ratio(), &assets);
    println!("ritar {} objekt", list.items.len());
    renderer.render(&list)?;

    let pixels = renderer.capture_rgba()?;
    std::fs::write(std::env::temp_dir().join("frame.raw"), &pixels)?;
    println!(
        "skrev {} ({} bytes, {WIDTH}x{HEIGHT} RGBA)",
        std::env::temp_dir().join("frame.raw").display(),
        pixels.len()
    );
    Ok(())
}
