//! Exporterar ett projekt till en körbar spelmapp och verifierar
//! resultatet – samma kodväg som Exportera-knappen i editorn.
//!
//!     cargo run -p ymer-editor --example export_demo -- <projekt> <utmapp>

use ymer_core::AssetSource;
use ymer_editor::{export, project};
use ymer_pak::PakArchive;

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    let mut args = std::env::args().skip(1);
    let project_dir = args.next().unwrap_or_else(|| {
        std::env::temp_dir()
            .join("testprojekt")
            .to_string_lossy()
            .into_owned()
    });
    let output = args.next().unwrap_or_else(|| {
        std::env::temp_dir()
            .join("export-test")
            .to_string_lossy()
            .into_owned()
    });

    let handle = project::open(&project_dir)?;
    println!("projekt: {}", handle.project.name);

    let report = export::export(&handle, std::path::Path::new(&output))?;
    println!("{}", report.summary());

    println!("\ninnehåll i {}:", report.output_dir.display());
    let mut entries: Vec<_> = std::fs::read_dir(&report.output_dir)?.flatten().collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in &entries {
        let size = entry.metadata()?.len();
        println!(
            "  {:24} {:>12} byte",
            entry.file_name().to_string_lossy(),
            size
        );
    }

    // Arkivet ska innehålla innehållet men inte editorns hjälpfiler.
    let pak = PakArchive::open(report.output_dir.join("game.pak"))?;
    println!("\narkivet innehåller {} filer:", pak.len());
    for path in pak.paths() {
        println!("  {path}");
    }

    anyhow::ensure!(pak.exists("project.ron"), "project.ron saknas i arkivet");
    anyhow::ensure!(
        !pak.paths().any(|p| p.ends_with("engine.d.ts")),
        "engine.d.ts skulle inte ha packats – den är till för kodeditorn"
    );
    anyhow::ensure!(
        !pak.paths().any(|p| p.ends_with(".pak")),
        "en tidigare export har packats in i den nya"
    );

    println!("\nkontroller: OK");
    Ok(())
}
