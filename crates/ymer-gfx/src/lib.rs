//! Renderarens ordförråd, utan renderare.
//!
//! Det här är allt en backend behöver veta för att kunna rita en frame:
//! vad en vertex är, vad en mesh är, och vad som ska ritas. Ingenting
//! här nämner wgpu, och ingenting här nämner en värld.
//!
//! Gränsen fanns egentligen redan — `ymer-render` tog emot en färdig
//! [`RenderList`] och såg aldrig en `World`. Men typerna bodde i
//! wgpu-lagret, så den som ville skriva en annan backend fick dra in
//! wgpu för att få tag på dem. Nu ligger de här, och wgpu-backenden är
//! bara *en* implementation av [`Backend`].
//!
//! Det spelar roll för konsolerna. En 3DS har ingen wgpu och inga
//! fragment-shaders alls — den har en fast texturkombinator. Men den kan
//! rita en [`RenderList`], för listan beskriver *vad* som ska ritas och
//! inte hur.

use ymer_core::{Color, Mat4, MeshId, TextureId, Vec3};

// ------------------------------------------------------------- meshdata

/// En vertex, i det format varje backend får den.
///
/// `repr(C)` för att wgpu-backenden ska kunna skicka den rakt till GPU:n
/// utan att packa om den.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

/// En mesh som data, innan någon backend tagit hand om den.
#[derive(Debug, Clone, Default)]
pub struct MeshData {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

impl MeshData {
    /// Halva storleken i objektrymd. Används för plockning och culling,
    /// och räknas här så att varje backend inte behöver göra om det.
    pub fn half_extents(&self) -> Vec3 {
        let mut max = Vec3::ZERO;
        for vertex in &self.vertices {
            max = max.max(Vec3::from(vertex.position).abs());
        }
        max
    }
}

// ----------------------------------------------------------- renderlista

/// En sak att rita den här framen.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DrawItem {
    pub mesh: MeshId,
    pub texture: TextureId,
    pub transform: Mat4,
    pub color: Color,
    /// `[offset_x, offset_y, scale_x, scale_y]` – väljer ut en ruta ur ett
    /// spritesheet. `[0, 0, 1, 1]` betyder hela texturen.
    pub uv_transform: [f32; 4],
}

impl DrawItem {
    /// Ritobjekt som använder hela texturen – 3D-fallet.
    pub fn new(mesh: MeshId, texture: TextureId, transform: Mat4, color: Color) -> Self {
        Self {
            mesh,
            texture,
            transform,
            color,
            uv_transform: [0.0, 0.0, 1.0, 1.0],
        }
    }
}

/// Allt som ska ritas en frame, i den ordning det ska ritas.
#[derive(Debug, Clone)]
pub struct RenderList {
    pub view_proj: Mat4,
    pub light_dir: Vec3,
    pub clear_color: Color,
    pub items: Vec<DrawItem>,
    /// Genomskinliga objekt: ritas efter all ogenomskinlig geometri, med
    /// djuptest men utan djupskrivning, och i den ordning listan har.
    /// `build_render_list` sorterar bakifrån och fram.
    pub sprite_items: Vec<DrawItem>,
    /// Ritas sist utan djuptest – gizmos och annat som alltid ska synas.
    pub overlay_items: Vec<DrawItem>,
    /// Gränssnitt i *skärmrymd*: transformen tolkas som pixlar med origo
    /// uppe till vänster, inte som en plats i världen.
    ///
    /// Egen lista och inte `overlay_items`, för de två kan inte dela
    /// kamera. Ett gizmo ska följa med när man vrider vyn; en knapp ska
    /// ligga still.
    pub ui_items: Vec<DrawItem>,
}

impl Default for RenderList {
    fn default() -> Self {
        Self {
            view_proj: Mat4::IDENTITY,
            light_dir: Vec3::new(-0.4, -1.0, -0.35).normalize(),
            clear_color: Color::rgb(0.02, 0.02, 0.03),
            items: Vec::new(),
            sprite_items: Vec::new(),
            overlay_items: Vec::new(),
            ui_items: Vec::new(),
        }
    }
}

