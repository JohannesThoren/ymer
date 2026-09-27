//! Stadsbyggaren i ett fönster.
//!
//!     cargo run -p stad --release
//!
//! Vänsterklick bygger det som är valt i paletten, klick på ett hus
//! markerar det. `` ` `` öppnar debug-konsolen.

use bevy_ecs::prelude::*;
use stad::Stad;
use stad::ui::UiKommando;
use ymer_core::{Camera, EntityName, GlobalTransform, Input, Transform, Vec3};
use ymer_runtime::prelude::*;
use ymer_runtime::{UiFocus, init_logging};

const PROJECT: &str = "projects/stad";

fn main() -> anyhow::Result<()> {
    init_logging();

    let mut app = App::new(AppConfig {
        title: "Staden".into(),
        width: 1280,
        height: 760,
    })
    .with_setup(|world, renderer, assets| {
        world.insert_resource(Stad::default());
        world.insert_resource(UiKommando::default());

        world.spawn((
            EntityName::new("Main Camera"),
            Transform::from_xyz(0.0, 11.0, 11.0).looking_at(Vec3::ZERO, Vec3::Y),
            GlobalTransform::default(),
            Camera::default(),
        ));

        stad::bygg_mark(world);

        // Modellerna registreras om vid varje start: mesh-handtag är
        // körtidsdata, namnen i koden är beständiga.
        let models = std::path::Path::new(PROJECT).join("models");
        match std::fs::read_dir(&models) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) != Some("glb") {
                        continue;
                    }
                    let name = format!("models/{}", path.file_name().unwrap().to_string_lossy());
                    if let Err(err) = ymer_render::import_gltf(renderer, assets, &path, &name) {
                        log::warn!("{name}: {err:#}");
                    }
                }
            }
            Err(err) => log::error!("hittar inga modeller i {}: {err}", models.display()),
        }
    })
    .with_ui(stad::ui::rita)
    .add_systems(stad::ekonomi_system);

    app = app.with_debug_console();
    app.add_systems(musklick).run()
}

/// Tolkar musklick mot rutnätet.
///
/// Körs som ett vanligt system, men behöver `&mut World` för att kunna
/// spawna – därför en exclusive system.
fn musklick(world: &mut World) {
    // UI:t äger pekaren när den är över en panel. Utan den här kollen
    // bygger ett klick på "Hus"-knappen också ett hus på marken bakom.
    if world
        .get_resource::<UiFocus>()
        .is_some_and(|focus| focus.pointer)
    {
        return;
    }

    // Rivning som UI:t bad om förra framen.
    if let Some(tile) = world
        .get_resource::<UiKommando>()
        .and_then(|kommando| kommando.riv)
    {
        stad::riv(world, tile);
        world.insert_resource(UiKommando { riv: None });
        return;
    }

    let Some(input) = world.get_resource::<Input>() else {
        return;
    };
    if !input.mouse_just_pressed("Left") {
        return;
    }
    let mus = input.mouse_position;

    let Some(skarm) = world.get_resource::<ymer_core::Screen>().copied() else {
        return;
    };
    let (skarm, aspect) = (skarm.size, skarm.aspect());

    let Some(view_proj) = stad::view_proj(world, aspect) else {
        return;
    };
    let Some(tile) = stad::ruta_under_musen(view_proj, mus, skarm) else {
        return;
    };

    // Klick på ett hus markerar det; klick på tom mark bygger.
    let upptagen = world.resource::<Stad>().upptagen(tile).is_some();
    if upptagen {
        let mut stad = world.resource_mut::<Stad>();
        stad.markerad = Some(tile);
        stad.status = "Markerad.".to_string();
        return;
    }

    let vald = world.resource::<Stad>().vald;
    match stad::bygg(world, tile, vald) {
        Ok(_) => {}
        Err(neka) => {
            let mut stad = world.resource_mut::<Stad>();
            stad.status = neka.text().to_string();
        }
    }
}
