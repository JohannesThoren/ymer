//! `.pak` – motorns arkivformat för exporterade spel.
//!
//! ```text
//! header    "JPAK" | format_version u32 | entry_count u32 | index_offset u64
//! data      alla filers innehåll, i sorterad ordning
//! index     per fil: sökväg, offset, storlekar, komprimering, crc32
//! ```
//!
//! Indexet ligger sist så att datan kan strömmas ut medan man skriver,
//! utan att veta offsets i förväg.
//!
//! Designval värda att känna till:
//!
//! - **Per fil, inte hela arkivet.** Enskilda filer kan läsas utan att
//!   packa upp allt, vilket är hela poängen med ett arkiv.
//! - **Komprimering bara när den lönar sig.** PNG och JPEG är redan
//!   komprimerade och blir *större* av zstd. RON och TypeScript krymper
//!   till en bråkdel. Tröskeln avgör per fil.
//! - **Sorterad ordning.** Samma indata ger samma arkiv, byte för byte.
//! - **crc32 per fil.** En trasig fil ger ett begripligt fel i stället för
//!   en krasch djupt inne i PNG-avkodaren.

use std::collections::BTreeMap;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

use anyhow::{Result, anyhow, bail};
use ymer_core::AssetSource;

const MAGIC: &[u8; 4] = b"JPAK";

/// Höjs bara vid brytande ändringar. En nyare runner ska kunna läsa
/// äldre arkiv – det är här bakåtkompatibiliteten bor, inte i någon ABI.
pub const FORMAT_VERSION: u32 = 1;

const COMPRESSION_NONE: u8 = 0;
const COMPRESSION_ZSTD: u8 = 1;

/// Filändelser som redan är komprimerade. Att zstd:a dem kostar tid och
/// ger normalt negativ vinst.
const ALREADY_COMPRESSED: [&str; 6] = ["png", "jpg", "jpeg", "webp", "ogg", "mp3"];

/// Komprimerat resultat måste spara minst så här mycket för att användas.
const COMPRESSION_THRESHOLD: f32 = 0.95;

#[derive(Debug, Clone)]
struct Entry {
    offset: u64,
    stored_size: u64,
    original_size: u64,
    compression: u8,
    crc32: u32,
}

// ------------------------------------------------------------- skrivare

pub struct PakWriter {
    entries: BTreeMap<String, Vec<u8>>,
}

impl Default for PakWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl PakWriter {
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }

    pub fn add(&mut self, path: impl Into<String>, data: Vec<u8>) {
        self.entries.insert(normalize(&path.into()), data);
    }

    /// Lägger till allt under `dir`, namngivet relativt `root`.
    /// Returnerar antal tillagda filer.
    pub fn add_dir(&mut self, root: &Path, dir: &Path) -> Result<usize> {
        let mut count = 0;
        let Ok(entries) = std::fs::read_dir(dir) else {
            return Ok(0);
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                count += self.add_dir(root, &path)?;
                continue;
            }
            let name = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            self.add(name, std::fs::read(&path)?);
            count += 1;
        }
        Ok(count)
    }

    /// Skriver arkivet. Returnerar (okomprimerad summa, filstorlek).
    pub fn write(&self, path: &Path) -> Result<(u64, u64)> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut file = std::io::BufWriter::new(std::fs::File::create(path)?);

        file.write_all(MAGIC)?;
        file.write_all(&FORMAT_VERSION.to_le_bytes())?;
        file.write_all(&(self.entries.len() as u32).to_le_bytes())?;
        // Platshållare för index_offset; fylls i när datan är skriven.
        file.write_all(&0u64.to_le_bytes())?;

        let mut offset: u64 = 4 + 4 + 4 + 8;
        let mut index: Vec<(String, Entry)> = Vec::with_capacity(self.entries.len());
        let mut raw_total = 0u64;

        for (name, data) in &self.entries {
            let crc32 = crc32fast::hash(data);
            let (payload, compression) = compress(name, data)?;

            file.write_all(&payload)?;
            let stored_size = payload.len() as u64;

            index.push((
                name.clone(),
                Entry {
                    offset,
                    stored_size,
                    original_size: data.len() as u64,
                    compression,
                    crc32,
                },
            ));

            offset += stored_size;
            raw_total += data.len() as u64;
        }

        let index_offset = offset;
        for (name, entry) in &index {
            let bytes = name.as_bytes();
            file.write_all(&(bytes.len() as u16).to_le_bytes())?;
            file.write_all(bytes)?;
            file.write_all(&entry.offset.to_le_bytes())?;
            file.write_all(&entry.stored_size.to_le_bytes())?;
            file.write_all(&entry.original_size.to_le_bytes())?;
            file.write_all(&[entry.compression])?;
            file.write_all(&entry.crc32.to_le_bytes())?;
        }

        // Tillbaka till headern och fyll i var indexet hamnade.
        file.flush()?;
        let mut file = file.into_inner()?;
        file.seek(SeekFrom::Start(12))?;
        file.write_all(&index_offset.to_le_bytes())?;
        file.flush()?;

        let size = std::fs::metadata(path)?.len();
        Ok((raw_total, size))
    }
}

