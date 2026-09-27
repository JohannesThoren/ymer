//! Editorns gränssnitt, byggt av `ymer-ui`.
//!
//! Motorn ritar sitt eget verktyg. Det är inte en poäng i sig: editorn är
//! det hårdaste provet gränssnittsbiblioteket kan få – rullande listor,
//! hopfällbara avsnitt, sifferfält man drar i, paneler man drar isär – och
//! allt som saknas märks här först.
//!
//! # Retained mode, byggt om varje frame
//!
//! `ymer-ui` är retained: ett träd av noder som ligger kvar. Editorns
//! innehåll beror ändå på vad som är markerat, så trädet byggs om varje
//! frame. De två går ihop genom att id:n är stabila och genom
//! [`Document::carry_view_state_from`], som bär över det som hör till
//! *vyn* – rullningslägen, utfällda avsnitt, panelbredder, halvskrivna
//! tal – medan värdena kommer ur världen.
//!
//! Ordningen i en frame är därför:
//!
//! 1. läs världen,
//! 2. mata pekare och tangenter till förra framens träd,
//! 3. läs tillbaka det som ändrades och gör om det till kommandon,
//! 4. verkställ kommandona mot världen,
//! 5. bygg ett nytt träd ur den nya världen och bär över vy-tillståndet.
//!
//! Steg 3 före steg 5 är hela knuten: läser man tillbaka efter
//! ombyggnaden är det den nybyggda nodens värde man läser, inte
//! användarens ändring.
//!
//! # Id:t är sökvägen
//!
//! Ett fält i inspektorn heter `f/Transform/translation/0`. Samma funktion
//! bygger id:t när noden skapas och när värdet läses tillbaka, så de kan
//! inte glida isär. Utan det hade varje komponenttyp behövt en egen
//! avläsare.

mod fields;
mod theme;
mod winit_input;

use bevy_ecs::prelude::*;
use ymer_scene::TypeRegistry;
use ymer_ui::prelude::*;
use ymer_ui::{Input, LaidOut, State};

use crate::{Action, EditorState, collect_hierarchy};

pub use theme::Theme;
pub use winit_input::InputPump;

/// Editorns gränssnitt mellan frames.
pub struct Chrome {
    document: Document,
    state: State,
    theme: Theme,
    /// Panelernas mått, som avdelarna styr. Ligger här och inte i
    /// dokumentet, eftersom den som bygger behöver dem *innan* trädet
    /// finns.
    left: f32,
    right: f32,
    bottom: f32,
    /// Hålet i mitten, som layouten gav det. Se [`Chrome::viewport`].
    scene: Rect,
    /// Förra framens layout, för [`Chrome::over_ui`].
    laid_out: LaidOut,
}

impl Default for Chrome {
    fn default() -> Self {
        Self::new()
    }
}

impl Chrome {
    pub fn new() -> Self {
        Self {
            document: Document::new(Node::panel()),
            state: State::default(),
            theme: Theme::dark(),
            left: 240.0,
            right: 340.0,
            bottom: 190.0,
            scene: Rect::new(0.0, 0.0, 0.0, 0.0),
            laid_out: LaidOut::default(),
        }
    }

