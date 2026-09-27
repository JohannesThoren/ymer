//! Ritlistan: vad som ska ritas, inte hur.
//!
//! Samma gräns som motorns `RenderList` drar mot renderaren. Biblioteket
//! producerar rektanglar och textrader; den som bäddar in översätter dem
//! till sitt eget API. Det är det som gör att `ymer-ui` kan användas
//! utanför Ymer utan att något behöver ändras.

use crate::geom::{Color, Rect};
use crate::layout::LaidOut;
use crate::metrics;
use crate::node::{Document, Kind, Node};
use crate::state::State;
use crate::text::TextMeasure;

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

/// Ett ritkommando och den yta det får synas inom.
///
/// Klippet ligger *utanför* kommandot, inte som ett fält i varje variant,
/// för att det inte ska gå att läsa ett kommando utan att se sitt klipp.
/// En konsument som struntar i det ritar rader som rullat ur bild, och
/// felet syns bara när någon rullar – alltså sällan.
#[derive(Debug, Clone, PartialEq)]
pub struct Clipped {
    /// Snittet av de klippande förfäderna. `Rect::EVERYTHING` betyder att
    /// ingenting klipper, vilket är det vanliga fallet.
    pub clip: Rect,
    pub command: Command,
}

#[derive(Debug, Clone, Default)]
pub struct DrawList {
    pub commands: Vec<Clipped>,
}

impl DrawList {
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    pub fn len(&self) -> usize {
        self.commands.len()
    }

    pub fn push(&mut self, clip: Rect, command: Command) {
        // Helt bortklippta kommandon slängs här istället för hos varje
        // konsument. Det är också det som gör en lång lista billig att
        // rita: bara de rader som syns hamnar i listan.
        if clip.is_empty() {
            return;
        }
        self.commands.push(Clipped { clip, command });
    }
}

/// Bygger ritlistan ur ett uträknat träd.
///
/// `state` avgör hur knappar ser ut just nu. Den skiljs från dokumentet
/// med flit: ett hovrat läge är inget som ska sparas till fil.
/// Bygger ritlistan med antagen monospace-mätning.
///
/// Bekvämt i tester och när inget fält har fokus. Markören i ett textfält
/// placeras dock efter mätningen, så med ett proportionellt typsnitt
/// hamnar den fel – använd [`draw_with`] och skicka samma mätning som
/// layouten fick.
pub fn draw(document: &Document, laid_out: &LaidOut, state: &State) -> DrawList {
    draw_with(
        document,
        laid_out,
        state,
        &crate::text::MonospaceMetrics::default(),
    )
}

/// Som [`draw`], men med den mätning layouten använde.
///
/// Behövs för det som måste hamna *i* en text: markören i ett fält står
/// efter så många tecken, och utan samma mätning hamnar den fel.
pub fn draw_with(
    document: &Document,
    laid_out: &LaidOut,
    state: &State,
    text: &dyn TextMeasure,
) -> DrawList {
    let mut list = DrawList::default();
    let mut index = 0usize;
    emit(&document.root, laid_out, state, text, &mut index, &mut list);

    // Den öppna dropdownens lista ritas sist, ovanpå allt annat. Den
    // sticker ut ur sin förälder och kan därför inte ritas i trädets
    // ordning utan att hamna under nästa syskon.
    if let Some(open) = state.open.as_ref()
        && let (Some(node), Some(rect)) = (document.find(open), laid_out.rect(open))
        && let Kind::Dropdown { options, .. } = &node.kind
    {
        // Listan flyter ovanpå allt och klipps inte av sin förälder – en
        // dropdown i en rullbar panel ska kunna fällas ut fritt.
        let clip = Rect::EVERYTHING;
        let row = rect.height.max(1.0);
        let panel = node
            .style
            .background
            .unwrap_or(Color::rgb(0.18, 0.20, 0.25));
        for (i, option) in options.iter().enumerate() {
            let slot = Rect::new(rect.x, rect.bottom() + row * i as f32, rect.width, row);
            let hovered = slot.contains(state.pointer_position());
            list.push(
                clip,
                Command::Rect {
                    rect: slot,
                    color: if hovered { shade(panel, 1.3) } else { panel },
                    radius: 0.0,
                },
            );
            list.push(
                clip,
                Command::Text {
                    rect: slot.shrink(crate::geom::Edges::symmetric(6.0, 0.0)),
                    text: option.clone(),
                    size: node.style.font_size,
                    color: node.style.color,
                },
            );
        }
    }

    list
}

