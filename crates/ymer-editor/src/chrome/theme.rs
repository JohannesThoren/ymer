//! Editorns färger och mått, på ett ställe.
//!
//! Inte för att de ska gå att byta, utan för att en panel och dess
//! avdelare måste vara överens om var kanten går. Står talen på två
//! ställen glider de isär, och det syns som en pixel glipa som ingen
//! hittar.

use ymer_ui::prelude::*;

/// Topplistens höjd. Ytan för scenen räknas från den.
pub const TOOLBAR: f32 = 34.0;
/// Radhöjd i hierarkin och filutforskaren.
pub const ROW: f32 = 20.0;
/// Hur mycket varje nivå i hierarkin dras in.
pub const INDENT: f32 = 12.0;

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub panel: Color,
    /// Bakgrunden i listor och fält – mörkare än panelen.
    pub well: Color,
    pub text: Color,
    /// Dämpad text: etiketter, sökvägar, statusrader.
    pub dim: Color,
    pub accent: Color,
    pub button: Color,
    /// Markerad rad i en lista.
    pub selected: Color,
    pub font: f32,
    pub small: f32,
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            panel: Color::rgb(0.13, 0.14, 0.17),
            well: Color::rgb(0.09, 0.10, 0.12),
            text: Color::rgb(0.88, 0.90, 0.93),
            dim: Color::rgb(0.55, 0.58, 0.64),
            accent: Color::rgb(0.29, 0.56, 0.90),
            button: Color::rgb(0.20, 0.22, 0.27),
            selected: Color::rgb(0.22, 0.34, 0.52),
            font: 13.0,
            small: 11.0,
        }
    }

    /// En knapp i topplisten eller en panel.
    pub fn button(&self) -> Style {
        Style::default()
            .with_padding(Edges::symmetric(8.0, 4.0))
            .with_background(self.button)
            .with_radius(3.0)
            .with_color(self.text)
            .with_font_size(self.font)
    }

    /// En rad i en lista: full bredd, markerbar.
    pub fn row(&self, selected: bool) -> Style {
        let style = Style::default()
            .with_size(Size::Fill, Size::Fixed(ROW))
            .with_padding(Edges::symmetric(6.0, 2.0))
            .with_color(self.text)
            .with_font_size(self.font);
        if selected {
            style.with_background(self.selected)
        } else {
            style
        }
    }

    pub fn label(&self) -> Style {
        Style::default()
            .with_color(self.text)
            .with_font_size(self.font)
    }

    pub fn dim_label(&self) -> Style {
        Style::default()
            .with_color(self.dim)
            .with_font_size(self.small)
    }

    /// Ett fält man skriver i.
    pub fn field(&self, width: Size) -> Style {
        Style::default()
            .with_size(width, Size::Fixed(22.0))
            .with_padding(Edges::symmetric(6.0, 3.0))
            .with_background(self.well)
            .with_radius(3.0)
            .with_color(self.text)
            .with_font_size(self.font)
    }

    /// En panel som håller sitt innehåll innanför kanterna.
    pub fn panel(&self, width: Size, height: Size) -> Style {
        Style::column()
            .with_size(width, height)
            .with_background(self.panel)
            .with_clip(true)
    }
}
