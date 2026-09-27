//! Utseende och layoutregler för en nod.
//!
//! Allt här är data: det som editorn redigerar, det som sparas i en fil
//! och det som TypeScript kan skriva om medan spelet kör.

use serde::{Deserialize, Serialize};

use crate::geom::{Color, Edges, Vec2};

/// Hur en nods barn placeras.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Layout {
    /// Under varandra.
    #[default]
    Column,
    /// Bredvid varandra.
    Row,
    /// Ovanpå varandra, var och en placerad med sin egen `anchor`.
    /// Det här är läget för HUD:ar: barnen hänger i hörn och kanter.
    Stack,
}

/// Hur stor en nod vill vara längs en axel.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub enum Size {
    /// Precis så stor som innehållet kräver.
    #[default]
    Auto,
    /// Ett bestämt antal pixlar.
    Fixed(f32),
    /// Ta den plats som blir över. Flera syskon med `Fill` delar lika.
    Fill,
    /// En andel av förälderns innermått, 0..1.
    Fraction(f32),
}

/// Placering längs tväraxeln (den layouten *inte* staplar längs).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Align {
    #[default]
    Start,
    Center,
    End,
    /// Fyll tväraxeln.
    Stretch,
}

/// Fördelning längs huvudaxeln när det finns plats över.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Justify {
    #[default]
    Start,
    Center,
    End,
    /// Mellanrum mellan barnen, inget i kanterna.
    SpaceBetween,
}

/// Var i förälderns yta en `Stack`-nod hänger.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Anchor {
    #[default]
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    Center,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

impl Anchor {
    /// Andelen av föräldern respektive av noden själv, 0..1 i båda led.
    /// Samma tal används för båda, vilket är det som gör att `BottomRight`
    /// lägger nodens *högra* kant mot förälderns högra kant.
    pub fn factors(self) -> Vec2 {
        let (x, y) = match self {
            Anchor::TopLeft => (0.0, 0.0),
            Anchor::TopCenter => (0.5, 0.0),
            Anchor::TopRight => (1.0, 0.0),
            Anchor::CenterLeft => (0.0, 0.5),
            Anchor::Center => (0.5, 0.5),
            Anchor::CenterRight => (1.0, 0.5),
            Anchor::BottomLeft => (0.0, 1.0),
            Anchor::BottomCenter => (0.5, 1.0),
            Anchor::BottomRight => (1.0, 1.0),
        };
        Vec2::new(x, y)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Style {
    pub layout: Layout,
    pub width: Size,
    pub height: Size,
    pub padding: Edges,
    /// Mellanrum mellan barn längs huvudaxeln.
    pub gap: f32,
    pub align: Align,
    pub justify: Justify,
    /// Bara meningsfullt i en `Stack`-förälder.
    pub anchor: Anchor,
    /// Förskjutning från ankaret, i pixlar.
    pub offset: Vec2,
    pub background: Option<Color>,
    /// Textfärg, ärvs inte – varje nod bär sin egen.
    pub color: Color,
    pub font_size: f32,
    pub radius: f32,
    /// Osynliga noder tar ingen plats och kan inte träffas av pekaren.
    pub visible: bool,
    /// Stänger av träffytan utan att dölja noden. En etikett ovanpå en
    /// knapp ska inte stjäla klicket.
    pub hit_test: bool,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            layout: Layout::Column,
            width: Size::Auto,
            height: Size::Auto,
            padding: Edges::ZERO,
            gap: 0.0,
            align: Align::Start,
            justify: Justify::Start,
            anchor: Anchor::TopLeft,
            offset: Vec2::ZERO,
            background: None,
            color: Color::WHITE,
            font_size: 16.0,
            radius: 0.0,
            visible: true,
            hit_test: true,
        }
    }
}

impl Style {
    /// Kortform för att bygga i kod. Editorn skriver fälten direkt.
    pub fn row() -> Self {
        Self {
            layout: Layout::Row,
            ..Default::default()
        }
    }

    pub fn column() -> Self {
        Self::default()
    }

    pub fn stack() -> Self {
        Self {
            layout: Layout::Stack,
            ..Default::default()
        }
    }

    pub fn with_size(mut self, width: Size, height: Size) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    pub fn with_padding(mut self, padding: Edges) -> Self {
        self.padding = padding;
        self
    }

    pub fn with_gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    pub fn with_background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn with_font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    pub fn with_anchor(mut self, anchor: Anchor) -> Self {
        self.anchor = anchor;
        self
    }

    pub fn with_offset(mut self, offset: Vec2) -> Self {
        self.offset = offset;
        self
    }

    pub fn with_align(mut self, align: Align) -> Self {
        self.align = align;
        self
    }

    pub fn with_justify(mut self, justify: Justify) -> Self {
        self.justify = justify;
        self
    }

    pub fn with_radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }
}
