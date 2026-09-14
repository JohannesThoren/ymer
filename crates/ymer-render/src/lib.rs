//! GPU-lagret. Äger device, queue, rendermål och pipelines.
//! Vet inget om ECS – det matas med en färdig `RenderList`.

use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;
use winit::window::Window;
use ymer_core::{Color, Mat4, MeshId, TextureId, Vec3};

pub mod assets;
pub mod egui_overlay;
pub mod gltf_import;
pub use assets::Assets;
pub use egui_overlay::EguiOverlay;
pub use gltf_import::{GltfAsset, GltfNode, import_gltf};
pub use wgpu;

pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
pub const OFFSCREEN_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

// ------------------------------------------------------------ mesh-data

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

impl Vertex {
    const ATTRIBUTES: [wgpu::VertexAttribute; 3] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 7 => Float32x2];

    fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: size_of::<Vertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

/// Mesh i RAM, innan den laddats upp.
#[derive(Debug, Clone, Default)]
pub struct MeshData {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

pub struct GpuMesh {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
    /// Halva storleken i objektrymd. Används för plockning och senare culling.
    half_extents: Vec3,
}

/// Enkelt register: `MeshId` är index i vektorn. Byts mot riktiga
/// asset-handtag med hot reload i milstolpe 5.
#[derive(Default)]
pub struct MeshRegistry {
    meshes: Vec<GpuMesh>,
}

impl MeshRegistry {
    pub fn add(&mut self, device: &wgpu::Device, data: &MeshData) -> MeshId {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("mesh vertices"),
            contents: bytemuck::cast_slice(&data.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("mesh indices"),
            contents: bytemuck::cast_slice(&data.indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let mut max = Vec3::ZERO;
        for vertex in &data.vertices {
            max = max.max(Vec3::from(vertex.position).abs());
        }

        self.meshes.push(GpuMesh {
            vertex_buffer,
            index_buffer,
            index_count: data.indices.len() as u32,
            half_extents: max,
        });
        MeshId(self.meshes.len() as u32 - 1)
    }

    fn get(&self, id: MeshId) -> Option<&GpuMesh> {
        self.meshes.get(id.0 as usize)
    }

    /// Halva storleken i objektrymd, eller None om handtaget är okänt.
    pub fn half_extents(&self, id: MeshId) -> Option<Vec3> {
        self.meshes.get(id.0 as usize).map(|mesh| mesh.half_extents)
    }
}

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

/// Texturer plus deras bindgrupper. Id 0 är en vit 1x1-pixel, så att
/// otexturerade objekt kan gå genom exakt samma pipeline.
#[derive(Default)]
pub struct TextureRegistry {
    bind_groups: Vec<wgpu::BindGroup>,
}

impl TextureRegistry {
    fn get(&self, id: TextureId) -> Option<&wgpu::BindGroup> {
        self.bind_groups.get(id.0 as usize)
    }

    pub fn len(&self) -> usize {
        self.bind_groups.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bind_groups.is_empty()
    }
}

// ----------------------------------------------------------- renderlista

/// En sak att rita den här framen.
#[derive(Debug, Clone, Copy)]
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

/// Allt renderaren behöver för en frame. Byggs av runtime ur ECS-världen.
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
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct CameraUniform {
    view_proj: [[f32; 4]; 4],
    light_dir: [f32; 4],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct InstanceRaw {
    model: [[f32; 4]; 4],
    color: [f32; 4],
    uv_transform: [f32; 4],
}

impl InstanceRaw {
    const ATTRIBUTES: [wgpu::VertexAttribute; 6] = wgpu::vertex_attr_array![
        2 => Float32x4, 3 => Float32x4, 4 => Float32x4, 5 => Float32x4, 6 => Float32x4,
        8 => Float32x4
    ];

    fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: size_of::<InstanceRaw>() as u64,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

// -------------------------------------------------------------- renderer

enum Target {
    Window {
        surface: wgpu::Surface<'static>,
        config: wgpu::SurfaceConfiguration,
    },
    Offscreen {
        texture: wgpu::Texture,
        view: wgpu::TextureView,
        width: u32,
        height: u32,
    },
}

pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    target: Target,
    depth_view: wgpu::TextureView,
    pipeline: wgpu::RenderPipeline,
    sprite_pipeline: wgpu::RenderPipeline,
    overlay_pipeline: wgpu::RenderPipeline,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    instance_buffer: wgpu::Buffer,
    instance_capacity: usize,
    material_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    pub meshes: MeshRegistry,
    pub textures: TextureRegistry,
    stats: FrameStats,
}

impl Renderer {
    /// Renderar till ett fönster.
    pub async fn new_windowed(window: Arc<Window>) -> anyhow::Result<Self> {
        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);

        let instance = create_instance();
        let surface = instance.create_surface(window)?;
        let (adapter, device, queue) = request_device(&instance, Some(&surface)).await?;

        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            color_space: wgpu::SurfaceColorSpace::Auto,
            width,
            height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        Ok(Self::assemble(
            device,
            queue,
            Target::Window { surface, config },
            format,
            width,
            height,
        ))
    }

    /// Renderar till en textur i minnet – för CI, tester och skärmdumpar.
    pub async fn new_offscreen(width: u32, height: u32) -> anyhow::Result<Self> {
        let instance = create_instance();
        let (_adapter, device, queue) = request_device(&instance, None).await?;

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("offscreen target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: OFFSCREEN_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Ok(Self::assemble(
            device,
            queue,
            Target::Offscreen {
                texture,
                view,
                width,
                height,
            },
            OFFSCREEN_FORMAT,
            width,
            height,
        ))
    }

    fn assemble(
        device: wgpu::Device,
        queue: wgpu::Queue,
        target: Target,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
    ) -> Self {
        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("camera uniform"),
            size: size_of::<CameraUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("camera layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera bind group"),
            layout: &camera_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let material_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("material layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("material sampler"),
            // Repeat så att plan kan kakla sin textur.
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });

        let pipeline = build_mesh_pipeline(
            &device,
            format,
            &camera_layout,
            &material_layout,
            DepthMode::Opaque,
        );
        let sprite_pipeline = build_mesh_pipeline(
            &device,
            format,
            &camera_layout,
            &material_layout,
            DepthMode::Transparent,
        );
        let overlay_pipeline = build_mesh_pipeline(
            &device,
            format,
            &camera_layout,
            &material_layout,
            DepthMode::Overlay,
        );
        let depth_view = create_depth_view(&device, width, height);

        let instance_capacity = 256;
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("instances"),
            size: (instance_capacity * size_of::<InstanceRaw>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut renderer = Self {
            device,
            queue,
            target,
            depth_view,
            pipeline,
            sprite_pipeline,
            overlay_pipeline,
            camera_buffer,
            camera_bind_group,
            instance_buffer,
            instance_capacity,
            material_layout,
            sampler,
            meshes: MeshRegistry::default(),
            textures: TextureRegistry::default(),
            stats: FrameStats::default(),
        };

        // TextureId(0): vit pixel för allt otexturerat.
        renderer.add_texture(&[255, 255, 255, 255], 1, 1, "white");
        renderer
    }

    /// Laddar upp rå RGBA och returnerar handtaget.
    pub fn add_texture(&mut self, rgba: &[u8], width: u32, height: u32, label: &str) -> TextureId {
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(label),
            layout: &self.material_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        self.textures.bind_groups.push(bind_group);
        TextureId(self.textures.bind_groups.len() as u32 - 1)
    }

    /// Avkodar en PNG och laddar upp den.
    pub fn add_texture_from_png(&mut self, bytes: &[u8], label: &str) -> anyhow::Result<TextureId> {
        let decoded = image::load_from_memory(bytes)?.to_rgba8();
        let (width, height) = decoded.dimensions();
        Ok(self.add_texture(&decoded, width, height, label))
    }

    pub fn stats(&self) -> FrameStats {
        self.stats
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    pub fn size(&self) -> (u32, u32) {
        match &self.target {
            Target::Window { config, .. } => (config.width, config.height),
            Target::Offscreen { width, height, .. } => (*width, *height),
        }
    }

    /// Färgformatet som render-passet skriver till – overlays måste matcha det.
    pub fn output_format(&self) -> wgpu::TextureFormat {
        match &self.target {
            Target::Window { config, .. } => config.format,
            Target::Offscreen { .. } => OFFSCREEN_FORMAT,
        }
    }

    pub fn aspect_ratio(&self) -> f32 {
        let (w, h) = self.size();
        w as f32 / h.max(1) as f32
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        if let Target::Window { surface, config } = &mut self.target {
            config.width = width;
            config.height = height;
            surface.configure(&self.device, config);
        }
        self.depth_view = create_depth_view(&self.device, width, height);
    }

    /// Laddar upp en mesh och returnerar dess handtag.
    pub fn add_mesh(&mut self, data: &MeshData) -> MeshId {
        self.meshes.add(&self.device, data)
    }

    /// Ritar en frame. Övergående surface-tillstånd hanteras internt;
    /// bara en förlorad surface bubblar upp.
    pub fn render(&mut self, list: &RenderList) -> Result<(), RenderError> {
        self.render_with_overlay(list, None)
    }

    /// Som `render`, men med ett UI-lager ovanpå scenen.
    pub fn render_with_overlay(
        &mut self,
        list: &RenderList,
        mut overlay: Option<&mut dyn Overlay>,
    ) -> Result<(), RenderError> {
        let frame = match &self.target {
            Target::Window { surface, config } => match surface.get_current_texture() {
                wgpu::CurrentSurfaceTexture::Success(frame) => Some(frame),
                wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                    surface.configure(&self.device, config);
                    Some(frame)
                }
                wgpu::CurrentSurfaceTexture::Outdated => {
                    surface.configure(&self.device, config);
                    return Ok(());
                }
                wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                    return Ok(());
                }
                wgpu::CurrentSurfaceTexture::Validation => {
                    log::warn!("valideringsfel vid get_current_texture, hoppar över framen");
                    return Ok(());
                }
                wgpu::CurrentSurfaceTexture::Lost => return Err(RenderError::SurfaceLost),
            },
            Target::Offscreen { .. } => None,
        };

        let frame_view = frame.as_ref().map(|f| {
            f.texture
                .create_view(&wgpu::TextureViewDescriptor::default())
        });
        let color_view = match (&frame_view, &self.target) {
            (Some(view), _) => view,
            (None, Target::Offscreen { view, .. }) => view,
            (None, Target::Window { .. }) => unreachable!("fönstermål har alltid en frame"),
        };

        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::bytes_of(&CameraUniform {
                view_proj: list.view_proj.to_cols_array_2d(),
                light_dir: list.light_dir.extend(0.0).to_array(),
            }),
        );

