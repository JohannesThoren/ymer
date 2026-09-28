//! Ymer på en Nintendo 3DS.
//!
//! Samma scen som `cargo run -p ymer-render-soft --example topscreen`
//! ritar på skrivbordet: en rutig mark, sex kuber i en trappa, en list i
//! skärmrymd. Det är med flit samma scen — kan man lägga de två bilderna
//! bredvid varandra och se samma sak har sömmen hållit hela vägen från
//! wgpu till hårdvara från 2011.
//!
//! Bygg och kör:
//!
//!     cd 3ds
//!     cargo 3ds run --release
//!
//! Se `README.md` i samma mapp för vad som krävs och vad som ännu är
//! oprövat.

#![feature(allocator_api)]

mod backend;

use backend::{Citro3dBackend, Screen};

use ctru::prelude::*;
use ctru::services::gfx::{RawFrameBuffer, Screen as _, TopScreen3D};

use ymer_core::{Color, Mat4, MeshId, Quat, TextureId, Vec3};
use ymer_gfx::{Backend, DrawItem, RenderList, primitives};

const TEXTURE_NONE: TextureId = TextureId(0);

fn main() {
    let gfx = Gfx::new().expect("ingen GFX");
    let mut hid = Hid::new().expect("ingen HID");
    let apt = Apt::new().expect("ingen APT");
    let _console = Console::new(gfx.bottom_screen.borrow_mut());

    println!("Ymer 3DS");
    println!("START avslutar");

    let (width, height) = Screen::Top.size();
    let mut renderer = match Citro3dBackend::new(width, height) {
        Ok(renderer) => renderer,
        Err(err) => {
            println!("kunde inte starta GPU:n: {err}");
            vanta(&apt, &mut hid);
            return;
        }
    };

    // Övre skärmen. Vi ritar bara vänster öga tills stereon är prövad.
    let top = TopScreen3D::from(&gfx.top_screen);
    let (mut left, _right) = top.split_mut();
    let RawFrameBuffer { width: fb_w, height: fb_h, .. } = left.raw_framebuffer();
    let mut target = renderer
        .instance_mut()
        .render_target(fb_w, fb_h, left, Some(citro3d::render::DepthFormat::Depth24Stencil8))
        .expect("inget rendermål");

    let (plane, cube, checker) = ladda(&mut renderer);
    let clear = ymer_pica::color_word(Color::rgb(0.05, 0.06, 0.09));

    let mut vinkel = 0.0f32;
    while apt.main_loop() {
        hid.scan_input();
        if hid.keys_down().contains(KeyPad::START) {
            break;
        }

        vinkel += 0.01;
        let list = scen(plane, cube, checker, vinkel);

        // Instansen och resurserna lånas var för sig: `render_frame_with`
        // håller instansen under hela framen.
        let (instance, scene) = renderer.split();
        instance.render_frame_with(|mut frame| {
            target.clear(citro3d::render::ClearFlags::ALL, clear, 0);
            let _ = frame.select_render_target(&target);
            scene.draw(&mut frame, &list, Screen::Top);
            frame
        });
    }
}

fn ladda(renderer: &mut Citro3dBackend) -> (MeshId, MeshId, TextureId) {
    // Vit 1x1 duger inte: PICA vill ha minst 8x8.
    let vit = vec![255u8; 8 * 8 * 4];
    let _white = renderer.add_texture(&vit, 8, 8, "vit");

    let plane = renderer.add_mesh(&primitives::plane(20.0), "mark");
    let cube = renderer.add_mesh(&primitives::cube(1.0), "kub");
    let checker = renderer.add_texture(&rutor(64), 64, 64, "rutor");
    (plane, cube, checker)
}

fn scen(plane: MeshId, cube: MeshId, checker: TextureId, vinkel: f32) -> RenderList {
    let (width, height) = Screen::Top.size();
    let eye = Vec3::new(6.0 * vinkel.cos(), 4.5, 9.0 * vinkel.sin().abs().max(0.4));

    let projection = ymer_core::glam::camera::rh::proj::directx::perspective(
        60f32.to_radians(),
        width as f32 / height as f32,
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

    list.ui_items.push(platta(
        cube,
        0.0,
        0.0,
        width as f32,
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

fn vanta(apt: &Apt, hid: &mut Hid) {
    while apt.main_loop() {
        hid.scan_input();
        if hid.keys_down().contains(KeyPad::START) {
            break;
        }
    }
}
