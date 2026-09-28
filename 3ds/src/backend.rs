//! Motorns renderare på PICA200.
//!
//! Den tredje implementationen av [`Backend`], efter wgpu och
//! mjukvarurasteriseraren. Att den gick att skriva utan att röra
//! `RenderList` är hela poängen med sömmen: listan säger *vad* som ska
//! ritas, och hårdvaran från 2011 får göra det på sitt sätt.
//!
//! # Vad som skiljer mot skrivbordet
//!
//! **Ljuset räknas per vertex.** PICA200 har ingen fragment-shader, bara
//! en kombinator med sex steg. Formeln bor därför i `ymer.v.pica` och
//! resultatet kommer ut som vertexfärg. Gouraud i stället för Phong; på
//! hårda kanter identiskt, på mjuka meshar syns brytningen.
//!
//! **Texturer måste vara tvåpotenser** mellan 8 och 1024, och lagras i
//! 8x8-brickor med Morton-ordning. `ymer-pica` sköter omläggningen och är
//! testad på skrivbordet.
//!
//! **Index är 16 bitar.** En mesh med fler än 65536 hörn avvisas när den
//! laddas, inte när den ritas.
//!
//! **Skärmen är roterad.** Framebufferten är 240x400, inte 400x240.
//! `citro3d::math::Projection` känner till det genom `AspectRatio` och
//! `ScreenOrientation`.

use std::collections::HashMap;

use citro3d::attrib;
use citro3d::buffer;
use citro3d::macros::include_shader;
use citro3d::math::Matrix4;
use citro3d::render::Target;
use citro3d::texenv;
use citro3d::texture;
use citro3d::{Instance, shader};
use ctru::linear::LinearAllocator;

use ymer_core::{Mat4, MeshId, TextureId, Vec3};
use ymer_gfx::{Backend, DrawItem, MeshData, RenderList};

static SHADER: &[u8] = include_shader!("../assets/ymer.v.pica");

/// Vertexen som den ligger i GPU-minnet.
///
/// Samma fält som `ymer_gfx::Vertex` och samma ordning, men egen typ:
/// den här måste vara `repr(C)` *och* ligga i linjärt minne, och att
/// låtsas att de två är samma typ hade dolt att det är ett byte.
#[repr(C)]
#[derive(Clone, Copy)]
struct PicaVertex {
    position: [f32; 3],
    normal: [f32; 3],
    uv: [f32; 2],
}

struct Mesh {
    /// Hålls vid liv: buffertinfon pekar in i den här allokeringen.
    _vertices: Vec<PicaVertex, LinearAllocator>,
    info: buffer::Info,
    indices: Vec<u16, LinearAllocator>,
    half_extents: Vec3,
}

struct Uniforms {
    mvp: citro3d::uniform::Index,
    model: citro3d::uniform::Index,
    /// Rad 0 = riktning mot ljuset, rad 1 = materialfärg.
    params: citro3d::uniform::Index,
}

/// Bygger en citro3d-matris ur rader.
///
/// **Den här funktionen är den enda raden i filen jag inte kunnat pröva
/// mot ett riktigt citro3d.** `Matrix4::from_rows` står inte i den
/// dokumentation jag hittade; finns den inte under det namnet är det
/// här den ska rättas, och bara här. Allt annat går genom den.
fn matrix(rows: [[f32; 4]; 4]) -> Matrix4 {
    Matrix4::from_rows(rows)
}

/// Allt utom GPU-instansen.
///
/// Skilt från `Instance` med flit. `render_frame_with` lånar instansen
/// under hela framen, så en `draw` som satt på samma struct hade inte
/// gått att anropa inifrån slingan. Med två fält lånas de var för sig.
pub struct Resources {
    program: shader::Program,
    uniforms: Uniforms,
    attr_info: attrib::Info,
    meshes: Vec<Mesh>,
    textures: HashMap<u32, texture::Texture>,
    next_texture: u32,
}

/// Renderaren.
pub struct Citro3dBackend {
    instance: Instance,
    resources: Resources,
    width: u32,
    height: u32,
}

impl Citro3dBackend {
    /// Startar GPU:n och laddar shadern.
    pub fn new(width: u32, height: u32) -> Result<Self, Box<dyn std::error::Error>> {
        let instance = Instance::new()?;

        let library = shader::Library::from_bytes(SHADER)?;
        let program = shader::Program::new(library.get(0)?)?;

        let uniforms = Uniforms {
            mvp: program.get_uniform("mvp")?,
            model: program.get_uniform("model")?,
            params: program.get_uniform("params")?,
        };

        // Layouten måste stämma med `.alias` i shadern: v0 position,
        // v1 normal, v2 uv. Går de isär ritas geometrin med normalerna
        // som koordinater, vilket ser ut som att meshen exploderat.
        let mut attr_info = attrib::Info::new();
        attr_info.add_loader(attrib::Register::V0, attrib::Format::Float, 3)?;
        attr_info.add_loader(attrib::Register::V1, attrib::Format::Float, 3)?;
        attr_info.add_loader(attrib::Register::V2, attrib::Format::Float, 2)?;

        Ok(Self {
            instance,
            resources: Resources {
                program,
                uniforms,
                attr_info,
                meshes: Vec::new(),
                textures: HashMap::new(),
                next_texture: 0,
            },
            width,
            height,
        })
    }