    pub fn document(&self) -> &Document {
        &self.document
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    /// Ytan i mitten, där scenen syns.
    ///
    /// Hämtas ur layouten i stället för att räknas ut. En uträkning måste
    /// känna till varje avdelares tjocklek och varje panels utfyllnad,
    /// och glider isär med trädet så fort någon av dem ändras – felet
    /// syns som att plockningen träffar bredvid muspekaren.
    pub fn viewport(&self, window: Rect) -> Rect {
        if self.scene.is_empty() {
            // Före första framen finns ingen layout att fråga.
            return Rect::new(
                window.x + self.left,
                window.y + theme::TOOLBAR,
                (window.width - self.left - self.right).max(0.0),
                (window.height - theme::TOOLBAR).max(0.0),
            );
        }
        self.scene
    }

    /// En hel frame: input in, ritlista ut, världen uppdaterad.
    pub fn frame(
        &mut self,
        input: &Input,
        window: Rect,
        editor: &mut EditorState,
        world: &mut World,
        registry: &TypeRegistry,
        text: &dyn TextMeasure,
    ) -> Frame {
        // 1–2. Förra framens träd tar emot pekaren.
        let laid_out = layout(&self.document, window, text);
        let events = self.state.update_with(&mut self.document, &laid_out, input);

        // 3. Vad ändrades? Läses ur det träd användaren faktiskt rörde.
        let mut actions = Vec::new();
        self.read_back(editor, world, registry, &mut actions);
        self.read_events(&events, editor, world, registry, &mut actions);

        // 4. Verkställ. Efter det här kan markeringen och världen se helt
        //    annorlunda ut, vilket är just därför trädet byggs om.
        crate::apply_actions(actions, editor, world, registry);

        // 5. Nytt träd ur den nya världen. Rålägets textrutor behöver
        //    en text att visa innan någon skrivit något; den hämtas ur
        //    komponenten och bryts upp så den går att läsa.
        seed_drafts(editor, world, registry);
        let previous = std::mem::replace(&mut self.document, Document::new(Node::panel()));
        self.document = self.build(editor, world, registry);
        self.document.carry_view_state_from(&previous);
        // Avdelarna bär panelmåtten; läs tillbaka dem för nästa frame.
        self.read_splits();

        let laid_out = layout(&self.document, window, text);
        self.scene = laid_out.rect("scen").unwrap_or(window);
        let list = draw_with(&self.document, &laid_out, &self.state, text);
        self.laid_out = laid_out;

        Frame {
            list,
            pointer_over_ui: events.pointer_over_ui,
            keyboard_captured: events.keyboard_captured,
        }
    }

    /// Hör punkten till gränssnittet?
    ///
    /// Frågas av fönstret när ett musklick kommer in, innan framen körs.
    /// Att i stället lita på förra framens svar hade räckt nästan alltid
    /// – och missat just det klick som kommer i samma frame som pekaren
    /// gled in över en panel, vilket är precis det klick som skjuter i
    /// scenen bakom knappen man siktade på.
    pub fn over_ui(&self, point: Vec2) -> bool {
        !self.laid_out.nodes.is_empty()
            && ymer_ui::hit_test(&self.document, &self.laid_out, point).is_some()
    }

    fn read_splits(&mut self) {
        for (id, slot) in [
            ("split/left", &mut self.left),
            ("split/right", &mut self.right),
            ("split/bottom", &mut self.bottom),
        ] {
            if let Some(NodeValue::Number(value)) = self.document.value(id) {
                *slot = value;
            }
        }
    }
}

/// Vad en frame gav.
pub struct Frame {
    pub list: DrawList,
    /// Pekaren är över gränssnittet; scenen bakom ska inte reagera.
    pub pointer_over_ui: bool,
    /// Ett fält har tangentbordet; genvägar ska inte lyssna.
    pub keyboard_captured: bool,
}

// Byggandet och avläsningen ligger i egna filer: de är långa, och den
// här filen ska gå att läsa för att förstå ordningen i en frame.
mod build;
mod read;

/// Raderna i hierarkin, med entiteten de hör till.
pub(crate) fn hierarchy_rows(
    world: &World,
    registry: &TypeRegistry,
) -> Vec<(Entity, String, usize)> {
    collect_hierarchy(world, registry)
        .into_iter()
        .map(|row| (row.entity, row.name, row.depth))
        .collect()
}

/// Komponenterna på en entitet: närvarande som JSON, och de som saknas.
pub(crate) fn components(
    world: &World,
    registry: &TypeRegistry,
    entity: Entity,
) -> (Vec<(String, serde_json::Value)>, Vec<String>) {
    let mut present = Vec::new();
    let mut missing = Vec::new();
    for component in registry.iter() {
        match component.read_json(world, entity) {
            Some(json) => present.push((component.name.to_string(), json)),
            None => {
                if component.read(world, entity).is_none() {
                    missing.push(component.name.to_string());
                }
            }
        }
    }
    (present, missing)
}

/// Fyller rålägets buffertar för den markerade entiteten.
///
/// Bara de som saknas: en buffert man redan skriver i ska inte skrivas
/// över av världen mitt i meningen.
fn seed_drafts(editor: &mut EditorState, world: &World, registry: &TypeRegistry) {
    if !editor.raw_mode {
        return;
    }
    let Some(entity) = editor.selected.filter(|e| world.entities().contains(*e)) else {
        return;
    };
    for component in registry.iter() {
        let Some(value) = component.read(world, entity) else {
            continue;
        };
        editor
            .drafts
            .entry((entity, component.name.to_string()))
            .or_insert_with(|| crate::pretty(value.get_ron()));
    }
}

/// Kommandon `read_events` och `read_back` samlar ihop.
pub(crate) type Actions = Vec<Action>;
