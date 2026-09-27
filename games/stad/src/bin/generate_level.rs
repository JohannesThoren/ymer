//! Skriver om `projects/stad/scenes/main.ron` från kartan i `lib.rs`.
//!
//! Kartan är data när den väl är skriven: spelet laddar scenfilen, och
//! den går att öppna och redigera i editorn som vilket projekt som helst.

fn main() -> anyhow::Result<()> {
    let mut registry = ymer_scene::TypeRegistry::new();
    ymer_scene::register_builtin_types(&mut registry);
    stad::register_types(&mut registry);

    let mut world = bevy_ecs::world::World::new();
    stad::bygg_karta(&mut world);

    let scene = ymer_scene::Scene::from_world(&mut world, &registry);
    let path = std::path::Path::new("projects/stad/scenes/main.ron");
    scene.save(path)?;
    println!("{} entiteter till {}", scene.entities.len(), path.display());
    Ok(())
}
