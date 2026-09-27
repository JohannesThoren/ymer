//! Interaktion: pekaren in, händelser ut.
//!
//! Tillståndet hålls skilt från dokumentet. Vad som är hovrat eller
//! nedtryckt hör till den här sekunden, inte till gränssnittet som sådant,
//! och ska varken sparas till fil eller redigeras i editorn.

use crate::geom::{Rect, Vec2};
use crate::layout::LaidOut;
use crate::node::{Document, Kind, Node};

/// Vad som hänt med pekaren sedan förra framen.
#[derive(Debug, Clone, Copy, Default)]
pub struct Pointer {
    pub position: Vec2,
    /// Knappen hålls nere.
    pub down: bool,
    /// Knappen trycktes ned den här framen.
    pub pressed: bool,
    /// Knappen släpptes den här framen.
    pub released: bool,
}

/// Interaktionsläget mellan frames.
#[derive(Debug, Clone, Default)]
pub struct State {
    pub hovered: Option<String>,
    pub pointer_down: bool,
    /// Noden som trycktes ned. Ett klick räknas först när pekaren
    /// släpps över *samma* nod – drar man ut och släpper händer inget,
    /// vilket är vad alla gränssnitt gör och vad folk förväntar sig.
    pressed: Option<String>,
}

/// Vad som hände i gränssnittet den här framen.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Events {
    /// Id:n på knappar som klickades.
    pub clicked: Vec<String>,
    /// Noden under pekaren, om någon.
    pub hovered: Option<String>,
    /// Pekaren är över gränssnittet. Spelet bakom ska då inte reagera –
    /// samma problem som `UiFocus` löser för egui.
    pub pointer_over_ui: bool,
}

impl Events {
    pub fn was_clicked(&self, id: &str) -> bool {
        self.clicked.iter().any(|clicked| clicked == id)
    }
}

impl State {
    /// Matar in pekaren och returnerar framens händelser.
    pub fn update(&mut self, document: &Document, laid_out: &LaidOut, pointer: Pointer) -> Events {
        let hit = hit_test(document, laid_out, pointer.position);
        let mut events = Events {
            hovered: hit.clone(),
            pointer_over_ui: hit.is_some(),
            ..Default::default()
        };

        self.hovered = hit.clone();
        self.pointer_down = pointer.down;

        if pointer.pressed {
            self.pressed = hit.clone();
        }
        if pointer.released {
            if let (Some(pressed), Some(over)) = (self.pressed.take(), hit)
                && pressed == over
                && !pressed.is_empty()
            {
                events.clicked.push(pressed);
            } else {
                self.pressed = None;
            }
        }

        events
    }
}

/// Noden under en punkt: den översta träffbara, eller `None`.
///
/// Ritordningen är föräldrar före barn, så den sist besökta träffen är
/// den som ligger överst – samma regel som den som ritar följer.
pub fn hit_test(document: &Document, laid_out: &LaidOut, point: Vec2) -> Option<String> {
    let mut index = 0usize;
    let mut best: Option<(usize, String)> = None;
    visit(
        &document.root,
        laid_out,
        point,
        &mut index,
        &mut best,
        Rect::new(f32::MIN / 2.0, f32::MIN / 2.0, f32::MAX, f32::MAX),
    );
    best.map(|(_, id)| id)
}

fn visit(
    node: &Node,
    laid_out: &LaidOut,
    point: Vec2,
    index: &mut usize,
    best: &mut Option<(usize, String)>,
    clip: Rect,
) {
    if !node.style.visible {
        return;
    }
    let Some(placed) = laid_out.nodes.get(*index) else {
        return;
    };
    let rect = placed.rect;
    let depth = placed.depth;
    *index += 1;

    // En nod utanför förälderns yta kan inte träffas.
    let visible_rect = rect.intersect(clip);

    let hittable = node.style.hit_test
        && !node.id.is_empty()
        // Bara noder som *är* något går att träffa. En panel utan
        // bakgrund är luft och ska inte blockera det som ligger bakom.
        && (matches!(node.kind, Kind::Button { .. }) || node.style.background.is_some());

    if hittable && visible_rect.contains(point) {
        let deeper = best
            .as_ref()
            .is_none_or(|(best_depth, _)| depth >= *best_depth);
        if deeper {
            *best = Some((depth, node.id.clone()));
        }
    }

    for child in &node.children {
        visit(child, laid_out, point, index, best, visible_rect);
    }
}
