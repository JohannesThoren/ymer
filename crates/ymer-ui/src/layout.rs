//! Layoutpasset: från träd till rektanglar.
//!
//! Två pass, som de flesta layoutmotorer. Först *mäter* vi varje nod
//! nerifrån och upp – hur stor vill den vara? Sedan *placerar* vi uppifrån
//! och ner, när föräldern vet hur mycket plats den har att dela ut.
//!
//! Anledningen till två pass: `Size::Auto` beror på barnen, `Size::Fill`
//! beror på föräldern. Ett enda pass kan inte lösa båda.

use crate::geom::{Rect, Vec2};
use crate::node::{Document, Kind, Node};
use crate::style::{Align, Justify, Layout, Size};
use crate::text::TextMeasure;

/// En uträknad nod: var den hamnade, och vilken nod det var.
#[derive(Debug, Clone, PartialEq)]
pub struct Placed {
    pub id: String,
    pub rect: Rect,
    /// Ytan innanför utfyllnaden, där barnen ligger.
    pub inner: Rect,
    /// Index i `LaidOut::nodes` för föräldern, om någon.
    pub parent: Option<usize>,
    /// Djup i trädet; roten är 0. Ritordning följer listan, och djupet
    /// används för att avgöra vilken träff som ligger överst.
    pub depth: usize,
}

/// Resultatet av ett layoutpass, parallellt med trädet.
#[derive(Debug, Clone, Default)]
pub struct LaidOut {
    pub nodes: Vec<Placed>,
}

impl LaidOut {
    pub fn rect(&self, id: &str) -> Option<Rect> {
        self.nodes
            .iter()
            .find(|placed| placed.id == id)
            .map(|placed| placed.rect)
    }
}

/// Nodens *egenstorlek*: så liten den kan vara utan att innehållet
/// klipps. `Size::Fill` räknas här som sitt innehåll, inte som hela ytan.
///
/// Skillnaden är hela poängen. Om Fill mätte sig som "all tillgänglig
/// plats" skulle en Fill-nod blåsa upp sin Auto-förälder till mer än
/// föräldern har – en utfyllnad i en rad gjorde topplisten dubbelt så
/// bred som fönstret. Den faktiska utfyllnaden sker i `place`, som är
/// den enda som vet hur mycket som blev över.
fn measure(node: &Node, text: &dyn TextMeasure, available: Vec2) -> Vec2 {
    if !node.style.visible {
        return Vec2::ZERO;
    }
    let style = &node.style;
    let padding = style.padding;

    // Innehållets egen storlek.
    let content = match &node.kind {
        Kind::Label { text: s } | Kind::Button { text: s } => text.measure(s, style.font_size),
        // En bild eller stapel har ingen inneboende storlek här; den
        // styrs av `width`/`height`. Att gissa på bildens pixelmått hade
        // krävt att layouten kände till texturerna.
        Kind::Image { .. } | Kind::Bar { .. } | Kind::Spacer => Vec2::ZERO,
        Kind::Panel => Vec2::ZERO,
    };

    // Barnens samlade storlek.
    let inner_available = Vec2::new(
        (available.x - padding.horizontal()).max(0.0),
        (available.y - padding.vertical()).max(0.0),
    );
    let mut children = Vec2::ZERO;
    let visible: Vec<&Node> = node
        .children
        .iter()
        .filter(|child| child.style.visible)
        .collect();

    for (index, child) in visible.iter().enumerate() {
        let size = measure(child, text, inner_available);
        match style.layout {
            Layout::Column => {
                children.x = children.x.max(size.x);
                children.y += size.y;
                if index + 1 < visible.len() {
                    children.y += style.gap;
                }
            }
            Layout::Row => {
                children.y = children.y.max(size.y);
                children.x += size.x;
                if index + 1 < visible.len() {
                    children.x += style.gap;
                }
            }
            Layout::Stack => {
                children.x = children.x.max(size.x);
                children.y = children.y.max(size.y);
            }
        }
    }

    let natural = Vec2::new(
        content.x.max(children.x) + padding.horizontal(),
        content.y.max(children.y) + padding.vertical(),
    );

    Vec2::new(
        resolve(style.width, natural.x, available.x),
        resolve(style.height, natural.y, available.y),
    )
}

fn resolve(size: Size, natural: f32, available: f32) -> f32 {
    match size {
        Size::Auto => natural,
        Size::Fixed(v) => v,
        // Se kommentaren över `measure`: Fill är sitt innehåll här.
        Size::Fill => natural,
        Size::Fraction(f) => available * f,
    }
}

/// Egenstorleken, men med `Fill` utsträckt till den givna ytan.
/// Används där en nod placeras direkt mot en yta i stället för att dela
/// den med syskon: roten, och barn i en `Stack`.
fn stretched(node: &Node, text: &dyn TextMeasure, available: Vec2) -> Vec2 {
    let mut size = measure(node, text, available);
    if node.style.width == Size::Fill {
        size.x = available.x;
    }
    if node.style.height == Size::Fill {
        size.y = available.y;
    }
    size
}

