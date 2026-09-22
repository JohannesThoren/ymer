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
        Self::copy_model_resources(source, &target)?;
        Ok(target)
    }

    /// En modellfil är sällan ensam. En `.gltf` är bara JSON med geometrin
    /// i en `.bin` bredvid och texturerna i bildfiler bredvid den – men
    /// även en `.glb` kan peka ut sina bilder, trots att formatet finns
    /// just för att bädda in allt. Kenneys paket gör precis det.
    ///
    /// Kopieras bara den utpekade filen hamnar modellen i projektet utan
    /// sina bilder och ritas tyst med den vita 1x1-pixeln, alltså helt
    /// färglös. Sökvägarna behåller sin form relativt modellen, så att
    /// `uri`-fälten fortsätter peka rätt.
    fn copy_model_resources(source: &Path, target: &Path) -> Result<()> {
        let Some(document) = read_gltf_json(source)? else {
            return Ok(());
        };
        let (Some(source_dir), Some(target_dir)) = (source.parent(), target.parent()) else {
            return Ok(());
        };

        for uri in gltf_uris(&document) {
            // Inbäddad data och webbadresser har ingen fil att kopiera.
            if uri.starts_with("data:") || uri.contains("://") {
                continue;
            }
            let relative = PathBuf::from(percent_decode(&uri));
            if relative.is_absolute() || relative.components().any(|c| c.as_os_str() == "..") {
                // Projektet ska vara flyttbart; en fil vi lägger utanför
                // roten hade brutit det löftet.
                log::warn!(
                    "{}: hoppar över {uri} (utanför modellens katalog)",
                    source.display()
                );
                continue;
            }
            let from = source_dir.join(&relative);
            if !from.is_file() {
                log::warn!("{}: {uri} saknas", source.display());
                continue;
            }
            let to = target_dir.join(&relative);
            if let Some(parent) = to.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(&from, &to)?;
        }
        Ok(())
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

/// JSON-dokumentet ur en modellfil, eller None om filen inte är en modell.
///
/// En `.gltf` *är* JSON. En `.glb` är en binär behållare: 12 byte huvud,
/// sedan chunkar, och den första är alltid JSON:en.
fn read_gltf_json(path: &Path) -> Result<Option<serde_json::Value>> {
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_lowercase)
        .unwrap_or_default();

    let text = match extension.as_str() {
        "gltf" => std::fs::read_to_string(path)?,
        "glb" => {
            let bytes = std::fs::read(path)?;
            let Some(json) = glb_json_chunk(&bytes) else {
                log::warn!("{}: ser inte ut som en glb", path.display());
                return Ok(None);
            };
            json
        }
        _ => return Ok(None),
    };

    serde_json::from_str(&text)
        .map(Some)
        .map_err(|err| anyhow!("{}: ogiltig gltf: {err}", path.display()))
}

/// Plockar ut JSON-chunken ur en glb. `None` om magin eller längderna
/// inte stämmer – en trasig fil ska inte få importen att fallera, den
/// hanteras av glTF-läsaren senare.
fn glb_json_chunk(bytes: &[u8]) -> Option<String> {
    if bytes.len() < 20 || &bytes[0..4] != b"glTF" {
        return None;
    }
    let length = u32::from_le_bytes(bytes[12..16].try_into().ok()?) as usize;
    if &bytes[16..20] != b"JSON" {
        return None;
    }
    let end = 20usize.checked_add(length)?;
    let chunk = bytes.get(20..end)?;
    Some(String::from_utf8_lossy(chunk).into_owned())
}

/// Alla `uri`-fält i en gltf: buffertar och bilder. Fältet heter likadant
/// på båda, så en enkel genomgång av de två listorna räcker.
fn gltf_uris(document: &serde_json::Value) -> Vec<String> {
    let mut found = Vec::new();
    for key in ["buffers", "images"] {
        let Some(list) = document.get(key).and_then(serde_json::Value::as_array) else {
            continue;
        };
        for item in list {
            if let Some(uri) = item.get("uri").and_then(serde_json::Value::as_str) {
                found.push(uri.to_string());
            }
        }
    }
    found
}

/// gltf-filer rymmer procentkodade sökvägar ("min%20textur.png").
fn percent_decode(uri: &str) -> String {
    let bytes = uri.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%'
            && let Some(hex) = uri.get(index + 1..index + 3)
            && let Ok(byte) = u8::from_str_radix(hex, 16)
        {
            out.push(byte);
            index += 3;
            continue;
        }
        out.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
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

    #[test]
    fn glb_tar_med_sin_externa_textur() {
        // En .glb bäddar normalt in allt, men behöver inte: Kenneys paket
        // lägger geometrin i binärchunken och pekar ut colormap.png som en
        // fil bredvid. Kopieras bara .glb-filen laddas modellen utan sin
        // textur och ritas färglös.
        let root = project("glb_resurser");
        let source_dir = root.join("utanfor");
        std::fs::create_dir_all(source_dir.join("Textures")).unwrap();
        std::fs::write(source_dir.join("Textures/color map.png"), b"PNG").unwrap();

        let json =
            br#"{"images":[{"uri":"Textures/color%20map.png"}],"buffers":[{"byteLength":4}]}"#;
        let mut glb = Vec::new();
        glb.extend_from_slice(b"glTF");
        glb.extend_from_slice(&2u32.to_le_bytes());
        glb.extend_from_slice(&((12 + 8 + json.len()) as u32).to_le_bytes());
        glb.extend_from_slice(&(json.len() as u32).to_le_bytes());
        glb.extend_from_slice(b"JSON");
        glb.extend_from_slice(json);
        std::fs::write(source_dir.join("vagg.glb"), &glb).unwrap();

        let mut browser = FileBrowser::new(&root);
        let placed = browser.import(&source_dir.join("vagg.glb")).unwrap();

        let models = root.join("models");
        assert_eq!(placed.parent().unwrap(), models);
        assert!(
            models.join("Textures/color map.png").is_file(),
            "texturen följde inte med, och sökvägen ska behålla sin form"
        );
    }

    #[test]
    fn gltf_tar_med_sina_filer() {
        let root = project("gltf_resurser");
        let source_dir = root.join("utanfor");
        std::fs::create_dir_all(&source_dir).unwrap();
        std::fs::write(
            source_dir.join("stol.gltf"),
            br#"{"buffers":[{"uri":"stol.bin"}],"images":[{"uri":"data:image/png;base64,AAAA"}]}"#,
        )
        .unwrap();
        std::fs::write(source_dir.join("stol.bin"), b"geometri").unwrap();

        let mut browser = FileBrowser::new(&root);
        browser.import(&source_dir.join("stol.gltf")).unwrap();

        assert!(
            root.join("models/stol.bin").is_file(),
            "bufferten följde inte med"
        );
    }
}
