//! Spelrunnern: en fristående binär som kör ett exporterat projekt.
//!
//! Runnern *är* motorn. Spelet är data – scener, assets och TypeScript –
//! så att uppdatera motorn betyder att byta ut den här filen. Ingen
//! dynamisk länkning behövs, och ingen ABI kan gå sönder.
//!
//!     runner                  # letar game.pak bredvid exe:n
//!     runner spel.pak         # uttryckligt arkiv
//!     runner projects/mitt    # lös katalog, för utveckling

use std::sync::Arc;

use ymer_core::{AssetSource, ConsoleLog, Input, Time};
use ymer_pak::{LooseFiles, PakArchive};
use ymer_runtime::{App, AppConfig, ScriptHost};
use ymer_scene::{Scene, TypeRegistry, register_builtin_types};

/// Arkivet som letas upp bredvid exe:n när inget anges.
const DEFAULT_ARCHIVE: &str = "game.pak";
/// Skriptvärden ligger bredvid exe:n, inte i projektarkivet – den hör
/// till motorn och byts ut tillsammans med runnern.
const SCRIPT_HOST: &str = "script_host.wasm";

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| std::path::PathBuf::from("."));

    // --- var ligger innehållet? ------------------------------------------
    let argument = std::env::args().nth(1);
    let (source, label): (Arc<dyn AssetSource>, String) = match argument {
        Some(path) => {
            let path = std::path::PathBuf::from(path);
            if path.is_dir() {
                let label = format!("katalog {}", path.display());
                (Arc::new(LooseFiles::new(path)), label)
            } else {
                let label = format!("arkiv {}", path.display());
                (Arc::new(PakArchive::open(&path)?), label)
            }
        }
        None => {
            let archive = exe_dir.join(DEFAULT_ARCHIVE);
            anyhow::ensure!(
                archive.exists(),
                "hittade inget {DEFAULT_ARCHIVE} bredvid {}. Ange ett arkiv eller en projektkatalog som argument.",
                exe_dir.display()
            );
            let label = format!("arkiv {}", archive.display());
            (Arc::new(PakArchive::open(&archive)?), label)
        }
    };
    log::info!("laddar från {label}");

    // --- projektfilen -----------------------------------------------------
    #[derive(serde::Deserialize)]
    struct ProjectFile {
        name: String,
        start_scene: String,
    }
    let project: ProjectFile = ron::from_str(&source.read_to_string("project.ron")?)
        .map_err(|err| anyhow::anyhow!("project.ron: {err}"))?;
    log::info!(
        "projekt: {} – startscen {}",
        project.name,
        project.start_scene
    );

    let mut registry = TypeRegistry::new();
    register_builtin_types(&mut registry);

    let wasm = std::fs::read(exe_dir.join(SCRIPT_HOST))
        .or_else(|_| std::fs::read(SCRIPT_HOST))
        .map_err(|err| anyhow::anyhow!("kunde inte läsa {SCRIPT_HOST}: {err}"))?;

    let scene_source = Arc::clone(&source);
    let script_source = Arc::clone(&source);
    let start_scene = project.start_scene.clone();
    let registry_for_setup = registry.clone();

    let mut app = App::new(AppConfig {
        title: project.name.clone(),
        width: 1280,
        height: 720,
    })
    .with_setup(move |world, renderer, assets| {
        world.insert_resource(Time::new());
        world.insert_resource(Input::default());
        world.insert_resource(ConsoleLog::default());

        let textures = assets.load_textures_from(renderer, scene_source.as_ref(), "");
        log::info!("{textures} texturer");

        // Modeller måste registreras om vid varje start: mesh-handtag
        // är körtidsdata, men namnen i scenfilerna är beständiga.
        let mut models = 0;
        for name in scene_source.list("", "glb") {
            match scene_source
                .read(&name)
                .map_err(|e| anyhow::anyhow!("{e}"))
                .and_then(|bytes| {
                    ymer_render::gltf_import::import_gltf_bytes(renderer, assets, &bytes, &name)
                }) {
                Ok(_) => models += 1,
                Err(err) => log::warn!("{name}: {err}"),
            }
        }
        log::info!("{models} modeller");

        match Scene::load_from(scene_source.as_ref(), &start_scene)
            .and_then(|scene| scene.spawn_into(world, &registry_for_setup))
        {
            Ok(mapping) => log::info!("{} entiteter", mapping.len()),
            Err(err) => log::error!("kunde inte ladda scenen: {err:#}"),
        }
    })
    .with_scripts(
        ScriptHost::from_source(wasm.clone(), script_source, "scripts"),
        registry.clone(),
    );

    // Debug-konsolen följer bara med i debugbyggen – ett släppt spel ska
    // inte ha en kommandorad som kan spawna entiteter.
    #[cfg(debug_assertions)]
    {
        app = app.with_console_scripting(wasm, registry);
        log::info!("debugbygge: tryck ` för konsolen");
    }

    app.run()
}