impl RenderList {
    /// Antal ritobjekt i alla fyra listor.
    pub fn len(&self) -> usize {
        self.items.len() + self.sprite_items.len() + self.overlay_items.len() + self.ui_items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

// ------------------------------------------------------------- ljusmodell

/// Ljusstyrkan på en yta med normalen `normal`.
///
/// Står här och inte i en shader, för varje backend måste räkna samma
/// sak och två av dem kan inte köra shaders. Formeln är avsiktligt
/// enkel: en riktad term plus en billig himmelsterm så att skuggsidor
/// inte blir helt döda.
///
/// `ymer-render` upprepar den i WGSL. Det är en dubblering, men den
/// alternativa vägen – att generera shadern ur den här funktionen – hade
/// kostat mer än den håller ihop.
pub fn shade(normal: Vec3, light_dir: Vec3) -> f32 {
    let normal = normal.normalize_or_zero();
    let to_light = (-light_dir).normalize_or_zero();
    let diffuse = normal.dot(to_light).max(0.0);
    let sky = 0.5 + 0.5 * normal.y;
    0.18 * sky + 0.82 * diffuse
}

// ---------------------------------------------------------------- backend

/// Det en renderare måste kunna.
///
/// Smal med flit. Allt som går att räkna ut ur en [`RenderList`] räknas
/// av den som bygger listan, inte av backenden — annars måste varje ny
/// plattform göra om samma arbete, och de plattformar som är svårast att
/// skriva för är också de som har minst att räkna med.
pub trait Backend {
    /// Laddar upp en mesh och ger den ett id.
    fn add_mesh(&mut self, data: &MeshData, label: &str) -> MeshId;

    /// Laddar upp en RGBA8-textur och ger den ett id.
    fn add_texture(&mut self, rgba: &[u8], width: u32, height: u32, label: &str) -> TextureId;

    /// Skriver om en textur på plats, med samma id. `false` om måtten
    /// inte stämmer eller id:t inte finns.
    ///
    /// Behövs av glyfatlasen, som växer under körning. En ny textur hade
    /// gjort id:n som redan hamnat i en halvbyggd lista ogiltiga.
    fn update_texture(&mut self, id: TextureId, rgba: &[u8], width: u32, height: u32) -> bool;

    /// Halva storleken i objektrymd för en mesh, om den finns.
    fn half_extents(&self, mesh: MeshId) -> Option<Vec3>;

    /// Ritar en frame.
    fn render(&mut self, list: &RenderList) -> anyhow::Result<()>;

    /// Målets storlek i pixlar.
    fn size(&self) -> (u32, u32);

    fn aspect_ratio(&self) -> f32 {
        let (width, height) = self.size();
        width as f32 / height.max(1) as f32
    }
}

// ------------------------------------------------------------ primitiver

pub mod primitives {
    use super::{MeshData, Vertex};

    /// Kub med hårda kanter: varje sida har egna hörn så normalerna blir platta.
    pub fn cube(size: f32) -> MeshData {
        let h = size * 0.5;
        let faces: [([f32; 3], [[f32; 3]; 4]); 6] = [
            (
                [0.0, 0.0, 1.0],
                [[-h, -h, h], [h, -h, h], [h, h, h], [-h, h, h]],
            ),
            (
                [0.0, 0.0, -1.0],
                [[h, -h, -h], [-h, -h, -h], [-h, h, -h], [h, h, -h]],
            ),
            (
                [1.0, 0.0, 0.0],
                [[h, -h, h], [h, -h, -h], [h, h, -h], [h, h, h]],
            ),
            (
                [-1.0, 0.0, 0.0],
                [[-h, -h, -h], [-h, -h, h], [-h, h, h], [-h, h, -h]],
            ),
            (
                [0.0, 1.0, 0.0],
                [[-h, h, h], [h, h, h], [h, h, -h], [-h, h, -h]],
            ),
            (
                [0.0, -1.0, 0.0],
                [[-h, -h, -h], [h, -h, -h], [h, -h, h], [-h, -h, h]],
            ),
        ];

        // Varje sida får hela UV-rutan, så en textur syns en gång per sida.
        const UVS: [[f32; 2]; 4] = [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];

        let mut data = MeshData::default();
        for (normal, corners) in faces {
            let base = data.vertices.len() as u32;
            for (index, position) in corners.into_iter().enumerate() {
                data.vertices.push(Vertex {
                    position,
                    normal,
                    uv: UVS[index],
                });
            }
            data.indices
                .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
        data
    }

    /// Kvadrat i XY-planet, normal mot +Z. Basen för alla sprites.
    pub fn quad(size: f32) -> MeshData {
        let h = size * 0.5;
        let normal = [0.0, 0.0, 1.0];
        MeshData {
            vertices: vec![
                Vertex {
                    position: [-h, -h, 0.0],
                    normal,
                    uv: [0.0, 1.0],
                },
                Vertex {
                    position: [h, -h, 0.0],
                    normal,
                    uv: [1.0, 1.0],
                },
                Vertex {
                    position: [h, h, 0.0],
                    normal,
                    uv: [1.0, 0.0],
                },
                Vertex {
                    position: [-h, h, 0.0],
                    normal,
                    uv: [0.0, 0.0],
                },
            ],
            indices: vec![0, 1, 2, 0, 2, 3],
        }
    }

    /// Plan i XZ-planet, normal uppåt. UV:erna upprepas per enhet så att
    /// en textur kaklar istället för att sträckas över hela ytan.
    pub fn plane(size: f32) -> MeshData {
        let h = size * 0.5;
        let normal = [0.0, 1.0, 0.0];
        let tiles = size;
        MeshData {
            vertices: vec![
                Vertex {
                    position: [-h, 0.0, h],
                    normal,
                    uv: [0.0, tiles],
                },
                Vertex {
                    position: [h, 0.0, h],
                    normal,
                    uv: [tiles, tiles],
                },
                Vertex {
                    position: [h, 0.0, -h],
                    normal,
                    uv: [tiles, 0.0],
                },
                Vertex {
                    position: [-h, 0.0, -h],
                    normal,
                    uv: [0.0, 0.0],
                },
            ],
            indices: vec![0, 1, 2, 0, 2, 3],
        }
    }
}