        // Sortera per mesh så att allt med samma mesh kan ritas i ett anrop.
        // Overlay-objekten ligger sist i samma buffert.
        let mut stats = FrameStats::default();
        let phase = std::time::Instant::now();

        // Tre faser i en enda instansbuffert: ogenomskinligt, sprites,
        // overlay. De två yttre sorteras på (textur, mesh) för att minimera
        // materialbyten – sprites får *inte* sorteras om, deras ordning är
        // bakifrån-och-fram och avgör blandningen.
        let mut items = list.items.clone();
        items.sort_by_key(|item| (item.texture, item.mesh));
        let opaque_range = 0..items.len();

        items.extend_from_slice(&list.sprite_items);
        let sprite_range = opaque_range.end..items.len();

        let mut overlay_draws = list.overlay_items.clone();
        overlay_draws.sort_by_key(|item| (item.texture, item.mesh));
        items.extend_from_slice(&overlay_draws);
        let overlay_range = sprite_range.end..items.len();

        stats.drawn = items.len();

        stats.prepare = phase.elapsed();
        let phase = std::time::Instant::now();

        let instances: Vec<InstanceRaw> = items
            .iter()
            .map(|item| InstanceRaw {
                model: item.transform.to_cols_array_2d(),
                color: item.color.to_array(),
                uv_transform: item.uv_transform,
            })
            .collect();

