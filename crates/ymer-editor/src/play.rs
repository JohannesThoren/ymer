//! Play-läget. Kör skript i editorn och – viktigast av allt – lägger
//! scenen tillbaka som den var när man trycker stopp.
//!
//! Utan ögonblicksbilden skulle varje testkörning förstöra scenen man
//! just byggt, eftersom skript flyttar saker och kan despawna dem.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use anyhow::{Result, anyhow};
use bevy_ecs::prelude::*;
use ymer_core::Script;
use ymer_scene::{Scene, TypeRegistry, clear_scene};
use ymer_script::{ScriptRuntime, run_script_general};

/// Hur ofta disken kollas efter ändrade skript.
const POLL_INTERVAL: Duration = Duration::from_millis(250);

struct Loaded {
    runtime: ScriptRuntime,
    modified: Option<SystemTime>,
}

pub struct PlayMode {
    wasm: Vec<u8>,
    script_dir: PathBuf,
    /// En QuickJS-instans per skriptfil, delad av alla entiteter som
    /// använder filen. Skripten körs som system.
    runtimes: BTreeMap<String, Loaded>,
    snapshot: Option<Scene>,
    last_poll: Instant,
}

impl PlayMode {
    /// Skriptvärden hör till motorn, skriptkatalogen till projektet –
    /// därför två sökvägar.
    pub fn new(wasm_path: impl AsRef<Path>, script_dir: impl AsRef<Path>) -> Result<Self> {
        let wasm = std::fs::read(wasm_path.as_ref())
            .map_err(|err| anyhow!("kunde inte läsa {}: {err}", wasm_path.as_ref().display()))?;

        Ok(Self {
            wasm,
            script_dir: script_dir.as_ref().to_path_buf(),
            runtimes: BTreeMap::new(),
            snapshot: None,
            last_poll: Instant::now(),
        })
    }

    pub fn is_running(&self) -> bool {
        self.snapshot.is_some()
    }

    /// Sparar scenen och laddar alla skript som används i den.
    pub fn start(&mut self, world: &mut World, registry: &TypeRegistry) -> Result<usize> {
        self.snapshot = Some(Scene::from_world(world, registry));
        self.runtimes.clear();

        let names = script_names(world);
        for name in &names {
            self.load(name)?;
        }
        Ok(names.len())
    }

    /// Slänger allt skripten gjorde och bygger upp scenen igen från
    /// ögonblicksbilden.
    pub fn stop(&mut self, world: &mut World, registry: &TypeRegistry) -> Result<()> {
        let Some(scene) = self.snapshot.take() else {
            return Ok(());
        };
        self.runtimes.clear();

        clear_scene(world, registry);

        scene.spawn_into(world, registry)?;
        Ok(())
    }

    /// Kör alla laddade skript en frame. Nya skript som dykt upp under
    /// körningen laddas i farten.
    pub fn tick(&mut self, world: &mut World, registry: &TypeRegistry, dt: f32) -> Result<usize> {
        let mut affected = 0;

        for name in script_names(world) {
            if !self.runtimes.contains_key(&name) {
                self.load(&name)?;
            }

            let loaded = self.runtimes.get_mut(&name).expect("laddad ovan");
            affected += run_script_general(world, &mut loaded.runtime, registry, &name, dt)?;
        }

        Ok(affected)
    }

    /// Läser om en skriptfil från disk utan att röra scenen. State ligger
    /// i komponenter, så hot reload är bara en ny kompilering.
    pub fn reload(&mut self, name: &str) -> Result<()> {
        self.load(name)
    }

    /// Kollar om någon skriptfil ändrats på disk och laddar om den.
    /// Returnerar en rad per fil som försökte laddas om.
    ///
    /// Det här är mtime-pollning, inte inotify. Skälet är att de flesta
    /// editorer sparar atomärt – skriver en temporärfil och byter namn –
    /// vilket filbevakare rapporterar olika på olika plattformar och som
    /// ofta ger både "borttagen" och "skapad" istället för "ändrad".
    /// En stat-anrop per skript var 250:e ms är osynligt i jämförelse.
    pub fn poll_reloads(&mut self, force: bool) -> Vec<Result<String>> {
        if !force && self.last_poll.elapsed() < POLL_INTERVAL {
            return Vec::new();
        }
        self.last_poll = Instant::now();

        let stale: Vec<String> = self
            .runtimes
            .iter()
            .filter(|(name, loaded)| {
                let current = modified_time(&self.script_dir.join(name.as_str()));
                current.is_some() && current != loaded.modified
            })
            .map(|(name, _)| name.clone())
            .collect();

        stale
            .into_iter()
            .map(|name| self.load(&name).map(|()| name))
            .collect()
    }

    fn load(&mut self, name: &str) -> Result<()> {
        let path = self.script_dir.join(name);
        let source = std::fs::read_to_string(&path).map_err(|err| anyhow!("{name}: {err}"))?;

        let mut runtime = ScriptRuntime::new(&self.wasm)?;
        runtime.load_typescript(&source, name)?;

        self.runtimes.insert(
            name.to_string(),
            Loaded {
                runtime,
                modified: modified_time(&path),
            },
        );
        Ok(())
    }
}

fn modified_time(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).ok()?.modified().ok()
}

fn script_names(world: &mut World) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    let mut query = world.query::<&Script>();
    for script in query.iter(world) {
        if !names.contains(&script.0) {
            names.push(script.0.clone());
        }
    }
    names
}