    pub fn instance_mut(&mut self) -> &mut Instance {
        &mut self.instance
    }

    /// Instansen och resurserna var för sig.
    ///
    /// Så här ritar man en frame:
    ///
    /// ```ignore
    /// let (instance, scene) = renderer.split();
    /// instance.render_frame_with(|mut frame| {
    ///     scene.draw(&mut frame, &list, Screen::Top);
    ///     frame
    /// });
    /// ```
    pub fn split(&mut self) -> (&mut Instance, &Resources) {
        (&mut self.instance, &self.resources)
    }
}

impl Resources {
    /// Ritar en lista.
    ///
    /// Skilt från [`Backend::render`], som inte får något mål: på en 3DS
    /// äger anroparen skärmarna, och samma lista ritas ofta till både
    /// övre och undre skärmen.
    pub fn draw(&self, frame: &mut citro3d::render::Frame, list: &RenderList, screen: Screen) {
        frame.bind_program(&self.program);
        frame.set_attr_info(&self.attr_info);

        // Kombinatorn ersätter fragment-shadern: textur gånger den färg
        // vertexshadern räknade fram. Otexturerade objekt binder den vita
        // texturen, så samma uppställning duger åt båda.
        let stage = texenv::TexEnv::new()
            .src(
                texenv::Mode::BOTH,
                texenv::Source::Texture0,
                Some(texenv::Source::PrimaryColor),
                None,
            )
            .func(texenv::Mode::BOTH, texenv::CombineFunc::Modulate);
        frame.set_texenvs(&[stage]);

        // Riktningen *mot* ljuset, normaliserad här så att shadern
        // slipper göra det en gång per vertex.
        let light = (-list.light_dir).normalize_or_zero();

        let projection = screen.projection(list.view_proj);

        for item in list.items.iter().chain(&list.sprite_items) {
            self.draw_item(frame, item, projection, light);
        }
        for item in &list.overlay_items {
            self.draw_item(frame, item, projection, light);
        }

        // Gränssnittet i skärmrymd: transformen är redan pixlar, så bara
        // den ortografiska matrisen läggs på.
        let ortho = screen.ortho();
        for item in &list.ui_items {
            self.draw_item(frame, item, ortho, light);
        }
    }

    fn draw_item(
        &self,
        frame: &mut citro3d::render::Frame,
        item: &DrawItem,
        camera: Mat4,
        light: Vec3,
    ) {
        let Some(mesh) = self.meshes.get(item.mesh.0 as usize) else {
            return;
        };

        let mvp = ymer_pica::rows(camera * item.transform);
        let model = ymer_pica::normal_rows(item.transform);

        frame.bind_vertex_uniform(self.uniforms.mvp, &matrix(mvp));
        frame.bind_vertex_uniform(
            self.uniforms.model,
            &matrix([model[0], model[1], model[2], [0.0, 0.0, 0.0, 1.0]]),
        );
        frame.bind_vertex_uniform(
            self.uniforms.params,
            &matrix([
                [light.x, light.y, light.z, 0.0],
                [item.color.r, item.color.g, item.color.b, item.color.a],
                [0.0; 4],
                [0.0; 4],
            ]),
        );

        if let Some(texture) = self.textures.get(&item.texture.0) {
            frame.bind_texture(texture::Index::Texture0, texture);
        }

        let _ = frame.draw_elements(buffer::Primitive::Triangles, &mesh.info, &mesh.indices);
    }
}

/// Vilken skärm en lista ritas till.
///
/// Måtten skiljer, och framför allt: framebuffertarna är roterade ett
/// kvarts varv. Att räkna med 400x240 rakt av ger en bild på högkant.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Top,
    Bottom,
}

impl Screen {
    pub fn size(self) -> (u32, u32) {
        match self {
            Self::Top => (400, 240),
            Self::Bottom => (320, 240),
        }
    }

    fn aspect(self) -> citro3d::math::AspectRatio {
        match self {
            Self::Top => citro3d::math::AspectRatio::TopScreen,
            Self::Bottom => citro3d::math::AspectRatio::BottomScreen,
        }
    }

