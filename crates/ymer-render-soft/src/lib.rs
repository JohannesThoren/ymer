//! En renderare utan GPU.
//!
//! Den finns av två skäl, och det ena är inte prestanda.
//!
//! Det första: en gräns med *en* implementation bakom sig är ingen gräns,
//! bara ett löfte. `RenderList` har alltid sett portabel ut — den bär
//! matriser och id:n, inget wgpu — men det gick inte att veta förrän
//! något annat läste den. Den här crate:n är det andra ögat.
//!
//! Det andra: det här är formen en 3DS-backend har. PICA200 har
//! vertex-shaders i eget assembly och *inga* fragment-shaders alls — en
//! fast texturkombinator. Allt den kan är att rita texturerade,
//! vertexbelysta trianglar. Kan motorn uttrycka sig i det, ryms den på en
//! 3DS; kan den inte det, ryms den inte, och det vill man veta innan man
//! installerar devkitARM.
//!
//! Så rasteriseraren här är med flit fattig: inga shaders, ingen
//! blandning utöver alfa, ett djupvärde per pixel. Ungefär vad hårdvaran
//! från 2011 gör.

use ymer_core::{Color, Mat4, MeshId, TextureId, Vec2, Vec3, Vec4};
use ymer_gfx::{Backend, DrawItem, MeshData, RenderList, Vertex, shade};

mod raster;

use raster::Fragment;

/// En textur i RAM.
struct Texture {
    width: u32,
    height: u32,
    /// RGBA8, rad för rad.
    pixels: Vec<u8>,
}

impl Texture {
    /// Närmaste granne. Ingen filtrering – 3DS:en har den, men det här är
    /// inte platsen att låtsas om den.
    fn sample(&self, u: f32, v: f32) -> [f32; 4] {
        if self.width == 0 || self.height == 0 {
            return [1.0, 1.0, 1.0, 1.0];
        }
        let x = (u.rem_euclid(1.0) * self.width as f32) as u32;
        let y = (v.rem_euclid(1.0) * self.height as f32) as u32;
        let x = x.min(self.width - 1);
        let y = y.min(self.height - 1);
        let index = ((y * self.width + x) * 4) as usize;
        let texel = &self.pixels[index..index + 4];
        [
            srgb_to_linear(texel[0]),
            srgb_to_linear(texel[1]),
            srgb_to_linear(texel[2]),
            texel[3] as f32 / 255.0,
        ]
    }
}

/// Mesh plus det som räknats ur den en gång.
struct Mesh {
    data: MeshData,
    half_extents: Vec3,
}

/// Hur ett pass förhåller sig till djupbufferten.
#[derive(Clone, Copy, PartialEq)]
enum Depth {
    /// Testa och skriv: ogenomskinlig geometri.
    TestWrite,
    /// Testa men skriv inte: genomskinligt, som ritas bakifrån och fram.
    TestOnly,
    /// Strunta i djupet: gizmos och gränssnitt, som alltid ska synas.
    Ignore,
}

/// Mjukvarurenderaren.
pub struct SoftRenderer {
    width: u32,
    height: u32,
    /// Linjär RGBA. Konverteras till sRGB först när bilden lämnas ut,
    /// precis som GPU:n gör i sitt sRGB-rendermål.
    color: Vec<[f32; 4]>,
    depth: Vec<f32>,
    meshes: Vec<Mesh>,
    textures: Vec<Texture>,
}

impl SoftRenderer {
    /// En renderare för en skärm av den här storleken.
    ///
    /// Textur 0 är en vit pixel, precis som i wgpu-backenden, så att
    /// otexturerade objekt kan gå genom samma väg.
    pub fn new(width: u32, height: u32) -> Self {
        let mut renderer = Self {
            width,
            height,
            color: vec![[0.0; 4]; (width * height) as usize],
            depth: vec![f32::INFINITY; (width * height) as usize],
            meshes: Vec::new(),
            textures: Vec::new(),
        };
        renderer.add_texture(&[255, 255, 255, 255], 1, 1, "vit");
        renderer
    }

    /// 3DS:ens övre skärm.
    pub fn top_screen() -> Self {
        Self::new(400, 240)
    }

    /// 3DS:ens undre skärm.
    pub fn bottom_screen() -> Self {
        Self::new(320, 240)
    }

