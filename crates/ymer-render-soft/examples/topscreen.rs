//! En scen ritad på 3DS:ens övre skärm, utan GPU.
//!
//!     cargo run -p ymer-render-soft --example topscreen -- ut.png
//!
//! Poängen är inte bilden utan vad som *inte* står i den här filen:
//! ingen wgpu, ingen winit, ingen shader. Bara `RenderList` och
//! `Backend`. Går det här, går en citro3d-backend också — den byter bara
//! ut rasteriseringen mot hårdvaran.

use ymer_core::{Color, Mat4, Quat, Vec3};
use ymer_gfx::{Backend, DrawItem, RenderList, primitives};
use ymer_render_soft::SoftRenderer;

fn main() -> anyhow::Result<()> {
    let output = std::env::args()
        .nth(1)
        .unwrap_or("topscreen.png".to_string());

    let mut renderer = SoftRenderer::top_screen();
    let (width, height) = renderer.size();

    let plane = renderer.add_mesh(&primitives::plane(20.0), "mark");
    let cube = renderer.add_mesh(&primitives::cube(1.0), "kub");
    // Ett rutmönster, så att perspektivkorrekt interpolation syns. Med
    // rak interpolation böjer sig rutorna på marken.
    let checker = renderer.add_texture(&checkerboard(64), 64, 64, "rutor");

    let mut list = RenderList {
        clear_color: Color::rgb(0.05, 0.06, 0.09),
        light_dir: Vec3::new(-0.4, -1.0, -0.35).normalize(),
        ..Default::default()
    };

    // Kameran. Samma högerhänta konvention som motorn i övrigt.
    let eye = Vec3::new(6.0, 4.5, 9.0);
    // Samma kameravägg som motorn använder: högerhänt vy, DirectX-djup
    // i 0..1. Blandar man konventioner hamnar halva scenen bakom
    // närplanet utan att något ser fel ut i koden.
    let projection = ymer_core::glam::camera::rh::proj::directx::perspective(
        60f32.to_radians(),
        width as f32 / height as f32,
        0.1,
        100.0,
    );
    let view =
        ymer_core::glam::camera::rh::view::look_at_mat4(eye, Vec3::new(0.0, 0.5, 0.0), Vec3::Y)
            .inverse();
    list.view_proj = projection * view;

    list.items.push(DrawItem::new(
        plane,
        checker,
        Mat4::IDENTITY,
        Color::rgb(0.55, 0.62, 0.48),
    ));

    // En trappa av kuber: olika djup, olika vinklar mot ljuset.
    for step in 0..6 {
        let t = step as f32;
        let transform = Mat4::from_scale_rotation_translation(
            Vec3::splat(1.0),
            Quat::from_rotation_y(t * 0.4),
            Vec3::new(-4.0 + t * 1.6, 0.5 + t * 0.35, -t * 1.1),
        );
        let hue = t / 5.0;
        list.items.push(DrawItem::new(
            cube,
            TEXTURE_NONE,
            transform,
            Color::rgb(0.35 + 0.5 * hue, 0.45, 0.85 - 0.45 * hue),
        ));
    }

    // En kub bakom kameran. Den ska inte synas – och framför allt inte
    // dyka upp spegelvänd, vilket är vad som händer utan närplansklippning.
    list.items.push(DrawItem::new(
        cube,
        TEXTURE_NONE,
        Mat4::from_translation(eye + Vec3::new(0.0, 0.0, 4.0)),
        Color::rgb(1.0, 0.0, 0.0),
    ));

    // Gränssnittet: en list längs överkanten, i pixlar.
    list.ui_items.push(screen_quad(
        cube,
        0.0,
        0.0,
        width as f32,
        22.0,
        Color::rgba(0.08, 0.09, 0.12, 0.85),
    ));
    list.ui_items.push(screen_quad(
        cube,
        8.0,
        6.0,
        60.0,
        10.0,
        Color::rgb(0.29, 0.56, 0.90),
    ));

    renderer.render(&list)?;

    image::RgbaImage::from_raw(width, height, renderer.rgba8())
        .ok_or_else(|| anyhow::anyhow!("fel bildstorlek"))?
        .save(&output)?;

    println!(
        "{width}x{height}, {} ritobjekt -> {output}",
        list.items.len() + list.ui_items.len()
    );
    Ok(())
}

/// Textur 0 är den vita pixeln, precis som i wgpu-backenden.
const TEXTURE_NONE: ymer_core::TextureId = ymer_core::TextureId(0);

/// En platta i skärmrymd. Kuben duger som geometri: bara framsidan syns,
/// och gränssnittet ritas utan djuptest.
fn screen_quad(
    mesh: ymer_core::MeshId,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    color: Color,
) -> DrawItem {
    let transform = Mat4::from_translation(Vec3::new(x + width * 0.5, y + height * 0.5, 0.0))
        * Mat4::from_scale(Vec3::new(width, height, 1.0));
    DrawItem::new(mesh, TEXTURE_NONE, transform, color)
}

fn checkerboard(size: u32) -> Vec<u8> {
    let mut pixels = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let light = ((x / 8) + (y / 8)) % 2 == 0;
            let value = if light { 235 } else { 190 };
            pixels.extend_from_slice(&[value, value, value, 255]);
        }
    }
    pixels
}