    /// Scenens kamera, med skärmens rotation pålagd.
    ///
    /// `RenderList::view_proj` är byggd för en vanlig skärm. Rotationen
    /// läggs på här i stället för att smyga in i varje spel.
    fn projection(self, view_proj: Mat4) -> Mat4 {
        let rotation = Mat4::from_rotation_z(core::f32::consts::FRAC_PI_2);
        let _ = self.aspect();
        rotation * view_proj
    }

    /// Gränssnittets matris: pixlar in, klippkoordinater ut, roterad.
    fn ortho(self) -> Mat4 {
        let (width, height) = self.size();
        // Samma handbyggda matris som motorns skärmrymdspass: x 0..bredd
        // till -1..1, y 0..höjd till 1..-1. Sedan skärmens kvartsvarv.
        let pixels = Mat4::from_cols_array_2d(&[
            [2.0 / width as f32, 0.0, 0.0, 0.0],
            [0.0, -2.0 / height as f32, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [-1.0, 1.0, 0.0, 1.0],
        ]);
        Mat4::from_rotation_z(core::f32::consts::FRAC_PI_2) * pixels
    }
}

impl Backend for Citro3dBackend {
    fn add_mesh(&mut self, data: &MeshData, label: &str) -> MeshId {
        let resources = &mut self.resources;
        let indices = match ymer_pica::narrow_indices(&data.indices) {
            Ok(indices) => indices,
            Err(worst) => {
                // Hellre en tom mesh med ett tydligt meddelande än
                // geometri som tyst faller bort.
                eprintln!(
                    "mesh \"{label}\" har index {worst}, men PICA200 klarar bara {}",
                    ymer_pica::MAX_VERTICES - 1
                );
                Vec::new()
            }
        };

        let vertices: Vec<PicaVertex, LinearAllocator> = {
            let mut out = Vec::with_capacity_in(data.vertices.len(), LinearAllocator);
            out.extend(data.vertices.iter().map(|vertex| PicaVertex {
                position: vertex.position,
                normal: vertex.normal,
                uv: vertex.uv,
            }));
            out
        };

        let mut linear_indices = Vec::with_capacity_in(indices.len(), LinearAllocator);
        linear_indices.extend_from_slice(&indices);

        let vbo = buffer::Buffer::new(&vertices);
        let mut info = buffer::Info::new();
        let _ = info.add(vbo, resources.attr_info.permutation());

        resources.meshes.push(Mesh {
            // Vertexdatan måste överleva så länge buffertinfon gör det.
            _vertices: vertices,
            info,
            indices: linear_indices,
            half_extents: data.half_extents(),
        });
        MeshId(resources.meshes.len() as u32 - 1)
    }

    fn add_texture(&mut self, rgba: &[u8], width: u32, height: u32, label: &str) -> TextureId {
        let id = self.resources.next_texture;
        self.resources.next_texture += 1;

        let tiled = match ymer_pica::tile_rgba8(rgba, width, height) {
            Ok(tiled) => tiled,
            Err(err) => {
                eprintln!("texturen \"{label}\" duger inte åt PICA200: {err}");
                return TextureId(id);
            }
        };

        let params =
            texture::TextureParameters::new_2d(width as u16, height as u16, texture::ColorFormat::Rgba8);
        let Ok(mut texture) = texture::Texture::new(params) else {
            eprintln!("kunde inte skapa texturen \"{label}\"");
            return TextureId(id);
        };
        let _ = texture.load_image(&tiled, texture::Face::default());
        // Närmaste granne: samma som mjukvarurenderaren, så att de två
        // går att jämföra. Linjär filtrering finns och är nästa steg.
        texture.set_filter(texture::Filter::Nearest, texture::Filter::Nearest);
        texture.set_wrap(texture::Wrap::Repeat, texture::Wrap::Repeat);

        self.resources.textures.insert(id, texture);
        TextureId(id)
    }

    fn update_texture(&mut self, id: TextureId, rgba: &[u8], width: u32, height: u32) -> bool {
        let Ok(tiled) = ymer_pica::tile_rgba8(rgba, width, height) else {
            return false;
        };
        let Some(texture) = self.resources.textures.get_mut(&id.0) else {
            return false;
        };
        texture.load_image(&tiled, texture::Face::default()).is_ok()
    }

    fn half_extents(&self, mesh: MeshId) -> Option<Vec3> {
        self.resources
            .meshes
            .get(mesh.0 as usize)
            .map(|mesh| mesh.half_extents)
    }

    fn render(&mut self, _list: &RenderList) -> anyhow::Result<()> {
        // Går inte att uppfylla här: en frame på 3DS börjar och slutar
        // hos den som äger skärmarna, och en lista ritas ofta till två
        // skärmar. Använd `draw` inifrån `render_frame_with` i stället.
        anyhow::bail!("använd Citro3dBackend::draw inuti render_frame_with")
    }

    fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}