fn compress(name: &str, data: &[u8]) -> Result<(Vec<u8>, u8)> {
    let extension = name.rsplit('.').next().unwrap_or("").to_lowercase();
    if ALREADY_COMPRESSED.contains(&extension.as_str()) || data.is_empty() {
        return Ok((data.to_vec(), COMPRESSION_NONE));
    }

    let compressed = zstd::encode_all(data, 10)?;
    // Bara om det faktiskt lönar sig – annars betalar man uppackningstid
    // för ingenting.
    if (compressed.len() as f32) < data.len() as f32 * COMPRESSION_THRESHOLD {
        Ok((compressed, COMPRESSION_ZSTD))
    } else {
        Ok((data.to_vec(), COMPRESSION_NONE))
    }
}

// --------------------------------------------------------------- läsare

pub struct PakArchive {
    path: std::path::PathBuf,
    entries: BTreeMap<String, Entry>,
    format_version: u32,
}

impl PakArchive {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let mut file = std::fs::File::open(&path)?;

        let mut magic = [0u8; 4];
        file.read_exact(&mut magic)?;
        if &magic != MAGIC {
            bail!("{} är inte ett pak-arkiv", path.display());
        }

        let format_version = read_u32(&mut file)?;
        if format_version > FORMAT_VERSION {
            bail!(
                "{} är version {format_version}, men den här motorn förstår som högst {FORMAT_VERSION} – uppdatera motorn",
                path.display()
            );
        }

        let entry_count = read_u32(&mut file)?;
        let index_offset = read_u64(&mut file)?;

        file.seek(SeekFrom::Start(index_offset))?;
        let mut entries = BTreeMap::new();
        for _ in 0..entry_count {
            let mut length = [0u8; 2];
            file.read_exact(&mut length)?;
            let mut name = vec![0u8; u16::from_le_bytes(length) as usize];
            file.read_exact(&mut name)?;
            let name = String::from_utf8(name)
                .map_err(|err| anyhow!("ogiltigt filnamn i arkivet: {err}"))?;

            let entry = Entry {
                offset: read_u64(&mut file)?,
                stored_size: read_u64(&mut file)?,
                original_size: read_u64(&mut file)?,
                compression: {
                    let mut byte = [0u8; 1];
                    file.read_exact(&mut byte)?;
                    byte[0]
                },
                crc32: read_u32(&mut file)?,
            };
            entries.insert(name, entry);
        }

        Ok(Self {
            path,
            entries,
            format_version,
        })
    }

    pub fn format_version(&self) -> u32 {
        self.format_version
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn paths(&self) -> impl Iterator<Item = &str> {
        self.entries.keys().map(String::as_str)
    }
}

