//! Gränssnitt som data.
//!
//! `ymer-ui` är ett retained-mode-bibliotek: ett UI är ett träd av noder
//! man kan spara, läsa in, redigera och peka på – inte kod som körs varje
//! frame. Det valet följer av vad biblioteket ska klara:
//!
//! * **Skapas i en editor.** Ett träd går att rita upp visuellt och spara.
//!   En funktion går inte.
//! * **Ändras från TypeScript.** Ett skript slår upp en nod på id och
//!   skriver dess text. Med immediate mode hade skriptet behövt äga hela
//!   ritslingan och korsa wasm-gränsen varje frame.
//! * **Användas utanför Ymer.** Inget här beror på någon annan del av
//!   motorn. Ut kommer en [`DrawList`] av rektanglar och textrader som
//!   vilken renderare som helst kan tolka.
//!
//! Slingan är densamma varje frame:
//!
//! ```
//! use ymer_ui::prelude::*;
//!
//! let document = Document::new(
//!     Node::panel()
//!         .with_style(Style::stack().with_size(Size::Fill, Size::Fill))
//!         .with_child(Node::button("spela", "Spela").with_style(
//!             Style::default()
//!                 .with_size(Size::Fixed(120.0), Size::Fixed(32.0))
//!                 .with_background(Color::rgb(0.2, 0.4, 0.8)),
//!         )),
//! );
//!
//! let metrics = MonospaceMetrics::default();
//! let viewport = Rect::new(0.0, 0.0, 800.0, 600.0);
//!
//! let laid_out = layout(&document, viewport, &metrics);
//! let mut state = State::default();
//!
//! // Pekaren mitt på knappen, nedtryckt och sedan släppt.
//! let pa_knappen = Vec2::new(60.0, 16.0);
//! state.update(&document, &laid_out, Pointer {
//!     position: pa_knappen, down: true, pressed: true, ..Default::default()
//! });
//! let events = state.update(&document, &laid_out, Pointer {
//!     position: pa_knappen, released: true, ..Default::default()
//! });
//!
//! assert!(events.was_clicked("spela"));
//! assert!(events.pointer_over_ui, "spelet bakom ska inte reagera");
//!
//! let commands = draw(&document, &laid_out, &state);
//! assert!(!commands.is_empty());
//! ```

pub mod draw;
pub mod geom;
pub mod layout;
pub mod node;
pub mod state;
pub mod style;
pub mod text;

pub use draw::{Command, DrawList, draw};
pub use geom::{Color, Edges, Rect, Vec2};
pub use layout::{LaidOut, Placed, layout};
pub use node::{Document, Kind, Node};
pub use state::{Events, Pointer, State, hit_test};
pub use style::{Align, Anchor, Justify, Layout, Size, Style};
pub use text::{MonospaceMetrics, TextMeasure};

#[cfg(feature = "text")]
pub use text::{Font, FontAtlas};

pub mod prelude {
    pub use crate::draw::{Command, DrawList, draw};
    pub use crate::geom::{Color, Edges, Rect, Vec2};
    pub use crate::layout::layout;
    pub use crate::node::{Document, Kind, Node};
    pub use crate::state::{Events, Pointer, State};
    pub use crate::style::{Align, Anchor, Justify, Layout, Size, Style};
    pub use crate::text::{MonospaceMetrics, TextMeasure};
}
