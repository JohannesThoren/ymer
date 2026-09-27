//! Ritar ett gränssnitt till en PNG, utan Ymer och utan GPU.
//!
//! Exemplet finns för att bevisa gränsen. Det importerar `ymer_ui` och
//! ingenting annat ur motorn, och rastreringen nedan är trettio rader
//! mjukvara – ingen wgpu, inget fönster. Går det att köra fungerar
//! biblioteket var som helst.
//!
//!     cargo run -p ymer-ui --example standalone -- ut.png [typsnitt.ttf]

use ymer_ui::prelude::*;
use ymer_ui::{Command, FontAtlas, Placed};

const WIDTH: u32 = 640;
const HEIGHT: u32 = 360;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args().nth(1).unwrap_or("ui.png".to_string());
    let font_path = std::env::args()
        .nth(2)
        .unwrap_or("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf".to_string());

    let mut atlas = FontAtlas::from_font_bytes(std::fs::read(&font_path)?, 512, 512)?;
    let document = hud();

    let viewport = Rect::new(0.0, 0.0, WIDTH as f32, HEIGHT as f32);
    let laid_out = layout(&document, viewport, &atlas);

    // Pekaren över "Bygg"-knappen, så att hovringen syns i bilden.
    let mut state = State::default();
    let hover = laid_out
        .rect("bygg")
        .map(|r| r.center())
        .unwrap_or(Vec2::ZERO);
    state.update(
        &document,
        &laid_out,
        Pointer {
            position: hover,
            ..Default::default()
        },
    );

    let commands = draw(&document, &laid_out, &state);
    let mut canvas = Canvas::new(WIDTH, HEIGHT, Color::rgb(0.10, 0.11, 0.14));
    canvas.run(&commands, &mut atlas);
    canvas.save(&output)?;

    println!(
        "{} noder, {} ritkommandon -> {output}",
        laid_out.nodes.len(),
        commands.len()
    );
    // Bevisar att uppslag på id fungerar utan att man vet var noden ligger.
    for id in ["guld", "antal", "topp"] {
        if let Some(Placed { rect, .. }) = laid_out.nodes.iter().find(|p| p.id == id) {
            println!("{id:8} {rect:?}");
        } else {
            println!("{id:8} placerades inte alls");
        }
    }
    Ok(())
}

/// En HUD byggd i kod. Exakt samma träd kan komma ur en .ron-fil.
fn hud() -> Document {
    let panel = Color::rgba(0.16, 0.18, 0.23, 0.95);
    let accent = Color::rgb(0.29, 0.56, 0.90);
    let dim = Color::rgb(0.62, 0.66, 0.74);

    Document::new(
        Node::panel()
            .with_style(Style::stack().with_size(Size::Fill, Size::Fill))
            // Topprad med nyckeltal.
            .with_child(
                Node::panel()
                    .with_id("topp")
                    .with_style(
                        Style::row()
                            .with_size(Size::Fill, Size::Fixed(48.0))
                            .with_padding(Edges::symmetric(16.0, 10.0))
                            .with_gap(24.0)
                            .with_align(Align::Center)
                            .with_background(panel),
                    )
                    .with_children([
                        nyckeltal("guld", "Guld", "1 170"),
                        nyckeltal("folk", "Invånare", "20 / 20"),
                        nyckeltal("inkomst", "Inkomst", "21,8/s"),
                        Node::spacer()
                            .with_style(Style::default().with_size(Size::Fill, Size::Fixed(1.0))),
                        Node::label("10 byggnader")
                            .with_id("antal")
                            .with_style(Style::default().with_color(dim).with_font_size(13.0)),
                    ]),
            )
            // Byggpalett till vänster.
            .with_child(
                Node::panel()
                    .with_id("palett")
                    .with_style(
                        Style::column()
                            .with_size(Size::Fixed(190.0), Size::Fixed(210.0))
                            .with_anchor(Anchor::CenterLeft)
                            .with_offset(Vec2::new(16.0, 20.0))
                            .with_padding(Edges::all(12.0))
                            .with_gap(8.0)
                            .with_background(panel)
                            .with_radius(6.0),
                    )
                    .with_children([
                        Node::label("Bygg")
                            .with_style(Style::default().with_font_size(15.0).with_color(dim)),
                        knapp("bygg", "Hus – 50 guld", accent),
                        knapp("butik", "Butik – 80 guld", Color::rgb(0.25, 0.28, 0.35)),
                        knapp("kontor", "Kontor – 300", Color::rgb(0.25, 0.28, 0.35)),
                        Node::label("Befolkning")
                            .with_style(Style::default().with_font_size(12.0).with_color(dim)),
                        Node::bar(0.8, accent).with_id("stapel").with_style(
                            Style::default()
                                .with_size(Size::Fill, Size::Fixed(10.0))
                                .with_background(Color::rgb(0.11, 0.12, 0.15))
                                .with_radius(3.0),
                        ),
                    ]),
            )
            // Statusrad längst ner.
            .with_child(
                Node::panel()
                    .with_style(
                        Style::row()
                            .with_size(Size::Fill, Size::Fixed(30.0))
                            .with_anchor(Anchor::BottomLeft)
                            .with_padding(Edges::symmetric(16.0, 7.0))
                            .with_background(panel),
                    )
                    .with_child(
                        Node::label("Ritad utan motor och utan GPU.")
                            .with_id("status")
                            .with_style(Style::default().with_font_size(13.0).with_color(dim)),
                    ),
            ),
    )
}

fn nyckeltal(id: &str, etikett: &str, varde: &str) -> Node {
    Node::panel()
        .with_id(id)
        .with_style(Style::column().with_gap(2.0))
        .with_children([
            Node::label(etikett).with_style(
                Style::default()
                    .with_font_size(11.0)
                    .with_color(Color::rgb(0.55, 0.59, 0.67)),
            ),
            Node::label(varde).with_style(Style::default().with_font_size(16.0)),
        ])
}

fn knapp(id: &str, text: &str, color: Color) -> Node {
    Node::button(id, text).with_style(
        Style::default()
            .with_size(Size::Fill, Size::Fixed(28.0))
            .with_padding(Edges::symmetric(10.0, 6.0))
            .with_background(color)
            .with_radius(4.0)
            .with_font_size(13.0),
    )
}

// ------------------------------------------------- liten mjukvaruritare

/// Precis tillräckligt för att visa att ritlistan är fullständig: fyllda
/// rektanglar och glyfer ur atlasen, alfablandade. Ingen antialiasing på
/// kanterna och ingen rundning – radien ignoreras med flit, för att hålla
/// exemplet läsbart.
struct Canvas {
    width: u32,
    height: u32,
    pixels: Vec<[f32; 4]>,
}

impl Canvas {
    fn new(width: u32, height: u32, background: Color) -> Self {
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

    fn run(&mut self, list: &DrawList, atlas: &mut FontAtlas) {
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

    fn save(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
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