        if instances.len() > self.instance_capacity {
            self.instance_capacity = instances.len().next_power_of_two();
            self.instance_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("instances"),
                size: (self.instance_capacity * size_of::<InstanceRaw>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        if !instances.is_empty() {
            self.queue
                .write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(&instances));
        }

        stats.upload = phase.elapsed();
        let phase = std::time::Instant::now();

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame"),
            });

        if let Some(overlay) = overlay.as_deref_mut() {
            overlay.prepare(&self.device, &self.queue, &mut encoder, self.size());
        }

        {
            // forget_lifetime: egui-wgpu kräver ett RenderPass<'static>.
            let mut pass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("main pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: color_view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: list.clear_color.r as f64,
                                g: list.clear_color.g as f64,
                                b: list.clear_color.b as f64,
                                a: list.clear_color.a as f64,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &self.depth_view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                })
                .forget_lifetime();

            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.camera_bind_group, &[]);
            pass.set_vertex_buffer(1, self.instance_buffer.slice(..));

            // Ett draw-anrop per sammanhängande körning av samma
            // (textur, mesh). Listan är sorterad, så materialbytena blir
            // så få som möjligt.
            let mut bound: Option<TextureId> = None;
            let draw = |pass: &mut wgpu::RenderPass<'static>,
                        bound: &mut Option<TextureId>,
                        from: usize,
                        to: usize| {
                let item = items[from];
                if *bound != Some(item.texture) {
                    match self.textures.get(item.texture) {
                        Some(material) => {
                            pass.set_bind_group(1, material, &[]);
                            *bound = Some(item.texture);
                        }
                        None => log::warn!("okänd textur {:?}", item.texture),
                    }
                }
                if let Some(mesh) = self.meshes.get(item.mesh) {
                    pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                    pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                    pass.draw_indexed(0..mesh.index_count, 0, from as u32..to as u32);
                } else {
                    log::warn!("okänd mesh {:?} i renderlistan", item.mesh);
                }
            };

            let mut draw_calls = 0usize;
            let run = |pass: &mut wgpu::RenderPass<'static>,
                       bound: &mut Option<TextureId>,
                       range: std::ops::Range<usize>,
                       calls: &mut usize| {
                let mut start = range.start;
                while start < range.end {
                    let key = (items[start].texture, items[start].mesh);
                    let mut end = start + 1;
                    // Sammanhängande körningar med samma material slås ihop
                    // till ett anrop. För sprites bevarar det ordningen,
                    // eftersom bara grannar slås ihop.
                    while end < range.end && (items[end].texture, items[end].mesh) == key {
                        end += 1;
                    }
                    draw(pass, bound, start, end);
                    *calls += 1;
                    start = end;
                }
            };

            run(&mut pass, &mut bound, opaque_range, &mut draw_calls);

            if !sprite_range.is_empty() {
                pass.set_pipeline(&self.sprite_pipeline);
                run(&mut pass, &mut bound, sprite_range, &mut draw_calls);
            }

            stats.draw_calls = draw_calls;

            if !overlay_range.is_empty() {
                pass.set_pipeline(&self.overlay_pipeline);
                run(&mut pass, &mut bound, overlay_range, &mut draw_calls);
            }

            if let Some(overlay) = overlay {
                overlay.paint(&mut pass);
            }
        }

        stats.record = phase.elapsed();
        let phase = std::time::Instant::now();

        self.queue.submit(std::iter::once(encoder.finish()));
        stats.submit = phase.elapsed();
        self.stats = stats;

        if let Some(frame) = frame {
            self.queue.present(frame);
        }
        Ok(())
    }

    /// Läser tillbaka offscreen-målet som rå RGBA (radvis, utan padding).
    pub fn capture_rgba(&self) -> anyhow::Result<Vec<u8>> {
        let Target::Offscreen {
            texture,
            width,
            height,
            ..
        } = &self.target
        else {
            anyhow::bail!("capture_rgba kräver ett offscreen-mål");
        };
        let (width, height) = (*width, *height);

        // copy_texture_to_buffer kräver att bytes_per_row är multipel av 256.
        let unpadded = width * 4;
        let padded = unpadded.div_ceil(256) * 256;

        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: (padded * height) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit(Some(encoder.finish()));

        let slice = buffer.slice(..);
        slice.map_async(wgpu::MapMode::Read, |result| {
            if let Err(err) = result {
                log::error!("kunde inte mappa readback-buffern: {err}");
            }
        });
        self.device.poll(wgpu::PollType::wait_indefinitely())?;

        let mapped = slice
            .get_mapped_range()
            .map_err(|err| anyhow::anyhow!("{err}"))?;
        let mut pixels = Vec::with_capacity((unpadded * height) as usize);
        for row in 0..height {
            let start = (row * padded) as usize;
            pixels.extend_from_slice(&mapped[start..start + unpadded as usize]);
        }
        Ok(pixels)
    }
}

