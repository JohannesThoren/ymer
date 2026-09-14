//! Filutforskaren. Bläddrar i projektroten, skapar filer och tar emot
//! filer som släpps på fönstret.

use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};

/// Vad användaren gjorde i utforskaren den här framen.
#[derive(Debug, Clone)]
pub enum FileAction {
    /// Dubbelklick på en fil – editorn avgör vad som ska hända med den.
    Open(PathBuf),
    Selected(PathBuf),
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
}

pub struct FileBrowser {
    root: PathBuf,
    cwd: PathBuf,
    pub selected: Option<PathBuf>,
    pub new_name: String,
    pub status: String,
}

impl FileBrowser {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        Self {
            cwd: root.clone(),
            root,
            selected: None,
            new_name: String::new(),
            status: String::new(),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn cwd(&self) -> &Path {
        &self.cwd
    }

    /// Sökvägen som visas i rubriken, alltid relativt projektroten.
    pub fn breadcrumb(&self) -> String {
        let relative = self.cwd.strip_prefix(&self.root).unwrap_or(Path::new(""));
        if relative.as_os_str().is_empty() {
            "/".to_string()
        } else {
            format!("/{}", relative.to_string_lossy().replace('\\', "/"))
        }
    }

    /// Kataloger först, sedan filer, båda i bokstavsordning.
    pub fn entries(&self) -> Vec<Entry> {
        let Ok(read) = std::fs::read_dir(&self.cwd) else {
            return Vec::new();
        };

        let mut entries: Vec<Entry> = read
            .flatten()
            .map(|entry| {
                let path = entry.path();
                Entry {
                    name: entry.file_name().to_string_lossy().into_owned(),
                    is_dir: path.is_dir(),
                    path,
                }
            })
            .collect();

        entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name)));
        entries
    }

    pub fn enter(&mut self, path: &Path) {
        if path.is_dir() {
            self.cwd = path.to_path_buf();
            self.selected = None;
        }
    }

    /// Upp en nivå, men aldrig utanför projektet.
    pub fn go_up(&mut self) {
        if self.cwd == self.root {
            return;
        }
        if let Some(parent) = self.cwd.parent() {
            self.cwd = parent.to_path_buf();
            self.selected = None;
        }
    }

    /// Skapar en ny fil med startinnehåll beroende på ändelse.
    pub fn create_file(&mut self, name: &str) -> Result<PathBuf> {
        anyhow::ensure!(!name.trim().is_empty(), "filnamnet är tomt");
        let name = if name.contains('.') {
            name.to_string()
        } else {
            format!("{name}.ts")
        };

        let path = self.cwd.join(&name);
        anyhow::ensure!(!path.exists(), "{name} finns redan");

        let template = if name.ends_with(".ts") {
            NEW_SCRIPT
        } else {
            ""
        };
        std::fs::write(&path, template)?;
        Ok(path)
    }

    pub fn create_folder(&mut self, name: &str) -> Result<PathBuf> {
        anyhow::ensure!(!name.trim().is_empty(), "mappnamnet är tomt");
        let path = self.cwd.join(name.trim());
        std::fs::create_dir_all(&path)?;
        Ok(path)
    }

    /// Standardmapp för en filtyp. Assets hör hemma på bestämda ställen –
    /// annars hittar varken modell-omladdningen vid projektöppning eller
    /// texturladdningen dem nästa gång.
    pub fn folder_for(&self, source: &Path) -> PathBuf {
        let extension = source
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_lowercase)
            .unwrap_or_default();

        let folder = match extension.as_str() {
            "glb" | "gltf" => "models",
            "png" | "jpg" | "jpeg" => "textures",
            "ts" => "scripts",
            // Okänd typ: låt den ligga där användaren står.
            _ => return self.cwd.clone(),
        };
        self.root.join(folder)
    }

    /// Kopierar in en fil utifrån. Projektet ska vara flyttbart, så
    /// editorn refererar aldrig till filer utanför roten.
    pub fn import(&mut self, source: &Path) -> Result<PathBuf> {
        let folder = self.folder_for(source);
        self.import_into(source, &folder)
    }

    /// Som `import`, men till en bestämd mapp.
    pub fn import_into(&mut self, source: &Path, folder: &Path) -> Result<PathBuf> {
        let name = source
            .file_name()
            .ok_or_else(|| anyhow!("{} har inget filnamn", source.display()))?;

        std::fs::create_dir_all(folder)?;
        let mut target = folder.join(name);
        // Krocka inte med befintliga filer – lägg på en siffra istället.
        let mut counter = 1;
        while target.exists() {
            let stem = source.file_stem().map(|s| s.to_string_lossy().into_owned());
            let extension = source.extension().map(|s| s.to_string_lossy().into_owned());
            let candidate = match (&stem, &extension) {
                (Some(stem), Some(extension)) => format!("{stem} {counter}.{extension}"),
                (Some(stem), None) => format!("{stem} {counter}"),
                _ => format!("import {counter}"),
            };
            target = folder.join(candidate);
            counter += 1;
        }

        std::fs::copy(source, &target)?;
        Ok(target)
    }

    pub fn delete(&mut self, path: &Path) -> Result<()> {
        anyhow::ensure!(path.starts_with(&self.root), "ligger utanför projektet");
        if path.is_dir() {
            std::fs::remove_dir_all(path)?;
        } else {
            std::fs::remove_file(path)?;
        }
        if self.selected.as_deref() == Some(path) {
            self.selected = None;
        }
        Ok(())
    }
}

