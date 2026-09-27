//! Text: mätning för layouten och rastrering för den som ritar.
//!
//! Mätningen är ett trait, inte en konkret typ, av två skäl. Layouten ska
//! gå att testa utan en typsnittsfil, och den som bäddar in biblioteket
//! kan redan ha en typsnittsstack och vill inte ha en till.
//!
//! Rastreringen ligger ändå *här*, bakom `text`-flaggan, och inte hos den
//! som ritar. Annars hade varje backend fått bygga sin egen atlas, och då
//! hade "fristående" bara varit sant på pappret.

use crate::geom::{Rect, Vec2};

/// Hur brett och högt en textsnutt blir.
pub trait TextMeasure {
    fn measure(&self, text: &str, size: f32) -> Vec2;

    /// Avståndet från radens överkant ner till baslinjen. Behövs för att
    /// placera glyfer i en rektangel layouten redan bestämt.
    fn ascent(&self, size: f32) -> f32 {
        size * 0.8
    }
}

/// Mätning som antar att alla tecken är lika breda.
///
/// Ger fel bredd för proportionella typsnitt, men gör layouten körbar och
/// testbar utan en enda fil på disk. Testerna använder den med flit: en
/// layoutbugg ska synas som fel tal, inte som ett annat typsnitt.
#[derive(Debug, Clone, Copy)]
pub struct MonospaceMetrics {
    /// Teckenbredd som andel av teckenstorleken.
    pub advance: f32,
    /// Radhöjd som andel av teckenstorleken.
    pub line_height: f32,
}

impl Default for MonospaceMetrics {
    fn default() -> Self {
        Self {
            advance: 0.5,
            line_height: 1.2,
        }
    }
}

impl TextMeasure for MonospaceMetrics {
    fn measure(&self, text: &str, size: f32) -> Vec2 {
        let lines = text.split('\n');
        let mut widest = 0usize;
        let mut count = 0usize;
        for line in lines {
            widest = widest.max(line.chars().count());
            count += 1;
        }
        Vec2::new(
            widest as f32 * self.advance * size,
            count.max(1) as f32 * self.line_height * size,
        )
    }
}

/// En glyf placerad i atlasen.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Glyph {
    /// Var i atlasen glyfen ligger, i pixlar.
    pub atlas: Rect,
    /// Förskjutning från penpositionen till glyfens övre vänstra hörn.
    pub offset: Vec2,
    /// Hur långt pennan flyttas efter glyfen.
    pub advance: f32,
}

#[cfg(feature = "text")]
pub use font::{Font, FontAtlas};

#[cfg(feature = "text")]
mod font {
    use std::collections::BTreeMap;

    use ab_glyph::{Font as _, FontVec, PxScale, ScaleFont as _};

    use super::{Glyph, TextMeasure};
    use crate::geom::{Rect, Vec2};

    /// Ett laddat typsnitt.
    pub struct Font {
        inner: FontVec,
    }

    impl Font {
        /// Från en .ttf eller .otf i minnet.
        pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, String> {
            FontVec::try_from_vec(bytes)
                .map(|inner| Self { inner })
                .map_err(|err| format!("kunde inte läsa typsnittet: {err}"))
        }
    }

    impl TextMeasure for Font {
        fn measure(&self, text: &str, size: f32) -> Vec2 {
            let scaled = self.inner.as_scaled(PxScale::from(size));
            let line_height = scaled.height() + scaled.line_gap();
            let mut widest = 0.0f32;
            let mut lines = 0usize;

            for line in text.split('\n') {
                let mut width = 0.0;
                let mut previous = None;
                for ch in line.chars() {
                    let id = scaled.glyph_id(ch);
                    if let Some(prev) = previous {
                        width += scaled.kern(prev, id);
                    }
                    width += scaled.h_advance(id);
                    previous = Some(id);
                }
                widest = widest.max(width);
                lines += 1;
            }

            Vec2::new(widest, lines.max(1) as f32 * line_height)
        }

        fn ascent(&self, size: f32) -> f32 {
            self.inner.as_scaled(PxScale::from(size)).ascent()
        }
    }

    /// Rastrerade glyfer i en gråskaleatlas.
    ///
    /// Atlasen växer radvis: enkelt, och gott nog för ett gränssnitt som
    /// använder en handfull storlekar. En riktig motor packar bättre.
    pub struct FontAtlas {
        font: Font,
        width: u32,
        height: u32,
        /// En byte per pixel, täckningsgrad.
        pixels: Vec<u8>,
        glyphs: BTreeMap<(u32, char), Option<Glyph>>,
        pen_x: u32,
        pen_y: u32,
        row_height: u32,
        /// Sätts när något rastrerats sedan senaste uppladdningen.
        dirty: bool,
    }

