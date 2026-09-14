//! Export: projektmapp in, körbar spelmapp ut.
//!
//! Resultatet är tre filer och ingenting annat:
//!
//! ```text
//! <namn>/
//!   <namn>[.exe]       runnern, alltså motorn
//!   script_host.wasm   QuickJS
//!   game.pak           allt spelinnehåll
//! ```
//!
//! Att uppdatera motorn i ett redan utgivet spel betyder att byta ut
//! runnern och skriptvärden. Arkivet rörs inte – dess `format_version`
//! är kontraktet som håller dem ihop.

use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow, bail};
use ymer_pak::PakWriter;

use crate::project::ProjectHandle;

/// Filer och kataloger som aldrig ska följa med i ett utgivet spel.
/// `engine.d.ts` och `tsconfig.json` är till för din kodeditor, och
/// `.pak`-filer i projektet är tidigare exporter.
const EXCLUDED: [&str; 3] = ["engine.d.ts", "tsconfig.json", ".pak"];

pub struct ExportReport {
    pub output_dir: PathBuf,
    pub files_packed: usize,
    pub raw_bytes: u64,
    pub archive_bytes: u64,
    pub runner_bytes: u64,
}

impl ExportReport {
    pub fn summary(&self) -> String {
        let ratio = self.archive_bytes as f64 / self.raw_bytes.max(1) as f64 * 100.0;
        format!(
            "exporterade {} filer – arkiv {:.1} kB ({ratio:.0}% av råstorleken), totalt {:.1} MB",
            self.files_packed,
            self.archive_bytes as f64 / 1024.0,
            (self.archive_bytes + self.runner_bytes) as f64 / (1024.0 * 1024.0),
        )
    }
}

/// Letar upp runnern. I ett installerat motorpaket ligger den bredvid
/// editorn; under utveckling i `target/`.
pub fn find_runner() -> Option<PathBuf> {
    let name = if cfg!(windows) {
        "ymer-play.exe"
    } else {
        "ymer-play"
    };

    let mut candidates = Vec::new();
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        candidates.push(dir.join(name));
    }
    // Release före debug: exporterar man ett spel vill man ha det snabba.
    candidates.push(PathBuf::from("target/release").join(name));
    candidates.push(PathBuf::from("target/debug").join(name));

    candidates.into_iter().find(|path| path.is_file())
}

/// Samma sak för skriptvärden.
pub fn find_script_host() -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        candidates.push(dir.join("script_host.wasm"));
    }
    candidates.push(PathBuf::from("assets/script_host.wasm"));

    candidates.into_iter().find(|path| path.is_file())
}

fn should_include(name: &str) -> bool {
    !EXCLUDED.iter().any(|excluded| name.ends_with(excluded))
}

/// Bygger utgivningsmappen.
pub fn export(handle: &ProjectHandle, output_dir: &Path) -> Result<ExportReport> {
    let runner = find_runner().ok_or_else(|| {
        anyhow!("hittade ingen runner – bygg den med `cargo build --release -p ymer-play`")
    })?;
    let script_host = find_script_host().ok_or_else(|| anyhow!("hittade inte script_host.wasm"))?;

    if output_dir.exists() && output_dir == handle.root {
        bail!("exportera inte över projektet självt");
    }
    std::fs::create_dir_all(output_dir)?;

    // --- arkivet ---------------------------------------------------------
    let mut writer = PakWriter::new();
    let mut files_packed = 0;
    collect(&mut writer, &handle.root, &handle.root, &mut files_packed)?;
    anyhow::ensure!(
        files_packed > 0,
        "projektet innehåller inga filer att packa"
    );

    let (raw_bytes, archive_bytes) = writer.write(&output_dir.join("game.pak"))?;

    // --- runnern, döpt efter spelet --------------------------------------
    let executable = sanitize(&handle.project.name);
    let executable = if cfg!(windows) {
        format!("{executable}.exe")
    } else {
        executable
    };
    let target = output_dir.join(&executable);
    std::fs::copy(&runner, &target)?;

    // Kopian måste vara körbar; std::fs::copy bevarar rättigheter på Unix,
    // men om målet fanns sedan tidigare kan de ha varit andra.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o755))?;
    }

    std::fs::copy(&script_host, output_dir.join("script_host.wasm"))?;

    let runner_bytes = std::fs::metadata(&target)?.len();

    Ok(ExportReport {
        output_dir: output_dir.to_path_buf(),
        files_packed,
        raw_bytes,
        archive_bytes,
        runner_bytes,
    })
}

fn collect(writer: &mut PakWriter, root: &Path, dir: &Path, count: &mut usize) -> Result<()> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Ok(());
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(writer, root, &path, count)?;
            continue;
        }

        let name = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        if !should_include(&name) {
            continue;
        }

        writer.add(name, std::fs::read(&path)?);
        *count += 1;
    }
    Ok(())
}

/// Projektnamn till filnamn: mellanslag och specialtecken bort.
fn sanitize(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let trimmed = cleaned.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "game".to_string()
    } else {
        trimmed
    }
}
