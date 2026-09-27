//! En liten mjukvaruritare, delad av exemplen.
//!
//! Precis tillräckligt för att visa att ritlistan är fullständig: fyllda
//! rektanglar och glyfer ur atlasen, alfablandade. Ingen antialiasing på
//! kanterna och ingen rundning – radien ignoreras med flit, för att hålla
//! exemplen läsbara.

use ymer_ui::prelude::*;
use ymer_ui::{Command, FontAtlas};

pub struct Canvas {
    width: u32,
    height: u32,
    pixels: Vec<[f32; 4]>,
}

impl Canvas {
    pub fn new(width: u32, height: u32, background: Color) -> Self {
        Self {
            width,
            height,
            pixels: vec![background.to_array(); (width * height) as usize],
        }
    }

    fn blend(&mut self, x: i64, y: i64, color: Color, coverage: f32) {
        if x < 0 || y < 0 || x >= self.width as i64 || y >= self.height as i64 {
            return;
        }
        let alpha = (color.a * coverage).clamp(0.0, 1.0);
        if alpha <= 0.0 {
            return;
        }
        let index = (y as u32 * self.width + x as u32) as usize;
        let dst = self.pixels[index];
        self.pixels[index] = [
            dst[0] + (color.r - dst[0]) * alpha,
            dst[1] + (color.g - dst[1]) * alpha,
            dst[2] + (color.b - dst[2]) * alpha,
            1.0,
        ];
    }

    fn fill(&mut self, rect: Rect, color: Color) {
        for y in rect.y.floor() as i64..rect.bottom().ceil() as i64 {
            for x in rect.x.floor() as i64..rect.right().ceil() as i64 {
                self.blend(x, y, color, 1.0);
            }
        }
    }

    pub fn run(&mut self, list: &DrawList, atlas: &mut FontAtlas) {
        for command in &list.commands {
            match command {
                Command::Rect { rect, color, .. } => self.fill(*rect, *color),
                Command::Image { rect, tint, .. } => {
                    // Ingen texturladdning i det här exemplet; en platta
                    // i tintfärgen visar ändå att kommandot kom fram.
                    self.fill(*rect, *tint)
                }
                Command::Text {
                    rect,
                    text,
                    size,
                    color,
                } => self.text(*rect, text, *size, *color, atlas),
            }
        }
    }

    fn text(&mut self, rect: Rect, text: &str, size: f32, color: Color, atlas: &mut FontAtlas) {
        let line_height = atlas.measure("M", size).y;
        let mut pen_y = rect.y + atlas.ascent(size);

        for line in text.split('\n') {
            let mut pen_x = rect.x;
            for ch in line.chars() {
                let Some(glyph) = atlas.glyph(ch, size) else {
                    continue;
                };
                if !glyph.atlas.is_empty() {
                    let (aw, _) = atlas.size();
                    let pixels = atlas.pixels();
                    for gy in 0..glyph.atlas.height as u32 {
                        for gx in 0..glyph.atlas.width as u32 {
                            let sx = glyph.atlas.x as u32 + gx;
                            let sy = glyph.atlas.y as u32 + gy;
                            let coverage = pixels[(sy * aw + sx) as usize] as f32 / 255.0;
                            self.blend(
                                (pen_x + glyph.offset.x) as i64 + gx as i64,
                                (pen_y + glyph.offset.y) as i64 + gy as i64,
                                color,
                                coverage,
                            );
                        }
                    }
                }
                pen_x += glyph.advance;
            }
            pen_y += line_height;
        }
    }

    pub fn save(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut bytes = Vec::with_capacity(self.pixels.len() * 4);
        for pixel in &self.pixels {
            for channel in pixel {
                bytes.push((channel.clamp(0.0, 1.0) * 255.0).round() as u8);
            }
        }
        image::RgbaImage::from_raw(self.width, self.height, bytes)
            .ok_or("fel bildstorlek")?
            .save(path)?;
        Ok(())
    }
}
