//! Samma scen genom båda renderarna, sida vid sida.
//!
//!     cargo run -p ymer-render-soft --example jamfor --features wgpu -- ut.png
//!
//! Det här är vad gränsen är till för. `scen()` nedan är generisk över
//! [`Backend`] och vet inte vilken renderare den matar — den anropar
//! `add_mesh`, `add_texture`, `render` och får en bild. Den ena vägen går
//! genom wgpu och en riktig GPU, den andra genom en trippelloop i
//! mjukvara.
//!
//! Två renderare som är oense är en bugg i minst en av dem, och den
//! oenigheten syns inte förrän någon jämför. Det är samma princip som
//! gjorde att marken saknades i mjukvarurenderaren: motorns pipeline
//! sorterar inte bort ryggsidor, men rasteriseraren gjorde det.

use ymer_core::{Color, Mat4, MeshId, Quat, TextureId, Vec3};
use ymer_gfx::{Backend, DrawItem, RenderList, primitives};
use ymer_render::Renderer;
use ymer_render_soft::SoftRenderer;

const WIDTH: u32 = 400;
const HEIGHT: u32 = 240;
const TEXTURE_NONE: TextureId = TextureId(0);

fn main() -> anyhow::Result<()> {
    let output = std::env::args().nth(1).unwrap_or("jamfor.png".to_string());

    // Mjukvara.
    let mut soft = SoftRenderer::new(WIDTH, HEIGHT);
    let list = scen(&mut soft);
    soft.render(&list)?;
    let mjukvara = soft.rgba8();

    // wgpu.
    let mut hard = pollster::block_on(Renderer::new_offscreen(WIDTH, HEIGHT))?;
    let list = scen(&mut hard);
    hard.render(&list)?;
    let gpu = hard.capture_rgba()?;

    // Skillnaden, kanal för kanal.
    let (mut summa, mut varsta) = (0u64, 0u32);
    for (a, b) in mjukvara
        .as_chunks::<4>()
        .0
        .iter()
        .zip(gpu.as_chunks::<4>().0)
    {
        for channel in 0..3 {
            let diff = a[channel].abs_diff(b[channel]) as u32;
            summa += diff as u64;
            varsta = varsta.max(diff);
        }
    }
    let medel = summa as f64 / (WIDTH * HEIGHT * 3) as f64;
    println!("medelskillnad {medel:.2} av 255, värsta pixel {varsta}");

    // Sida vid sida: mjukvara till vänster, GPU till höger.
    let mut bild = image::RgbaImage::new(WIDTH * 2, HEIGHT);
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let index = ((y * WIDTH + x) * 4) as usize;
            bild.put_pixel(x, y, image::Rgba(hamta(&mjukvara, index)));
            bild.put_pixel(x + WIDTH, y, image::Rgba(hamta(&gpu, index)));
        }
    }
    bild.save(&output)?;
    println!("skrev {output} ({}x{HEIGHT})", WIDTH * 2);
    Ok(())
}

fn hamta(pixels: &[u8], index: usize) -> [u8; 4] {
    [
        pixels[index],
        pixels[index + 1],
        pixels[index + 2],
        pixels[index + 3],
    ]
}

/// Bygger scenen mot vilken backend som helst.
///
/// Signaturen är hela poängen: `impl Backend` och inget mer. Ingenting
/// här vet om det finns en GPU i andra änden.
fn scen(backend: &mut impl Backend) -> RenderList {
    let plane = backend.add_mesh(&primitives::plane(20.0), "mark");
    let cube = backend.add_mesh(&primitives::cube(1.0), "kub");
    let checker = backend.add_texture(&rutor(64), 64, 64, "rutor");

    let eye = Vec3::new(6.0, 4.5, 9.0);
    // Samma kameravägg som motorn använder: högerhänt vy, DirectX-djup
    // i 0..1. Blandar man konventioner hamnar halva scenen bakom
    // närplanet utan att något ser fel ut i koden.
    let projection = ymer_core::glam::camera::rh::proj::directx::perspective(
        60f32.to_radians(),
        WIDTH as f32 / HEIGHT as f32,
        0.1,
        100.0,
    );
    let view =
        ymer_core::glam::camera::rh::view::look_at_mat4(eye, Vec3::new(0.0, 0.5, 0.0), Vec3::Y)
            .inverse();

    let mut list = RenderList {
        view_proj: projection * view,
        clear_color: Color::rgb(0.05, 0.06, 0.09),
        light_dir: Vec3::new(-0.4, -1.0, -0.35).normalize(),
        ..Default::default()
    };

    list.items.push(DrawItem::new(
        plane,
        checker,
        Mat4::IDENTITY,
        Color::rgb(0.55, 0.62, 0.48),
    ));

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

    // Bakom kameran: ska inte synas i någon av dem.
    list.items.push(DrawItem::new(
        cube,
        TEXTURE_NONE,
        Mat4::from_translation(eye + Vec3::new(0.0, 0.0, 4.0)),
        Color::rgb(1.0, 0.0, 0.0),
    ));

    list.ui_items.push(platta(
        cube,
        0.0,
        0.0,
        WIDTH as f32,
        22.0,
        Color::rgba(0.08, 0.09, 0.12, 0.85),
    ));
    list.ui_items.push(platta(
        cube,
        8.0,
        6.0,
        60.0,
        10.0,
        Color::rgb(0.29, 0.56, 0.90),
    ));

    list
}

fn platta(mesh: MeshId, x: f32, y: f32, width: f32, height: f32, color: Color) -> DrawItem {
    let transform = Mat4::from_translation(Vec3::new(x + width * 0.5, y + height * 0.5, 0.0))
        * Mat4::from_scale(Vec3::new(width, height, 1.0));
    DrawItem::new(mesh, TEXTURE_NONE, transform, color)
}

fn rutor(size: u32) -> Vec<u8> {
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