/// Något som ritar ovanpå scenen i samma frame – i praktiken egui.
///
/// Två faser eftersom UI-bibliotek behöver ladda upp buffertar innan
/// render-passet öppnas, men rita när det är igång.
pub trait Overlay {
    fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        size: (u32, u32),
    );

    fn paint(&mut self, pass: &mut wgpu::RenderPass<'static>);
}

/// Vad som faktiskt hände under senaste framen.
#[derive(Debug, Clone, Copy, Default)]
pub struct FrameStats {
    pub drawn: usize,
    pub culled: usize,
    pub draw_calls: usize,
    /// Kopiering och sortering av ritlistan.
    pub prepare: std::time::Duration,
    /// Bygga och ladda upp instansbufferten.
    pub upload: std::time::Duration,
    /// Spela in render-passet.
    pub record: std::time::Duration,
    /// Skicka till GPU:n.
    pub submit: std::time::Duration,
}

/// Fel som anroparen faktiskt måste agera på.
#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("surface förlorad – måste skapas om")]
    SurfaceLost,
}

// ------------------------------------------------------------- interna

fn create_instance() -> wgpu::Instance {
    // WGPU_BACKEND=vulkan i miljön styr backend; annars plattformens primära.
    wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::from_env().unwrap_or(wgpu::Backends::PRIMARY),
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    })
}