    impl FontAtlas {
        pub fn new(font: Font, width: u32, height: u32) -> Self {
            Self {
                font,
                width,
                height,
                pixels: vec![0; (width * height) as usize],
                glyphs: BTreeMap::new(),
                // Lämna första raden fri för en helvit pixel, så att den
                // som ritar kan använda *en* textur för både text och
                // fyllda rektanglar.
                pen_x: 2,
                pen_y: 0,
                row_height: 2,
                dirty: true,
            }
        }

        /// Skapar en atlas med ett inbyggt typsnitt om det finns, annars
        /// ett fel. Den som bäddar in väljer själv typsnitt.
        pub fn from_font_bytes(bytes: Vec<u8>, width: u32, height: u32) -> Result<Self, String> {
            Ok(Self::new(Font::from_bytes(bytes)?, width, height))
        }

        pub fn font(&self) -> &Font {
            &self.font
        }

        pub fn size(&self) -> (u32, u32) {
            (self.width, self.height)
        }

        /// Täckningsgrad per pixel, en byte styck.
        pub fn pixels(&self) -> &[u8] {
            &self.pixels
        }

        /// Samma data som RGBA, vilket är vad de flesta texturformat vill
        /// ha. Vit med alfa ur täckningen, så den går att tona med
        /// nodens färg.
        pub fn rgba(&self) -> Vec<u8> {
            let mut out = Vec::with_capacity(self.pixels.len() * 4);
            for &coverage in &self.pixels {
                out.extend_from_slice(&[255, 255, 255, coverage]);
            }
            out
        }

        pub fn take_dirty(&mut self) -> bool {
            std::mem::take(&mut self.dirty)
        }

        /// Den vita pixeln, för fyllda ytor. Ligger i hörnet och är alltid
        /// helt täckt.
        pub fn white_pixel(&mut self) -> Rect {
            if self.pixels[0] != 255 {
                self.pixels[0] = 255;
                self.dirty = true;
            }
            Rect::new(0.0, 0.0, 1.0, 1.0)
        }

        /// Rastrerar glyfen om den inte redan finns. `None` betyder att
        /// tecknet inte har någon bild – mellanslag, till exempel.
        pub fn glyph(&mut self, ch: char, size: f32) -> Option<Glyph> {
            // Storleken kvantiseras, annars fylls atlasen av nästan
            // identiska kopior när något animerar en textstorlek.
            let key = ((size * 4.0).round() as u32, ch);
            if let Some(cached) = self.glyphs.get(&key) {
                return *cached;
            }

            let result = self.rasterize(ch, size);
            self.glyphs.insert(key, result);
            result
        }

        fn rasterize(&mut self, ch: char, size: f32) -> Option<Glyph> {
            let scaled = self.font.inner.as_scaled(PxScale::from(size));
            let id = scaled.glyph_id(ch);
            let advance = scaled.h_advance(id);

            let glyph = id.with_scale(PxScale::from(size));
            let Some(outline) = self.font.inner.outline_glyph(glyph) else {
                // Inget att rita, men pennan ska ändå flyttas.
                return Some(Glyph {
                    atlas: Rect::ZERO,
                    offset: Vec2::ZERO,
                    advance,
                });
            };

            let bounds = outline.px_bounds();
            let w = bounds.width().ceil() as u32;
            let h = bounds.height().ceil() as u32;
            if w == 0 || h == 0 {
                return Some(Glyph {
                    atlas: Rect::ZERO,
                    offset: Vec2::ZERO,
                    advance,
                });
            }

            // Radbrytning i atlasen.
            if self.pen_x + w + 1 >= self.width {
                self.pen_x = 0;
                self.pen_y += self.row_height + 1;
                self.row_height = 0;
            }
            if self.pen_y + h >= self.height {
                // Full atlas. Hellre en glyf utan bild än en panik.
                return None;
            }

            let (ox, oy) = (self.pen_x, self.pen_y);
            let width = self.width;
            outline.draw(|gx, gy, coverage| {
                let x = ox + gx;
                let y = oy + gy;
                let index = (y * width + x) as usize;
                if let Some(slot) = self.pixels.get_mut(index) {
                    // Max, inte addition: glyfer får inte lysa upp
                    // varandra om de råkar överlappa en pixel.
                    let value = (coverage * 255.0) as u8;
                    *slot = (*slot).max(value);
                }
            });

            self.pen_x += w + 1;
            self.row_height = self.row_height.max(h);
            self.dirty = true;

            Some(Glyph {
                atlas: Rect::new(ox as f32, oy as f32, w as f32, h as f32),
                offset: Vec2::new(bounds.min.x, bounds.min.y),
                advance,
            })
        }
    }

    impl TextMeasure for FontAtlas {
        fn measure(&self, text: &str, size: f32) -> Vec2 {
            self.font.measure(text, size)
        }

        fn ascent(&self, size: f32) -> f32 {
            self.font.ascent(size)
        }
    }
}