fn emit(
    node: &Node,
    laid_out: &LaidOut,
    state: &State,
    text: &dyn TextMeasure,
    index: &mut usize,
    list: &mut DrawList,
) {
    if !node.style.visible {
        return;
    }
    let Some(placed) = laid_out.nodes.get(*index) else {
        return;
    };
    let rect = placed.rect;
    let clip = placed.clip;
    let content = placed.content;
    *index += 1;

    // Helt utanför sitt klipp: varken noden eller dess barn kan synas.
    // Barnen måste ändå räknas bort ur `index`, annars glider listan ur
    // fas med trädet och alla efterföljande noder ritas på fel plats.
    if rect.intersect(clip).is_empty() {
        for child in &node.children {
            skip(child, index);
        }
        return;
    }

    let interactive = matches!(
        node.kind,
        Kind::Button { .. } | Kind::Checkbox { .. } | Kind::Radio { .. } | Kind::Dropdown { .. }
    );
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
        list.push(
            clip,
            Command::Rect {
                rect,
                color,
                radius: node.style.radius,
            },
        );
    }

    match &node.kind {
        Kind::Label { text } | Kind::Button { text } => {
            list.push(
                clip,
                Command::Text {
                    rect: rect.shrink(node.style.padding),
                    text: text.clone(),
                    size: node.style.font_size,
                    color: node.style.color,
                },
            );
        }
        Kind::Image { source } => {
            list.push(
                clip,
                Command::Image {
                    rect,
                    source: source.clone(),
                    tint: node.style.color,
                },
            );
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
                list.push(
                    clip,
                    Command::Rect {
                        rect: filled,
                        color: *fill,
                        radius: node.style.radius,
                    },
                );
            }
        }
        Kind::Checkbox { label, checked } | Kind::Radio { label, checked, .. } => {
            let radio = matches!(node.kind, Kind::Radio { .. });
            let box_size = node.style.font_size;
            let square = Rect::new(
                rect.x,
                rect.y + (rect.height - box_size) * 0.5,
                box_size,
                box_size,
            );
            list.push(
                clip,
                Command::Rect {
                    rect: square,
                    color: Color::rgb(0.12, 0.13, 0.17),
                    // Radioknappen är rund, kryssrutan kantig. Rundningen är
                    // ritarens ansvar; mjukvaruexemplet struntar i den.
                    radius: if radio { box_size * 0.5 } else { 3.0 },
                },
            );
            if *checked {
                let inset = box_size * 0.28;
                list.push(
                    clip,
                    Command::Rect {
                        rect: Rect::new(
                            square.x + inset,
                            square.y + inset,
                            box_size - inset * 2.0,
                            box_size - inset * 2.0,
                        ),
                        color: node.style.color,
                        radius: if radio { box_size * 0.5 } else { 2.0 },
                    },
                );
            }
            list.push(
                clip,
                Command::Text {
                    rect: Rect::new(
                        square.right() + metrics::GAP,
                        rect.y,
                        (rect.width - box_size - metrics::GAP).max(0.0),
                        rect.height,
                    ),
                    text: label.clone(),
                    size: node.style.font_size,
                    color: node.style.color,
                },
            );
        }

        Kind::Slider {
            value, min, max, ..
        } => {
            let span = (*max - *min).abs().max(f32::EPSILON);
            let t = ((*value - *min) / span).clamp(0.0, 1.0);
            let track_height = (rect.height * 0.3).max(3.0);
            let track = Rect::new(
                rect.x,
                rect.y + (rect.height - track_height) * 0.5,
                rect.width,
                track_height,
            );
            list.push(
                clip,
                Command::Rect {
                    rect: track,
                    color: Color::rgb(0.12, 0.13, 0.17),
                    radius: track_height * 0.5,
                },
            );
            // Fylld del fram till greppet.
            let travel = (rect.width - metrics::HANDLE).max(0.0);
            list.push(
                clip,
                Command::Rect {
                    rect: Rect::new(
                        track.x,
                        track.y,
                        metrics::HANDLE * 0.5 + travel * t,
                        track.height,
                    ),
                    color: node.style.color,
                    radius: track_height * 0.5,
                },
            );
            list.push(
                clip,
                Command::Rect {
                    rect: Rect::new(rect.x + travel * t, rect.y, metrics::HANDLE, rect.height),
                    color: if hovered || held {
                        shade(node.style.color, 1.25)
                    } else {
                        Color::rgb(0.86, 0.89, 0.94)
                    },
                    radius: metrics::HANDLE * 0.5,
                },
            );
        }

        Kind::TextInput {
            text: value,
            placeholder,
        } => {
            let focused = state.focused.as_deref() == Some(node.id.as_str());
            let inner = rect.shrink(node.style.padding);
            let empty = value.is_empty();
            list.push(
                clip,
                Command::Text {
                    rect: inner,
                    text: if empty {
                        placeholder.clone()
                    } else {
                        value.clone()
                    },
                    size: node.style.font_size,
                    color: if empty {
                        node.style.color.with_alpha(0.45)
                    } else {
                        node.style.color
                    },
                },
            );
            if focused {
                caret(
                    list,
                    clip,
                    Field {
                        inner,
                        size: node.style.font_size,
                        color: node.style.color,
                        text,
                    },
                    value,
                    state.caret,
                );
            }
        }

        Kind::TextArea { text: value, .. } => {
            let focused = state.focused.as_deref() == Some(node.id.as_str());
            let inner = rect.shrink(node.style.padding);
            list.push(
                clip,
                Command::Text {
                    rect: inner,
                    text: value.clone(),
                    size: node.style.font_size,
                    color: node.style.color,
                },
            );
            if focused {
                // Markören hamnar på den rad den står i.
                let before: String = value.chars().take(state.caret).collect();
                let line = before.rsplit('\n').next().unwrap_or("");
                let row = before.matches('\n').count() as f32;
                let line_height = text.measure("M", node.style.font_size).y;
                let offset = text.measure(line, node.style.font_size).x;
                list.push(
                    clip,
                    Command::Rect {
                        rect: Rect::new(
                            inner.x + offset,
                            inner.y + row * line_height,
                            metrics::CARET,
                            line_height,
                        ),
                        color: node.style.color,
                        radius: 0.0,
                    },
                );
            }
        }

        Kind::Dropdown {
            options,
            selected,
            placeholder,
        } => {
            let inner = rect.shrink(node.style.padding);
            let chosen = selected.and_then(|i| options.get(i));
            list.push(
                clip,
                Command::Text {
                    rect: inner,
                    text: chosen.cloned().unwrap_or_else(|| placeholder.clone()),
                    size: node.style.font_size,
                    color: if chosen.is_some() {
                        node.style.color
                    } else {
                        node.style.color.with_alpha(0.45)
                    },
                },
            );
            // Pilen: en liten platta i högerkanten. Att rita en triangel
            // hade krävt ett nytt ritkommando, och listan ska vara smal.
            let size = metrics::ARROW * 0.5;
            list.push(
                clip,
                Command::Rect {
                    rect: Rect::new(
                        inner.right() - metrics::ARROW,
                        inner.y + (inner.height - size) * 0.5,
                        size,
                        size,
                    ),
                    color: node.style.color.with_alpha(0.7),
                    radius: 1.0,
                },
            );
        }

        Kind::Scroll { offset } => {
            // En rullningslist, men bara när det finns något att rulla.
            // Den ritas *före* barnen och hamnar därför under dem; det gör
            // inget, för den ligger i kanten där inget innehåll når.
            let inner = rect.shrink(node.style.padding);
            if content.y > inner.height + 0.5 {
                let bar = Rect::new(
                    inner.right() - metrics::SCROLLBAR,
                    inner.y,
                    metrics::SCROLLBAR,
                    inner.height,
                );
                let andel = (inner.height / content.y).clamp(0.05, 1.0);
                let travel = inner.height * (1.0 - andel);
                let max = (content.y - inner.height).max(1.0);
                let t = (offset.y / max).clamp(0.0, 1.0);
                list.push(
                    clip,
                    Command::Rect {
                        rect: bar,
                        color: Color::rgba(0.0, 0.0, 0.0, 0.25),
                        radius: metrics::SCROLLBAR * 0.5,
                    },
                );
                list.push(
                    clip,
                    Command::Rect {
                        rect: Rect::new(bar.x, bar.y + travel * t, bar.width, inner.height * andel),
                        color: node.style.color.with_alpha(0.5),
                        radius: metrics::SCROLLBAR * 0.5,
                    },
                );
            }
        }

        Kind::Panel | Kind::Spacer => {}
    }

    for child in &node.children {
        emit(child, laid_out, state, text, index, list);
    }
}

/// Textmarkören efter `caret` tecken.
fn caret(list: &mut DrawList, clip: Rect, field: Field, value: &str, caret: usize) {
    let before: String = value.chars().take(caret).collect();
    let offset = field.text.measure(&before, field.size).x;
    let height = field.text.measure("M", field.size).y;
    list.push(
        clip,
        Command::Rect {
            rect: Rect::new(
                field.inner.x + offset,
                field.inner.y,
                metrics::CARET,
                height,
            ),
            color: field.color,
            radius: 0.0,
        },
    );
}

/// Måtten ett textfält ritas med.
#[derive(Clone, Copy)]
struct Field<'a> {
    inner: Rect,
    size: f32,
    color: Color,
    text: &'a dyn TextMeasure,
}

/// Räknar bort ett delträd ur `index` utan att rita det.
///
/// Osynliga noder hoppas över redan i layouten, så bara de synliga har en
/// plats i listan – samma filter måste gälla här.
fn skip(node: &Node, index: &mut usize) {
    if !node.style.visible {
        return;
    }
    *index += 1;
    for child in &node.children {
        skip(child, index);
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
