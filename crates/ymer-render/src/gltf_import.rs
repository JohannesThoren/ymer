//! glTF-import.
//!
//! Laddar meshar och texturer, och returnerar nodhierarkin som ren data –
//! namn, transform, mesh- och texturnamn, barn. Ingen kunskap om
//! `ymer_scene::Scene` här; det håller den här filen oberoende av
//! scenformatet och gör att renderaren fortfarande bara känner till
//! `ymer-core`. Den som anropar (editorn) bygger en prefab av resultatet.

use std::collections::BTreeMap;
use std::path::Path;

use ymer_core::{Color, Quat, Vec3};

use crate::{Assets, MeshData, Renderer, Vertex};

/// En nod i glTF:ens scenträd, redan kopplad till uppladdade assets.
pub struct GltfNode {
    pub name: String,
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
    /// Assetnamn, redan registrerat i `Assets` – redo att sättas på en
    /// `MeshInstance`.
    pub mesh: Option<String>,
    pub texture: Option<String>,
    /// Materialets `baseColorFactor` – tonar texturen, eller är själva
    /// färgen om meshen saknar textur.
    pub color: Color,
    pub children: Vec<GltfNode>,
}

pub struct GltfAsset {
    pub mesh_names: Vec<String>,
    pub texture_names: Vec<String>,
    pub roots: Vec<GltfNode>,
}

/// Läser en `.gltf`/`.glb`-fil, laddar upp geometri och material, och
/// returnerar nodträdet. `name` är assetnamnet filen får, normalt dess
/// sökväg relativt projektroten – meshar och texturer namnges
/// `"{name}#den-här-meshen"`.
pub fn import_gltf(
    renderer: &mut Renderer,
    assets: &mut Assets,
    path: &Path,
    name: &str,
) -> anyhow::Result<GltfAsset> {
    let import = gltf::import(path)
        .map_err(|err| anyhow::anyhow!("kunde inte läsa {}: {err}", path.display()))?;
    import_parsed(renderer, assets, import, name)
}

/// Samma sak, men från bytes i minnet – vägen när modellen ligger i ett
/// `.pak`-arkiv. Fungerar för `.glb` (allt inbäddat). En `.gltf` som
/// refererar externa filer kan inte lösas upp här, eftersom det inte
/// finns någon katalog att leta i.
pub fn import_gltf_bytes(
    renderer: &mut Renderer,
    assets: &mut Assets,
    bytes: &[u8],
    name: &str,
) -> anyhow::Result<GltfAsset> {
    let import = gltf::import_slice(bytes)
        .map_err(|err| anyhow::anyhow!("kunde inte tolka {name}: {err}"))?;
    import_parsed(renderer, assets, import, name)
}

