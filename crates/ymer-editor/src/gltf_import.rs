//! Kopplar glTF-import till scenformatet.
//!
//! Ligger i editorn, inte i `ymer-render`: renderaren ska inte känna till
//! `Scene` eller prefab-formatet – den vet bara om meshar och texturer.
//! Nodträdet från `import_gltf` är ren data; den här filen är översättaren
//! till entiteter.

use std::collections::BTreeMap;
use std::path::Path;

use ymer_core::{EntityName, MeshInstance, Transform};
use ymer_render::{Assets, GltfAsset, GltfNode, Renderer, import_gltf};
use ymer_scene::{Scene, SceneEntity};

/// Läser en `.gltf`/`.glb`-fil, laddar upp dess assets, och bygger en
/// prefab av nodträdet. Sparar den till `prefabs/<filnamn>.ron` och
/// returnerar sökvägen.
pub fn import_as_prefab(
    renderer: &mut Renderer,
    assets: &mut Assets,
    root: &Path,
    path: &Path,
) -> anyhow::Result<std::path::PathBuf> {
    let name = relative_name(root, path);
    let asset = import_gltf(renderer, assets, path, &name)?;
    let scene = prefab_from_gltf(&asset);

    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or(name);
    let output = root.join("prefabs").join(format!("{stem}.ron"));
    scene.save(&output)?;
    Ok(output)
}

/// Samma sak, men utan att spara – används när projektet öppnas och alla
/// modeller under `models/` laddas om i tysthet (assetnamnen måste
/// registreras även om ingen ny prefab behöver skrivas).
pub fn reload(
    renderer: &mut Renderer,
    assets: &mut Assets,
    root: &Path,
    path: &Path,
) -> anyhow::Result<GltfAsset> {
    let name = relative_name(root, path);
    import_gltf(renderer, assets, path, &name)
}

fn relative_name(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// Bygger en `Scene` ur nodträdet. Roten (eller rötterna, om filen har
/// flera) blir prefabens toppnivå-entiteter.
fn prefab_from_gltf(asset: &GltfAsset) -> Scene {
    let mut scene = Scene::default();
    let mut next_id = 0u32;

    fn visit(node: &GltfNode, parent: Option<u32>, scene: &mut Scene, next_id: &mut u32) {
        let id = *next_id;
        *next_id += 1;

        let mut components: BTreeMap<String, Box<ron::value::RawValue>> = BTreeMap::new();

        let transform = Transform {
            translation: node.translation,
            rotation: node.rotation,
            scale: node.scale,
        };
        insert(&mut components, "Transform", &transform);
        insert(&mut components, "Name", &EntityName::new(node.name.clone()));

        if let Some(mesh) = &node.mesh {
            let instance = MeshInstance {
                mesh: mesh.clone(),
                texture: node.texture.clone().unwrap_or_default(),
                color: node.color,
            };
            insert(&mut components, "MeshInstance", &instance);
        }

        scene.entities.push(SceneEntity {
            id,
            parent,
            components,
        });

        for child in &node.children {
            visit(child, Some(id), scene, next_id);
        }
    }

    fn insert<T: serde::Serialize>(
        components: &mut BTreeMap<String, Box<ron::value::RawValue>>,
        name: &str,
        value: &T,
    ) {
        if let Ok(raw) = ron::value::RawValue::from_rust(value) {
            components.insert(name.to_string(), raw);
        }
    }

    for root in &asset.roots {
        visit(root, None, &mut scene, &mut next_id);
    }

    scene
}