    /// Bilden som RGBA8, redo att skrivas till fil eller till en skärm.
    pub fn rgba8(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.color.len() * 4);
        for pixel in &self.color {
            out.push(linear_to_srgb(pixel[0]));
            out.push(linear_to_srgb(pixel[1]));
            out.push(linear_to_srgb(pixel[2]));
            out.push((pixel[3].clamp(0.0, 1.0) * 255.0) as u8);
        }
        out
    }

    fn clear(&mut self, color: Color) {
        let clear = [color.r, color.g, color.b, color.a];
        self.color.fill(clear);
        self.depth.fill(f32::INFINITY);
    }

    /// Ett pass: alla objekt i en lista, med samma djupregler.
    ///
    /// `projection` är None för gränssnittet, vars transform redan ger
    /// pixlar. Att köra det genom en ortografisk matris hade gett samma
    /// svar efter en omväg.
    fn pass(&mut self, items: &[DrawItem], projection: Option<Mat4>, light: Vec3, depth: Depth) {
        for item in items {
            let Some(mesh) = self.meshes.get(item.mesh.0 as usize) else {
                continue;
            };
            // Lånas ut som rådata: rasteriseringen skriver till self.color
            // och kan därför inte hålla kvar lånet på self.meshes.
            let vertices = mesh.data.vertices.clone();
            let indices = mesh.data.indices.clone();

            let model = item.transform;
            let mvp = match projection {
                Some(projection) => projection * model,
                None => model,
            };
            // Normalmatrisen: modellens rotation och skala, utan flytt.
            let normal_matrix = Mat3Rows::from_mat4(model);

            for triangle in indices.as_chunks::<3>().0 {
                let corners: [Vertex; 3] = [
                    vertices[triangle[0] as usize],
                    vertices[triangle[1] as usize],
                    vertices[triangle[2] as usize],
                ];
                self.triangle(&corners, mvp, normal_matrix, item, projection, light, depth);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn triangle(
        &mut self,
        corners: &[Vertex; 3],
        mvp: Mat4,
        normal_matrix: Mat3Rows,
        item: &DrawItem,
        projection: Option<Mat4>,
        light: Vec3,
        depth: Depth,
    ) {
        // Till klipprymd.
        let mut clip = [Vec4::ZERO; 3];
        let mut normals = [Vec3::ZERO; 3];
        let mut uvs = [Vec2::ZERO; 3];
        for (index, vertex) in corners.iter().enumerate() {
            let position = Vec4::new(
                vertex.position[0],
                vertex.position[1],
                vertex.position[2],
                1.0,
            );
            clip[index] = mvp * position;
            normals[index] = normal_matrix
                .mul(Vec3::from(vertex.normal))
                .normalize_or_zero();
            uvs[index] = Vec2::new(
                vertex.uv[0] * item.uv_transform[2] + item.uv_transform[0],
                vertex.uv[1] * item.uv_transform[3] + item.uv_transform[1],
            );
        }

        if projection.is_some() {
            // Klipp mot närplanet innan divisionen. Utan det hamnar
            // trianglar bakom kameran i bild, spegelvända och enorma –
            // felet ser ut som geometri som exploderar när man vrider sig.
            for part in raster::clip_near(&clip, &normals, &uvs) {
                self.raster(
                    &part.clip,
                    &part.normals,
                    &part.uvs,
                    item,
                    light,
                    depth,
                    true,
                );
            }
        } else {
            self.raster(&clip, &normals, &uvs, item, light, depth, false);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn raster(
        &mut self,
        clip: &[Vec4; 3],
        normals: &[Vec3; 3],
        uvs: &[Vec2; 3],
        item: &DrawItem,
        light: Vec3,
        depth: Depth,
        perspective: bool,
    ) {
        // Till skärmrymd. Gränssnittet är redan där.
        let mut screen = [Vec3::ZERO; 3];
        let mut inv_w = [1.0f32; 3];
        for index in 0..3 {
            if perspective {
                let w = clip[index].w;
                if w.abs() < 1e-6 {
                    return;
                }
                inv_w[index] = 1.0 / w;
                let ndc = Vec3::new(
                    clip[index].x * inv_w[index],
                    clip[index].y * inv_w[index],
                    clip[index].z * inv_w[index],
                );
                screen[index] = Vec3::new(
                    (ndc.x * 0.5 + 0.5) * self.width as f32,
                    // y pekar uppåt i klipprymd och nedåt i bild.
                    (0.5 - ndc.y * 0.5) * self.height as f32,
                    ndc.z,
                );
            } else {
                screen[index] = Vec3::new(clip[index].x, clip[index].y, 0.0);
            }
        }

        // Ingen ryggsidesbortsortering: motorns mesh-pipeline har
        // `cull_mode: None`, och två renderare som inte är överens om
        // vad som syns är värre än en långsam renderare. Bara helt platta
        // trianglar kastas.
        let area = raster::signed_area(
            screen[0].truncate(),
            screen[1].truncate(),
            screen[2].truncate(),
        );
        if area.abs() < 1e-9 {
            return;
        }

        let texture = self.textures.get(item.texture.0 as usize);
        let base = [item.color.r, item.color.g, item.color.b, item.color.a];

        let bounds = raster::bounds(&screen, self.width, self.height);
        for y in bounds.min_y..bounds.max_y {
            for x in bounds.min_x..bounds.max_x {
                let point = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                let Some(bary) = raster::barycentric(&screen, point, area) else {
                    continue;
                };

                let fragment =
                    Fragment::interpolate(bary, &screen, &inv_w, normals, uvs, perspective);
                if depth != Depth::Ignore {
                    // Utanför djupspannet: bakom närplanet eller bortom
                    // bortre planet.
                    if perspective && !(0.0..=1.0).contains(&fragment.depth) {
                        continue;
                    }
                    let slot = (y * self.width + x) as usize;
                    if fragment.depth >= self.depth[slot] {
                        continue;
                    }
                    if depth == Depth::TestWrite {
                        self.depth[slot] = fragment.depth;
                    }
                }

                let texel = match texture {
                    Some(texture) => texture.sample(fragment.uv.x, fragment.uv.y),
                    None => [1.0; 4],
                };
                let mut rgba = [
                    base[0] * texel[0],
                    base[1] * texel[1],
                    base[2] * texel[2],
                    base[3] * texel[3],
                ];
                // Gränssnittet ljussätts inte; en knapp har ingen normal
                // att vända mot solen.
                if perspective {
                    let light = shade(fragment.normal, light);
                    rgba[0] *= light;
                    rgba[1] *= light;
                    rgba[2] *= light;
                }

                // Direkt mot fältet, inte genom en metod på self:
                // texturen ovan lånar self.textures, och ett &mut self
                // här hade krockat med det. Disjunkta fält går bra.
                let slot = (y * self.width + x) as usize;
                blend(&mut self.color[slot], rgba);
            }
        }
    }
}

/// Alfablandning mot en pixel.
fn blend(destination: &mut [f32; 4], source: [f32; 4]) {
    let alpha = source[3].clamp(0.0, 1.0);
    if alpha <= 0.0 {
        return;
    }
    *destination = [
        destination[0] + (source[0] - destination[0]) * alpha,
        destination[1] + (source[1] - destination[1]) * alpha,
        destination[2] + (source[2] - destination[2]) * alpha,
        1.0,
    ];
}

impl Backend for SoftRenderer {
    fn add_mesh(&mut self, data: &MeshData, _label: &str) -> MeshId {
        self.meshes.push(Mesh {
            half_extents: data.half_extents(),
            data: data.clone(),
        });
        MeshId(self.meshes.len() as u32 - 1)
    }

    fn add_texture(&mut self, rgba: &[u8], width: u32, height: u32, _label: &str) -> TextureId {
        self.textures.push(Texture {
            width,
            height,
            pixels: rgba.to_vec(),
        });
        TextureId(self.textures.len() as u32 - 1)
    }

    fn update_texture(&mut self, id: TextureId, rgba: &[u8], width: u32, height: u32) -> bool {
        let Some(texture) = self.textures.get_mut(id.0 as usize) else {
            return false;
        };
        if texture.width != width || texture.height != height {
            return false;
        }
        texture.pixels.clear();
        texture.pixels.extend_from_slice(rgba);
        true
    }

    fn half_extents(&self, mesh: MeshId) -> Option<Vec3> {
        self.meshes
            .get(mesh.0 as usize)
            .map(|mesh| mesh.half_extents)
    }

    fn render(&mut self, list: &RenderList) -> anyhow::Result<()> {
        self.clear(list.clear_color);

        let view_proj = Some(list.view_proj);
        self.pass(&list.items, view_proj, list.light_dir, Depth::TestWrite);
        self.pass(
            &list.sprite_items,
            view_proj,
            list.light_dir,
            Depth::TestOnly,
        );
        self.pass(
            &list.overlay_items,
            view_proj,
            list.light_dir,
            Depth::Ignore,
        );
        // Gränssnittet sist, i pixlar, utan kamera och utan ljus.
        self.pass(&list.ui_items, None, list.light_dir, Depth::Ignore);

        Ok(())
    }

    fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

/// Modellmatrisens rotation och skala, utan flytten.
#[derive(Clone, Copy)]
struct Mat3Rows([Vec3; 3]);

impl Mat3Rows {
    fn from_mat4(matrix: Mat4) -> Self {
        let columns = matrix.to_cols_array_2d();
        Self([
            Vec3::new(columns[0][0], columns[0][1], columns[0][2]),
            Vec3::new(columns[1][0], columns[1][1], columns[1][2]),
            Vec3::new(columns[2][0], columns[2][1], columns[2][2]),
        ])
    }

    fn mul(self, vector: Vec3) -> Vec3 {
        self.0[0] * vector.x + self.0[1] * vector.y + self.0[2] * vector.z
    }
}

fn srgb_to_linear(value: u8) -> f32 {
    let value = value as f32 / 255.0;
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(value: f32) -> u8 {
    let value = value.clamp(0.0, 1.0);
    let encoded = if value <= 0.0031308 {
        value * 12.92
    } else {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    };
    (encoded * 255.0 + 0.5) as u8
}