fn import_parsed(
    renderer: &mut Renderer,
    assets: &mut Assets,
    import: (
        gltf::Document,
        Vec<gltf::buffer::Data>,
        Vec<gltf::image::Data>,
    ),
    name: &str,
) -> anyhow::Result<GltfAsset> {
    let (document, buffers, images) = import;

    // --- geometri och material, en gång per mesh --------------------------
    let mut mesh_names = Vec::new();
    let mut texture_names = Vec::new();
    let mut mesh_asset_by_index: BTreeMap<usize, String> = BTreeMap::new();
    let mut texture_asset_by_index: BTreeMap<usize, Option<String>> = BTreeMap::new();
    let mut color_by_index: BTreeMap<usize, Color> = BTreeMap::new();
    // En bild kan återanvändas av flera meshar – ladda upp den en gång.
    let mut uploaded_images: BTreeMap<usize, String> = BTreeMap::new();

    for mesh in document.meshes() {
        let mut data = MeshData::default();
        let mut texture_name: Option<String> = None;
        let mut base_color = Color::WHITE;

        for primitive in mesh.primitives() {
            let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

            let positions: Vec<[f32; 3]> = reader
                .read_positions()
                .map(Iterator::collect)
                .unwrap_or_default();
            let normals: Vec<[f32; 3]> = reader
                .read_normals()
                .map(Iterator::collect)
                .unwrap_or_else(|| vec![[0.0, 1.0, 0.0]; positions.len()]);
            let uvs: Vec<[f32; 2]> = reader
                .read_tex_coords(0)
                .map(|iter| iter.into_f32().collect())
                .unwrap_or_else(|| vec![[0.0, 0.0]; positions.len()]);

            let base_index = data.vertices.len() as u32;
            for (index, position) in positions.iter().enumerate() {
                data.vertices.push(Vertex {
                    position: *position,
                    normal: normals.get(index).copied().unwrap_or([0.0, 1.0, 0.0]),
                    uv: uvs.get(index).copied().unwrap_or([0.0, 0.0]),
                });
            }

            let indices: Vec<u32> = match reader.read_indices() {
                Some(read) => read.into_u32().map(|index| index + base_index).collect(),
                None => (base_index..base_index + positions.len() as u32).collect(),
            };
            data.indices.extend(indices);

            // Bara första primitivens material – flera material per mesh
            // (multi-material-meshar) stöds inte, eftersom `MeshInstance`
            // har exakt en textur.
            if texture_name.is_none() {
                let pbr = primitive.material().pbr_metallic_roughness();
                let factor = pbr.base_color_factor();
                base_color = Color::rgba(factor[0], factor[1], factor[2], factor[3]);

                if let Some(info) = pbr.base_color_texture() {
                    let image_index = info.texture().source().index();
                    let asset_name = uploaded_images.entry(image_index).or_insert_with(|| {
                        let image = &images[image_index];
                        let rgba = to_rgba8(image);
                        let asset_name = format!("{name}#tex{image_index}");
                        let id =
                            renderer.add_texture(&rgba, image.width, image.height, &asset_name);
                        assets.register_texture(asset_name.clone(), id);
                        asset_name
                    });
                    texture_name = Some(asset_name.clone());
                }
            }
        }

        if data.vertices.is_empty() {
            continue;
        }

        let mesh_index = mesh.index();
        let label = mesh.name().filter(|n| !n.is_empty()).map(str::to_string);
        let asset_name = match &label {
            Some(label) => format!("{name}#{label}"),
            None => format!("{name}#{mesh_index}"),
        };

        let id = renderer.add_mesh(&data);
        assets.register_mesh(asset_name.clone(), id);
        mesh_names.push(asset_name.clone());
        if let Some(texture) = &texture_name {
            texture_names.push(texture.clone());
        }
        mesh_asset_by_index.insert(mesh_index, asset_name);
        texture_asset_by_index.insert(mesh_index, texture_name);
        color_by_index.insert(mesh_index, base_color);
    }

    // --- nodhierarki --------------------------------------------------
    fn visit(
        node: &gltf::Node,
        mesh_asset_by_index: &BTreeMap<usize, String>,
        texture_asset_by_index: &BTreeMap<usize, Option<String>>,
        color_by_index: &BTreeMap<usize, Color>,
    ) -> GltfNode {
        let (translation, rotation, scale) = node.transform().decomposed();

        let (mesh, texture, color) = match node.mesh() {
            Some(mesh) => (
                mesh_asset_by_index.get(&mesh.index()).cloned(),
                texture_asset_by_index.get(&mesh.index()).cloned().flatten(),
                color_by_index
                    .get(&mesh.index())
                    .copied()
                    .unwrap_or(Color::WHITE),
            ),
            None => (None, None, Color::WHITE),
        };

        GltfNode {
            name: node.name().unwrap_or("Node").to_string(),
            translation: Vec3::from(translation),
            rotation: Quat::from_array(rotation),
            scale: Vec3::from(scale),
            mesh,
            texture,
            color,
            children: node
                .children()
                .map(|child| {
                    visit(
                        &child,
                        mesh_asset_by_index,
                        texture_asset_by_index,
                        color_by_index,
                    )
                })
                .collect(),
        }
    }

    let scene = document
        .default_scene()
        .or_else(|| document.scenes().next());
    let roots = scene
        .map(|scene| {
            scene
                .nodes()
                .map(|node| {
                    visit(
                        &node,
                        &mesh_asset_by_index,
                        &texture_asset_by_index,
                        &color_by_index,
                    )
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(GltfAsset {
        mesh_names,
        texture_names,
        roots,
    })
}

/// Konverterar glTF:ens avkodade pixlar till RGBA8, oavsett ursprungsformat.
fn to_rgba8(image: &gltf::image::Data) -> Vec<u8> {
    use gltf::image::Format;

    let pixel_count = (image.width * image.height) as usize;
    match image.format {
        Format::R8G8B8A8 => image.pixels.clone(),
        Format::R8G8B8 => {
            let mut out = Vec::with_capacity(pixel_count * 4);
            for chunk in image.pixels.as_chunks::<3>().0 {
                out.extend_from_slice(chunk);
                out.push(255);
            }
            out
        }
        Format::R8 => {
            let mut out = Vec::with_capacity(pixel_count * 4);
            for &value in &image.pixels {
                out.extend_from_slice(&[value, value, value, 255]);
            }
            out
        }
        Format::R8G8 => {
            let mut out = Vec::with_capacity(pixel_count * 4);
            for chunk in image.pixels.as_chunks::<2>().0 {
                out.extend_from_slice(&[chunk[0], chunk[0], chunk[0], chunk[1]]);
            }
            out
        }
        // 16-bitars format är ovanliga för base-color och inte värda
        // komplexiteten just nu – degraderar till vitt hellre än att krascha.
        _ => vec![255; pixel_count * 4],
    }
}