/// Räknar ut var allt hamnar inom `viewport`.
///
/// Roten behandlas som vilken nod som helst: den mäts mot ytan och
/// placeras efter sitt eget ankare. Att i stället ge roten hela ytan hade
/// gjort dess `width` och `height` verkningslösa – ett `Size::Fixed(300)`
/// på roten hade tyst blivit hela fönstret.
pub fn layout(document: &Document, viewport: Rect, text: &dyn TextMeasure) -> LaidOut {
    let mut out = LaidOut::default();
    let root = &document.root;
    if !root.style.visible {
        return out;
    }

    let size = stretched(root, text, viewport.size());
    let factors = root.style.anchor.factors();
    let position = Vec2::new(
        viewport.x + (viewport.width - size.x) * factors.x + root.style.offset.x,
        viewport.y + (viewport.height - size.y) * factors.y + root.style.offset.y,
    );

    place(
        root,
        Rect::from_pos_size(position, size),
        text,
        None,
        0,
        &mut out,
    );
    out
}

fn place(
    node: &Node,
    slot: Rect,
    text: &dyn TextMeasure,
    parent: Option<usize>,
    depth: usize,
    out: &mut LaidOut,
) {
    if !node.style.visible {
        return;
    }
    let style = &node.style;
    let inner = slot.shrink(style.padding);

    let index = out.nodes.len();
    out.nodes.push(Placed {
        id: node.id.clone(),
        rect: slot,
        inner,
        parent,
        depth,
    });

    let visible: Vec<&Node> = node
        .children
        .iter()
        .filter(|child| child.style.visible)
        .collect();
    if visible.is_empty() {
        return;
    }

    match style.layout {
        Layout::Stack => {
            // Varje barn placeras för sig, mot sitt eget ankare.
            for child in visible {
                let size = stretched(child, text, inner.size());
                let factors = child.style.anchor.factors();
                let position = Vec2::new(
                    inner.x + (inner.width - size.x) * factors.x + child.style.offset.x,
                    inner.y + (inner.height - size.y) * factors.y + child.style.offset.y,
                );
                place(
                    child,
                    Rect::from_pos_size(position, size),
                    text,
                    Some(index),
                    depth + 1,
                    out,
                );
            }
        }
        Layout::Row | Layout::Column => {
            let row = style.layout == Layout::Row;
            let main_available = if row { inner.width } else { inner.height };

            // Mät alla, och notera vilka som vill växa.
            let mut sizes: Vec<Vec2> = Vec::with_capacity(visible.len());
            let mut fill_count = 0usize;
            let mut fixed_total = 0.0f32;

            for child in &visible {
                let size = measure(child, text, inner.size());
                let wants_fill = if row {
                    child.style.width == Size::Fill
                } else {
                    child.style.height == Size::Fill
                };
                if wants_fill {
                    fill_count += 1;
                } else {
                    fixed_total += if row { size.x } else { size.y };
                }
                sizes.push(size);
            }

            let gaps = style.gap * visible.len().saturating_sub(1) as f32;
            let leftover = (main_available - fixed_total - gaps).max(0.0);
            // Syskon med Fill delar det som blir över, lika.
            let per_fill = if fill_count > 0 {
                leftover / fill_count as f32
            } else {
                0.0
            };

            // Med Fill finns inget över att fördela med `justify`.
            let slack = if fill_count > 0 { 0.0 } else { leftover };
            let (mut cursor, between) = match style.justify {
                Justify::Start => (0.0, style.gap),
                Justify::Center => (slack * 0.5, style.gap),
                Justify::End => (slack, style.gap),
                Justify::SpaceBetween if visible.len() > 1 => (
                    0.0,
                    style.gap + slack / visible.len().saturating_sub(1) as f32,
                ),
                Justify::SpaceBetween => (slack * 0.5, style.gap),
            };

            for (child, size) in visible.iter().zip(sizes) {
                let wants_fill = if row {
                    child.style.width == Size::Fill
                } else {
                    child.style.height == Size::Fill
                };
                let main = if wants_fill {
                    per_fill
                } else if row {
                    size.x
                } else {
                    size.y
                };

                // Tväraxeln styrs av förälderns `align`, utom när barnet
                // självt bett om Fill eller ett fast mått.
                let cross_available = if row { inner.height } else { inner.width };
                let child_cross = if row { size.y } else { size.x };
                // Ett barn som självt bett om Fill på tväraxeln sträcks
                // oavsett förälderns `align`; att be om det och ändå bli
                // innehållsstort vore förvirrande.
                let child_fills_cross = if row {
                    child.style.height == Size::Fill
                } else {
                    child.style.width == Size::Fill
                };
                let cross_size = if child_fills_cross || style.align == Align::Stretch {
                    cross_available
                } else {
                    child_cross.min(cross_available)
                };
                let cross_offset = match style.align {
                    Align::Start | Align::Stretch => 0.0,
                    Align::Center => (cross_available - cross_size) * 0.5,
                    Align::End => cross_available - cross_size,
                };

                let rect = if row {
                    Rect::new(inner.x + cursor, inner.y + cross_offset, main, cross_size)
                } else {
                    Rect::new(inner.x + cross_offset, inner.y + cursor, cross_size, main)
                };

                place(child, rect, text, Some(index), depth + 1, out);
                cursor += main + between;
            }
        }
    }
}