async fn request_device(
    instance: &wgpu::Instance,
    surface: Option<&wgpu::Surface<'static>>,
) -> anyhow::Result<(wgpu::Adapter, wgpu::Device, wgpu::Queue)> {
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: surface,
            ..Default::default()
        })
        .await?;

    let info = adapter.get_info();
    log::info!(
        "adapter: {} ({:?}, {:?})",
        info.name,
        info.device_type,
        info.backend
    );

    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("engine device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            ..Default::default()
        })
        .await?;

    Ok((adapter, device, queue))
}

fn create_depth_view(device: &wgpu::Device, width: u32, height: u32) -> wgpu::TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("depth"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

/// Hur en pipeline förhåller sig till djupbufferten.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DepthMode {
    /// Ogenomskinligt: testar och skriver djup.
    Opaque,
    /// Genomskinligt: testar djup men skriver inte, så sprites bakom
    /// varandra blandas korrekt istället för att klippa bort varandra.
    Transparent,
    /// Gizmos: ritar alltid, skriver inget djup.
    Overlay,
}

fn build_mesh_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    camera_layout: &wgpu::BindGroupLayout,
    material_layout: &wgpu::BindGroupLayout,
    depth_mode: DepthMode,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("mesh shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/mesh.wgsl").into()),
    });

    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("mesh layout"),
        bind_group_layouts: &[Some(camera_layout), Some(material_layout)],
        immediate_size: 0,
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(match depth_mode {
            DepthMode::Opaque => "mesh pipeline",
            DepthMode::Transparent => "sprite pipeline",
            DepthMode::Overlay => "overlay pipeline",
        }),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: Default::default(),
            buffers: &[Some(Vertex::layout()), Some(InstanceRaw::layout())],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            front_face: wgpu::FrontFace::Ccw,
            // Sprites får synas bakifrån: en vänd sprite (flip_x via negativ
            // skala) skulle annars försvinna helt.
            cull_mode: match depth_mode {
                DepthMode::Opaque => Some(wgpu::Face::Back),
                _ => None,
            },
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(depth_mode == DepthMode::Opaque),
            depth_compare: Some(match depth_mode {
                DepthMode::Opaque | DepthMode::Transparent => wgpu::CompareFunction::Less,
                DepthMode::Overlay => wgpu::CompareFunction::Always,
            }),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}