impl AssetSource for PakArchive {
    fn read(&self, path: &str) -> std::io::Result<Vec<u8>> {
        let name = normalize(path);
        let entry = self.entries.get(&name).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("{name} finns inte i {}", self.path.display()),
            )
        })?;

        let mut file = std::fs::File::open(&self.path)?;
        file.seek(SeekFrom::Start(entry.offset))?;
        let mut stored = vec![0u8; entry.stored_size as usize];
        file.read_exact(&mut stored)?;

        let data = match entry.compression {
            COMPRESSION_NONE => stored,
            COMPRESSION_ZSTD => zstd::decode_all(&stored[..])?,
            other => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("okänd komprimering {other} för {name}"),
                ));
            }
        };

        if crc32fast::hash(&data) != entry.crc32 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("{name} är skadad (crc32 stämmer inte)"),
            ));
        }

        Ok(data)
    }

    fn list(&self, prefix: &str, extension: &str) -> Vec<String> {
        let prefix = normalize(prefix);
        self.entries
            .keys()
            .filter(|name| name.starts_with(&prefix))
            .filter(|name| extension.is_empty() || name.ends_with(&format!(".{extension}")))
            .cloned()
            .collect()
    }

    fn exists(&self, path: &str) -> bool {
        self.entries.contains_key(&normalize(path))
    }
}

// ---------------------------------------------------------- lösa filer

/// Läser direkt från en katalog. Det editorn använder.
pub struct LooseFiles {
    root: std::path::PathBuf,
}

impl LooseFiles {
    pub fn new(root: impl Into<std::path::PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
}

impl AssetSource for LooseFiles {
    fn read(&self, path: &str) -> std::io::Result<Vec<u8>> {
        std::fs::read(self.root.join(normalize(path)))
    }

    fn list(&self, prefix: &str, extension: &str) -> Vec<String> {
        fn walk(dir: &Path, root: &Path, extension: &str, found: &mut Vec<String>) {
            let Ok(entries) = std::fs::read_dir(dir) else {
                return;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, root, extension, found);
                } else {
                    let matches = extension.is_empty()
                        || path.extension().is_some_and(|found| found == extension);
                    if matches {
                        let name = path.strip_prefix(root).unwrap_or(&path).to_string_lossy();
                        found.push(name.replace('\\', "/"));
                    }
                }
            }
        }

        let mut found = Vec::new();
        walk(
            &self.root.join(normalize(prefix)),
            &self.root,
            extension,
            &mut found,
        );
        found.sort();
        found
    }

    fn exists(&self, path: &str) -> bool {
        self.root.join(normalize(path)).exists()
    }
}

// --------------------------------------------------------------- internt

/// Windows-separatorer och inledande `./` bort, så att samma sträng
/// fungerar oavsett var den kom ifrån.
fn normalize(path: &str) -> String {
    path.replace('\\', "/").trim_start_matches("./").to_string()
}

fn read_u32(file: &mut std::fs::File) -> Result<u32> {
    let mut bytes = [0u8; 4];
    file.read_exact(&mut bytes)?;
    Ok(u32::from_le_bytes(bytes))
}

