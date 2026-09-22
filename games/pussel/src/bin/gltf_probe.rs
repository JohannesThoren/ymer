//! Vad överlever en modellimport?
//!
//! Skriver ut nodträdet med mesh, textur och färg per nod, och slår upp
//! namnen i assetregistret. Att en textur registreras räcker inte – namnet
//! i `MeshInstance` måste hitta tillbaka till den, och gör det inte det
//! ritas objektet tyst med den vita 1x1-pixeln i stället.
//!
//!     cargo run -p pussel --bin gltf_probe --release -- modell.glb

use ymer_render::{Assets, GltfNode, Renderer, import_gltf};

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    let path = std::env::args()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("ange en .gltf eller .glb"))?;
    let path = std::path::PathBuf::from(path);
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "modell".to_string());

    let mut renderer = pollster::block_on(Renderer::new_offscreen(64, 64))?;
    let mut assets = Assets::new(&mut renderer);

    let asset = import_gltf(&mut renderer, &mut assets, &path, &name)?;
    println!("meshar:   {:?}", asset.mesh_names);
    println!("texturer: {:?}", asset.texture_names);

    fn visit(node: &GltfNode, assets: &Assets, depth: usize) {
        let indent = "  ".repeat(depth);
        let c = node.color;
        println!(
            "{indent}{} mesh={:?} textur={:?} färg=({:.2} {:.2} {:.2} {:.2})",
            node.name, node.mesh, node.texture, c.r, c.g, c.b, c.a
        );
        println!(
            "{indent}  pos={:?} skala={:?}",
            [node.translation.x, node.translation.y, node.translation.z]
                .map(|v| (v * 100.0).round() / 100.0),
            [node.scale.x, node.scale.y, node.scale.z].map(|v| (v * 100.0).round() / 100.0),
        );
        if let Some(mesh) = &node.mesh {
            println!("{indent}  mesh-id: {:?}", assets.mesh(mesh));
        }
        if let Some(texture) = &node.texture {
            println!(
                "{indent}  textur-id: {:?} (0 = vit platshållare)",
                assets.texture(texture)
            );
        }
        for child in &node.children {
            visit(child, assets, depth + 1);
        }
    }

    for node in &asset.roots {
        visit(node, &assets, 0);
    }
    Ok(())
}
