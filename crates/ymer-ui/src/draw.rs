//! Ritlistan: vad som ska ritas, inte hur.
//!
//! Samma gräns som motorns `RenderList` drar mot renderaren. Biblioteket
//! producerar rektanglar och textrader; den som bäddar in översätter dem
//! till sitt eget API. Det är det som gör att `ymer-ui` kan användas
//! utanför Ymer utan att något behöver ändras.

use crate::geom::{Color, Rect};
use crate::layout::LaidOut;
use crate::node::{Document, Kind, Node};
use crate::state::State;

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// Fylld rektangel.
    Rect {
        rect: Rect,
        color: Color,
        radius: f32,
    },
    /// En textrad. Den som ritar hämtar glyferna ur atlasen.
    Text {
        /// Rektangeln layouten gav noden; texten börjar i dess vänsterkant.
        rect: Rect,
        text: String,
        size: f32,
        color: Color,
    },
    /// Namngiven textur, uppslagen av den som ritar.
    Image {
        rect: Rect,
        source: String,
        tint: Color,
    },
}

#[derive(Debug, Clone, Default)]
pub struct DrawList {
    pub commands: Vec<Command>,
}

impl DrawList {
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    pub fn len(&self) -> usize {
        self.commands.len()
    }
}

/// Bygger ritlistan ur ett uträknat träd.
///
/// `state` avgör hur knappar ser ut just nu. Den skiljs från dokumentet
/// med flit: ett hovrat läge är inget som ska sparas till fil.
pub fn draw(document: &Document, laid_out: &LaidOut, state: &State) -> DrawList {
    let mut list = DrawList::default();
    let mut index = 0usize;
    emit(&document.root, laid_out, state, &mut index, &mut list);
    list
}

fn emit(node: &Node, laid_out: &LaidOut, state: &State, index: &mut usize, list: &mut DrawList) {
    if !node.style.visible {
        return;
    }
    let Some(placed) = laid_out.nodes.get(*index) else {
        return;
    };
    let rect = placed.rect;
    *index += 1;

    let interactive = matches!(node.kind, Kind::Button { .. });
    let hovered = interactive && state.hovered.as_deref() == Some(node.id.as_str());
    let held = hovered && state.pointer_down;

    // Bakgrund.
    if let Some(background) = node.style.background {
        let color = if held {
            shade(background, 0.82)
        } else if hovered {
            shade(background, 1.18)
        } else {
            background
        };
        list.commands.push(Command::Rect {
            rect,
            color,
            radius: node.style.radius,
        });
    }

    match &node.kind {
        Kind::Label { text } | Kind::Button { text } => {
            list.commands.push(Command::Text {
                rect: rect.shrink(node.style.padding),
                text: text.clone(),
                size: node.style.font_size,
                color: node.style.color,
            });
        }
        Kind::Image { source } => {
            list.commands.push(Command::Image {
                rect,
                source: source.clone(),
                tint: node.style.color,
            });
        }
        Kind::Bar { value, fill } => {
            let inner = rect.shrink(node.style.padding);
            let filled = Rect::new(
                inner.x,
                inner.y,
                inner.width * value.clamp(0.0, 1.0),
                inner.height,
            );
            if !filled.is_empty() {
                list.commands.push(Command::Rect {
                    rect: filled,
                    color: *fill,
                    radius: node.style.radius,
                });
            }
        }
        Kind::Panel | Kind::Spacer => {}
    }

    for child in &node.children {
        emit(child, laid_out, state, index, list);
    }
}

/// Ljusare eller mörkare variant, för hovrade och nedtryckta knappar.
fn shade(color: Color, factor: f32) -> Color {
    Color::rgba(
        (color.r * factor).clamp(0.0, 1.0),
        (color.g * factor).clamp(0.0, 1.0),
        (color.b * factor).clamp(0.0, 1.0),
        color.a,
    )
}
