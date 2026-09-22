//! Skriver om `projects/pussel/scenes/main.ron` från banan i `lib.rs`.

fn main() -> anyhow::Result<()> {
    let mut registry = ymer_scene::TypeRegistry::new();
    ymer_scene::register_builtin_types(&mut registry);

    let scene = pussel::scene(&registry)?;
    let path = std::path::Path::new("projects/pussel/scenes/main.ron");
    scene.save(path)?;
    println!("{} entiteter till {}", scene.entities.len(), path.display());
    Ok(())
}
