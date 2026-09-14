//! Skriptkörning för spel.
//!
//! Editorns `PlayMode` gör mer: ögonblicksbild av scenen, återställning vid
//! stopp, omladdning under körning. Ett exporterat spel behöver inget av
//! det – bara ladda skripten en gång och köra dem varje frame. Den här
//! modulen är den delen, utan beroende på editorn.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Result, anyhow};
use bevy_ecs::prelude::*;
use ymer_core::{AssetSource, Script};
use ymer_scene::TypeRegistry;
use ymer_script::{ScriptRuntime, run_script_general};

pub struct ScriptHost {
    wasm: Vec<u8>,
    /// Var skripten läses ifrån. `Directory` i editorn och i utveckling,
    /// `Source` när spelet kör ur ett arkiv.
    location: ScriptLocation,
    /// En QuickJS-instans per skriptfil, delad av alla entiteter som
    /// använder filen.
    runtimes: BTreeMap<String, ScriptRuntime>,
}

enum ScriptLocation {
    Directory(PathBuf),
    Source {
        source: Arc<dyn AssetSource>,
        prefix: String,
    },
}

impl ScriptLocation {
    fn read(&self, name: &str) -> Result<String> {
        match self {
            Self::Directory(dir) => {
                std::fs::read_to_string(dir.join(name)).map_err(|err| anyhow!("{name}: {err}"))
            }
            Self::Source { source, prefix } => {
                let path = if prefix.is_empty() {
                    name.to_string()
                } else {
                    format!("{}/{name}", prefix.trim_end_matches('/'))
                };
                source
                    .read_to_string(&path)
                    .map_err(|err| anyhow!("{path}: {err}"))
            }
        }
    }
}

impl ScriptHost {
    pub fn new(wasm: Vec<u8>, script_dir: impl Into<PathBuf>) -> Self {
        Self {
            wasm,
            location: ScriptLocation::Directory(script_dir.into()),
            runtimes: BTreeMap::new(),
        }
    }

    /// Läser skript ur en assetkälla. `prefix` är katalogen inuti källan,
    /// normalt `"scripts"` – `Script`-komponenten pekar ut filen därunder.
    pub fn from_source(
        wasm: Vec<u8>,
        source: Arc<dyn AssetSource>,
        prefix: impl Into<String>,
    ) -> Self {
        Self {
            wasm,
            location: ScriptLocation::Source {
                source,
                prefix: prefix.into(),
            },
            runtimes: BTreeMap::new(),
        }
    }

    /// Läser `script_host.wasm` från en sökväg.
    pub fn from_paths(wasm_path: impl AsRef<Path>, script_dir: impl Into<PathBuf>) -> Result<Self> {
        let wasm = std::fs::read(wasm_path.as_ref())
            .map_err(|err| anyhow!("kunde inte läsa {}: {err}", wasm_path.as_ref().display()))?;
        Ok(Self::new(wasm, script_dir))
    }

    /// Kör alla skript en frame. Nya skriptnamn laddas i farten, så en
    /// entitet som spawnas med en `Script`-komponent fungerar direkt.
    pub fn tick(&mut self, world: &mut World, registry: &TypeRegistry, dt: f32) -> Result<usize> {
        let mut affected = 0;

        for name in script_names(world) {
            if !self.runtimes.contains_key(&name) {
                self.load(&name)?;
            }
            let runtime = self.runtimes.get_mut(&name).expect("laddad ovan");
            affected += run_script_general(world, runtime, registry, &name, dt)?;
        }

        Ok(affected)
    }

    fn load(&mut self, name: &str) -> Result<()> {
        let source = self.location.read(name)?;

        let mut runtime = ScriptRuntime::new(&self.wasm)?;
        runtime.load_typescript(&source, name)?;
        self.runtimes.insert(name.to_string(), runtime);
        Ok(())
    }
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
