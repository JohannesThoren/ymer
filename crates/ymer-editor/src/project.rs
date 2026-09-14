//! Projekt på disk.
//!
//! ```text
//! projects/demo-spel-1/
//!   project.ron        namn och startscen
//!   scenes/main.ron
//!   scripts/*.ts
//!   textures/*.png
//!   prefabs/*.ron
//! ```
//!
//! Allt editorn bläddrar i ligger under projektroten. Att importera en
//! textur betyder alltid att filen kopieras in hit – inga sökvägar ut i
//! användarens filsystem, annars går projektet inte att flytta.

use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};

pub const PROJECT_FILE: &str = "project.ron";
pub const FOLDERS: [&str; 5] = ["scenes", "scripts", "textures", "prefabs", "models"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    /// Scen som öppnas när projektet laddas, relativt projektroten.
    pub start_scene: String,
}

#[derive(Debug, Clone)]
pub struct ProjectHandle {
    pub root: PathBuf,
    pub project: Project,
}

impl ProjectHandle {
    pub fn path(&self, relative: impl AsRef<Path>) -> PathBuf {
        self.root.join(relative)
    }

    pub fn scripts_dir(&self) -> PathBuf {
        self.root.join("scripts")
    }

    pub fn start_scene(&self) -> PathBuf {
        self.root.join(&self.project.start_scene)
    }

    /// Sökväg relativt projektroten – det är så skript och scener
    /// refererar till varandra.
    pub fn relative(&self, path: &Path) -> String {
        path.strip_prefix(&self.root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/")
    }
}

/// Alla projekt i en katalog, sorterade på namn.
pub fn list(projects_dir: impl AsRef<Path>) -> Vec<ProjectHandle> {
    let Ok(entries) = std::fs::read_dir(projects_dir.as_ref()) else {
        return Vec::new();
    };

    let mut found: Vec<ProjectHandle> = entries
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| open(entry.path()).ok())
        .collect();

    found.sort_by(|a, b| a.project.name.cmp(&b.project.name));
    found
}

pub fn open(root: impl AsRef<Path>) -> Result<ProjectHandle> {
    let root = root.as_ref().to_path_buf();
    let text = std::fs::read_to_string(root.join(PROJECT_FILE))
        .map_err(|err| anyhow!("{}: {err}", root.display()))?;
    let project: Project = ron::from_str(&text).map_err(|err| anyhow!("{err}"))?;
    Ok(ProjectHandle { root, project })
}

/// Skapar katalogstrukturen plus en startscen och ett startskript, så att
/// ett nytt projekt går att trycka Spela på direkt.
pub fn create(projects_dir: impl AsRef<Path>, name: &str) -> Result<ProjectHandle> {
    let slug: String = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let slug = slug.trim_matches('-').to_string();
    anyhow::ensure!(!slug.is_empty(), "projektnamnet måste innehålla bokstäver");

    let root = projects_dir.as_ref().join(&slug);
    anyhow::ensure!(!root.exists(), "{} finns redan", root.display());

    for folder in FOLDERS {
        std::fs::create_dir_all(root.join(folder))?;
    }

    let project = Project {
        name: name.to_string(),
        start_scene: "scenes/main.ron".to_string(),
    };
    let config = ron::ser::to_string_pretty(&project, ron::ser::PrettyConfig::new())?;
    std::fs::write(root.join(PROJECT_FILE), config)?;

    std::fs::write(root.join("scripts/spin.ts"), STARTER_SCRIPT)?;
    std::fs::write(root.join("scenes/main.ron"), STARTER_SCENE)?;

    Ok(ProjectHandle { root, project })
}

/// Meshar refereras med assetnamn, inte index – scenen bryr sig inte om
/// i vilken ordning något laddas.
const STARTER_SCENE: &str = r#"Scene(
  entities: [
    SceneEntity(
      id: 0,
      components: {
        "Camera": (fov_y_radians:1.0471976,z_near:0.1,z_far:1000.0),
        "Name": ("Main Camera"),
        "Transform": (translation:(0.0,4.0,9.0),rotation:(-0.17,0.0,0.0,0.985),scale:(1.0,1.0,1.0)),
      },
    ),
    SceneEntity(
      id: 1,
      components: {
        "MeshInstance": (mesh:"builtin/plane",color:(r:0.35,g:0.36,b:0.4,a:1.0)),
        "Name": ("Ground"),
        "Transform": (translation:(0.0,-0.5,0.0),rotation:(0.0,0.0,0.0,1.0),scale:(1.0,1.0,1.0)),
      },
    ),
    SceneEntity(
      id: 2,
      components: {
        "MeshInstance": (mesh:"builtin/cube",color:(r:0.9,g:0.9,b:0.95,a:1.0)),
        "Name": ("Kub"),
        "Script": ("spin.ts"),
        "Transform": (translation:(0.0,0.5,0.0),rotation:(0.0,0.0,0.0,1.0),scale:(1.0,1.0,1.0)),
      },
    ),
  ],
)
"#;

const STARTER_SCRIPT: &str = r#"// Startskript. Spara filen medan spelet kör så laddas det om direkt.

let elapsed: number = 0;

export function update(dt: number, entities: Entity[]): void {
  elapsed += dt;

  for (const e of entities) {
    const t = e.Transform;
    if (!t) continue;

    engine.rotate(t.rotation, 0, 1, 0, dt * 1.2);
    t.translation[1] = 0.5 + Math.sin(elapsed * 2) * 0.4;
  }
}
"#;
