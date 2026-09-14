//! Kopplar egui till renderarens `Overlay`-hook.

use crate::{DEPTH_FORMAT, Overlay, wgpu};
use egui_wgpu::{Renderer as EguiRenderer, RendererOptions, ScreenDescriptor};

pub struct EguiOverlay {
    renderer: EguiRenderer,
    paint_jobs: Vec<egui::ClippedPrimitive>,
    textures_delta: egui::TexturesDelta,
    pixels_per_point: f32,
    screen: ScreenDescriptor,
}

impl EguiOverlay {
    pub fn new(device: &wgpu::Device, output_format: wgpu::TextureFormat) -> Self {
        Self {
            renderer: EguiRenderer::new(
                device,
                output_format,
                RendererOptions {
                    // Måste matcha render-passets djupfäste, annars vägrar wgpu.
                    depth_stencil_format: Some(DEPTH_FORMAT),
                    msaa_samples: 1,
                    ..Default::default()
                },
            ),
            paint_jobs: Vec::new(),
            textures_delta: egui::TexturesDelta::default(),
            pixels_per_point: 1.0,
            screen: ScreenDescriptor {
                size_in_pixels: [1, 1],
                pixels_per_point: 1.0,
            },
        }
    }

    /// Tar emot resultatet av en egui-frame.
    pub fn accept(&mut self, ctx: &egui::Context, output: egui::FullOutput, size: (u32, u32)) {
        self.pixels_per_point = output.pixels_per_point;
        self.paint_jobs = ctx.tessellate(output.shapes, output.pixels_per_point);
        self.textures_delta.append(output.textures_delta);
        self.screen = ScreenDescriptor {
            size_in_pixels: [size.0, size.1],
            pixels_per_point: output.pixels_per_point,
        };
    }
}

impl Overlay for EguiOverlay {
    fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        size: (u32, u32),
    ) {
        self.screen = ScreenDescriptor {
            size_in_pixels: [size.0, size.1],
            pixels_per_point: self.pixels_per_point,
        };

        // TexturesDelta har Drop, så vi lånar istället för att flytta ut fälten.
        let mut delta = std::mem::take(&mut self.textures_delta);
        for (id, images) in &delta.set {
            // 0.36 kan skicka flera deltan per textur-id.
            for image in images {
                self.renderer.update_texture(device, queue, *id, image);
            }
        }
        for id in &delta.free {
            self.renderer.free_texture(id);
        }
        // Drop-implementationen panikar om deltan inte kvitteras.
        delta.clear();

        // Buffertuppladdningen kan returnera egna kommandobuffertar.
        let commands =
            self.renderer
                .update_buffers(device, queue, encoder, &self.paint_jobs, &self.screen);
        if !commands.is_empty() {
            queue.submit(commands);
        }
    }

    fn paint(&mut self, pass: &mut wgpu::RenderPass<'static>) {
        self.renderer.render(pass, &self.paint_jobs, &self.screen);
    }
}