fn read_u64(file: &mut std::fs::File) -> Result<u64> {
    let mut bytes = [0u8; 8];
    file.read_exact(&mut bytes)?;
    Ok(u64::from_le_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("pak_test_{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn round_trip() {
        let dir = temp("round_trip");
        let archive = dir.join("game.pak");

        let mut writer = PakWriter::new();
        writer.add("scenes/main.ron", b"Scene( entities: [] )".to_vec());
        writer.add(
            "scripts/spin.ts",
            "export function update() {}".as_bytes().to_vec(),
        );
        writer.write(&archive).unwrap();

        let pak = PakArchive::open(&archive).unwrap();
        assert_eq!(pak.len(), 2);
        assert_eq!(
            pak.read_to_string("scenes/main.ron").unwrap(),
            "Scene( entities: [] )"
        );
        assert!(pak.exists("scripts/spin.ts"));
        assert!(!pak.exists("saknas.ron"));
    }

    #[test]
    fn komprimerar_text_men_inte_png() {
        let dir = temp("compression");
        let archive = dir.join("game.pak");

        // Repetitiv text komprimerar extremt bra.
        let text = "fn update() { /* samma rad om och om igen */ }\n".repeat(200);
        let mut writer = PakWriter::new();
        writer.add("scripts/stor.ts", text.clone().into_bytes());
        // "PNG" med slumpliknande innehåll: ska lagras rått.
        let png: Vec<u8> = (0..4096u32)
            .map(|i| (i.wrapping_mul(2654435761) >> 24) as u8)
            .collect();
        writer.add("textures/brus.png", png.clone());
        let (raw, size) = writer.write(&archive).unwrap();

        assert!(size < raw, "arkivet ska vara mindre än råsumman");

        let pak = PakArchive::open(&archive).unwrap();
        assert_eq!(pak.read("scripts/stor.ts").unwrap(), text.as_bytes());
        assert_eq!(pak.read("textures/brus.png").unwrap(), png);
    }

    #[test]
    fn listar_med_prefix_och_andelse() {
        let dir = temp("listing");
        let archive = dir.join("game.pak");

        let mut writer = PakWriter::new();
        writer.add("scripts/a.ts", b"a".to_vec());
        writer.add("scripts/nested/b.ts", b"b".to_vec());
        writer.add("scenes/main.ron", b"c".to_vec());
        writer.write(&archive).unwrap();

        let pak = PakArchive::open(&archive).unwrap();
        let scripts = pak.list("scripts", "ts");
        assert_eq!(scripts, vec!["scripts/a.ts", "scripts/nested/b.ts"]);
        assert_eq!(pak.list("", "ron"), vec!["scenes/main.ron"]);
    }

    #[test]
    fn upptacker_skadad_fil() {
        let dir = temp("corrupt");
        let archive = dir.join("game.pak");

        let mut writer = PakWriter::new();
        // Inkompressibel, så den lagras rått och går att peta i direkt.
        let payload: Vec<u8> = (0..2048u32)
            .map(|i| (i.wrapping_mul(2246822519) >> 24) as u8)
            .collect();
        writer.add("data/brus.png", payload);
        writer.write(&archive).unwrap();

        // Peta sönder en byte mitt i datablocket.
        let mut bytes = std::fs::read(&archive).unwrap();
        bytes[100] ^= 0xff;
        std::fs::write(&archive, bytes).unwrap();

        let pak = PakArchive::open(&archive).unwrap();
        let error = pak.read("data/brus.png").unwrap_err();
        assert!(
            error.to_string().contains("skadad"),
            "väntade crc-fel, fick: {error}"
        );
    }

    #[test]
    fn losa_filer_och_arkiv_beter_sig_lika() {
        let dir = temp("parity");
        std::fs::create_dir_all(dir.join("scripts")).unwrap();
        std::fs::write(dir.join("scripts/spin.ts"), "export function update() {}").unwrap();
        std::fs::write(dir.join("scenes.ron"), "Scene()").unwrap();
        std::fs::create_dir_all(dir.join("scenes")).unwrap();

        let mut writer = PakWriter::new();
        writer.add_dir(&dir, &dir).unwrap();
        let archive = std::env::temp_dir().join("pak_parity.pak");
        writer.write(&archive).unwrap();

        let loose = LooseFiles::new(&dir);
        let pak = PakArchive::open(&archive).unwrap();

        // Samma fråga, samma svar – oavsett källa.
        assert_eq!(
            loose.read_to_string("scripts/spin.ts").unwrap(),
            pak.read_to_string("scripts/spin.ts").unwrap()
        );
        assert_eq!(loose.list("scripts", "ts"), pak.list("scripts", "ts"));
        assert_eq!(loose.exists("saknas.txt"), pak.exists("saknas.txt"));
    }
}
