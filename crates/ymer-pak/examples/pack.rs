//! Packar en katalog till ett .pak-arkiv och verifierar resultatet.
//!
//!     cargo run -p ymer-pak --example pack -- <katalog> <arkiv.pak>

use ymer_core::AssetSource;
use ymer_pak::{LooseFiles, PakArchive, PakWriter};

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let dir = args
        .next()
        .unwrap_or_else(|| "projects/demo-spel-1".to_string());
    let output = args.next().unwrap_or_else(|| {
        std::env::temp_dir()
            .join("game.pak")
            .to_string_lossy()
            .into_owned()
    });

    let dir = std::path::Path::new(&dir);
    let output = std::path::Path::new(&output);

    let mut writer = PakWriter::new();
    let count = writer.add_dir(dir, dir)?;
    let (raw, size) = writer.write(output)?;

    println!("packade {count} filer från {}", dir.display());
    println!(
        "  rått:   {:>9} byte\n  arkiv:  {:>9} byte  ({:.1}% av originalet)",
        raw,
        size,
        size as f64 / raw.max(1) as f64 * 100.0
    );

    // Verifiera att varje fil läses tillbaka exakt – crc32 kollas internt.
    let loose = LooseFiles::new(dir);
    let pak = PakArchive::open(output)?;
    println!("  version: {}", pak.format_version());

    let mut checked = 0;
    for path in pak.paths().map(str::to_string).collect::<Vec<_>>() {
        let from_pak = pak.read(&path)?;
        let from_disk = loose.read(&path)?;
        anyhow::ensure!(
            from_pak == from_disk,
            "{path} skiljer sig mellan disk och arkiv"
        );
        checked += 1;
    }
    println!("  verifierade {checked} filer mot originalen: identiska");

    Ok(())
}