const NEW_SCRIPT: &str = r#"// Nytt skript. Sätt en Script-komponent på en entitet och peka hit.

export function update(dt: number, entities: Entity[]): void {
  for (const e of entities) {
    const t = e.Transform;
    if (!t) continue;

    engine.rotate(t.rotation, 0, 1, 0, dt);
  }
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    fn project(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("browser_test_{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        for folder in ["models", "textures", "scripts", "prefabs", "scenes"] {
            std::fs::create_dir_all(dir.join(folder)).unwrap();
        }
        dir
    }

    /// Regression: en släppt .glb hamnade i projektroten istället för i
    /// models/, eftersom importen kopierade till filutforskarens nuvarande
    /// mapp. Modell-omladdningen vid projektöppning hittade den då aldrig.
    #[test]
    fn assets_hamnar_i_ratt_mapp_oavsett_var_man_star() {
        let root = project("routing");
        let source_dir = std::env::temp_dir().join("browser_test_source");
        std::fs::create_dir_all(&source_dir).unwrap();

        let model = source_dir.join("robot.glb");
        std::fs::write(&model, b"glTF-platshallare").unwrap();
        let texture = source_dir.join("tegel.png");
        std::fs::write(&texture, b"PNG-platshallare").unwrap();
        let script = source_dir.join("spelare.ts").to_path_buf();
        std::fs::write(&script, b"export function update() {}").unwrap();

        let mut browser = FileBrowser::new(&root);
        // Står i projektroten – precis som när man just öppnat ett projekt.
        assert_eq!(browser.cwd(), root.as_path());

        let placed_model = browser.import(&model).unwrap();
        let placed_texture = browser.import(&texture).unwrap();
        let placed_script = browser.import(&script).unwrap();

        assert_eq!(placed_model.parent().unwrap(), root.join("models"));
        assert_eq!(placed_texture.parent().unwrap(), root.join("textures"));
        assert_eq!(placed_script.parent().unwrap(), root.join("scripts"));

        // Inget ska ha hamnat i roten.
        let in_root: Vec<String> = std::fs::read_dir(&root)
            .unwrap()
            .flatten()
            .filter(|e| e.path().is_file())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert!(
            in_root.is_empty(),
            "filer hamnade i projektroten: {in_root:?}"
        );
    }

    #[test]
    fn okand_filtyp_hamnar_dar_man_star() {
        let root = project("unknown");
        let source = std::env::temp_dir().join("browser_test_unknown.xyz");
        std::fs::write(&source, b"data").unwrap();

        let mut browser = FileBrowser::new(&root);
        browser.enter(&root.join("scenes"));

        let placed = browser.import(&source).unwrap();
        assert_eq!(placed.parent().unwrap(), root.join("scenes"));
    }

    #[test]
    fn namnkrock_ger_nytt_namn_i_samma_mapp() {
        let root = project("collision");
        let source = std::env::temp_dir().join("browser_test_collide.glb");
        std::fs::write(&source, b"glTF").unwrap();

        let mut browser = FileBrowser::new(&root);
        let first = browser.import(&source).unwrap();
        let second = browser.import(&source).unwrap();

        assert_ne!(first, second);
        assert_eq!(second.parent().unwrap(), root.join("models"));
        assert!(first.exists() && second.exists());
    }
}
