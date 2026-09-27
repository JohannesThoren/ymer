//! Stadsbyggaren i ett fönster.
//!
//!     cargo run -p stad --release
//!
//! Vänsterklick bygger det som är valt i paletten, klick på ett hus
//! markerar det. `` ` `` öppnar debug-konsolen.
//!
//! Kartan kommer ur `scenes/main.ron` och reglerna ur `scripts/ekonomi.ts`.
//! Rust gör bara det skript inte når: plocket, som behöver kameramatrisen,
//! och gränssnittet, som är egui.

use bevy_ecs::prelude::*;
use stad::Val;
use stad::ui::UiKommando;
use ymer_core::{Camera, EntityName, GlobalTransform, Input, Transform, Vec3};
use ymer_runtime::prelude::*;
use ymer_runtime::{ScriptHost, UiFocus, init_logging};
use ymer_scene::{Scene, TypeRegistry, register_builtin_types};

const PROJECT: &str = "projects/stad";

fn main() -> anyhow::Result<()> {
    init_logging();

    let mut registry = TypeRegistry::new();
    register_builtin_types(&mut registry);
    stad::register_types(&mut registry);
    let registry_for_setup = registry.clone();

    let wasm = std::fs::read("assets/script_host.wasm")?;

    App::new(AppConfig {
        title: "Staden".into(),
        width: 1280,
        height: 760,
    })
    .with_setup(move |world, renderer, assets| {
        world.insert_resource(Val::default());
        world.insert_resource(UiKommando::default());

        world.spawn((
            EntityName::new("Main Camera"),
            Transform::from_xyz(0.0, 11.0, 11.0).looking_at(Vec3::ZERO, Vec3::Y),
            GlobalTransform::default(),
            Camera::default(),
        ));

        // Modellerna registreras om vid varje start: mesh-handtag är
        // körtidsdata, namnen i scenen är beständiga.
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

        match Scene::load(format!("{PROJECT}/scenes/main.ron"))
            .and_then(|scene| scene.spawn_into(world, &registry_for_setup))
        {
            Ok(mapping) => log::info!("{} entiteter ur scenen", mapping.len()),
            Err(err) => log::error!("kunde inte ladda kartan: {err:#}"),
        }
    })
    .with_scripts(
        ScriptHost::new(wasm.clone(), format!("{PROJECT}/scripts")),
        registry.clone(),
    )
    .with_console_scripting(wasm, registry)
    .with_ui(stad::ui::rita)
    .add_systems(musklick)
    .run()
}

/// Tolkar musklick mot rutnätet.
///
/// Exclusive system: att bygga spawnar, och det kräver hela världen.
fn musklick(world: &mut World) {
    // Rivning som UI:t bad om förra framen. Sker före pekarkollen –
    // knappen ligger i UI:t, så pekaren är per definition upptagen där.
    if let Some(tile) = world
        .get_resource::<UiKommando>()
        .and_then(|kommando| kommando.riv)
    {
        if stad::riv(world, tile) {
            let mut val = world.resource_mut::<Val>();
            val.markerad = None;
            val.status = "Rivet, halva kostnaden tillbaka.".to_string();
        }
        world.insert_resource(UiKommando { riv: None });
        return;
    }

    // UI:t äger pekaren när den är över en panel. Utan den här kollen
    // bygger ett klick på "Hus"-knappen också ett hus på marken bakom.
    if world
        .get_resource::<UiFocus>()
        .is_some_and(|focus| focus.pointer)
    {
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
    let (aspect, skarm) = (skarm.aspect(), skarm.size);

    let Some(view_proj) = stad::view_proj(world, aspect) else {
        return;
    };
    let Some(tile) = stad::ruta_under_musen(view_proj, mus, skarm) else {
        return;
    };

    // Klick på ett hus markerar det; klick på tom mark bygger.
    if stad::byggnad_pa(world, tile).is_some() {
        let mut val = world.resource_mut::<Val>();
        val.markerad = Some(tile);
        val.status = format!("Ruta {}, {} markerad.", tile.0, tile.1);
        return;
    }

    let vald = world.resource::<Val>().vald;
    match stad::bygg(world, tile, vald) {
        Ok(_) => {
            let mut val = world.resource_mut::<Val>();
            val.status = format!("{} byggt.", vald.namn());
        }
        Err(neka) => {
            let mut val = world.resource_mut::<Val>();
            val.status = neka.text().to_string();
        }
    }
}
