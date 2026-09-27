//! Interaktion: pekare och tangenter in, ändringar ut.
//!
//! Tillståndet hålls skilt från dokumentet, men *värdena* gör det inte.
//! Att bocka i en kryssruta ändrar noden, för det är noden som editorn
//! redigerar och TypeScript läser. Vad som råkar vara hovrat eller
//! fokuserat just den här sekunden hör däremot hemma här, och ska varken
//! sparas till fil eller synas i inspectorn.

use crate::geom::{Rect, Vec2};
use crate::layout::LaidOut;
use crate::node::{Document, Kind, Node, clear_group};

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
    /// Det här nedtrycket är det andra i ett dubbelklick.
    ///
    /// Biblioteket har ingen klocka och ska inte skaffa sig en; den som
    /// bäddar in vet redan vad systemet räknar som ett dubbelklick.
    pub double: bool,
    /// Hjulets rörelse den här framen, i pixlar. Positiv y rullar nedåt i
    /// innehållet, som i alla andra gränssnitt.
    pub scroll: Vec2,
}

/// Tangenter som betyder något för ett textfält.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Backspace,
    Delete,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    Enter,
    Tab,
    Escape,
}

/// Allt gränssnittet får veta om en frame.
///
/// Texten kommer färdigtolkad från fönstersystemet – den som bäddar in
/// skickar in de tecken som faktiskt skrevs, inte råa tangentkoder.
/// Annars hade biblioteket behövt känna till tangentbordslayouter, och
/// en svensk å hade blivit fel.
#[derive(Debug, Clone, Default)]
pub struct Input {
    pub pointer: Pointer,
    /// Tecken som skrevs den här framen.
    pub text: String,
    /// Redigeringstangenter som trycktes ned.
    pub keys: Vec<Key>,
}

impl Input {
    pub fn from_pointer(pointer: Pointer) -> Self {
        Self {
            pointer,
            ..Default::default()
        }
    }

    /// Trycktes tangenten den här framen?
    pub fn pressed_key(&self, key: Key) -> bool {
        self.keys.contains(&key)
    }
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
    /// Noden som dras just nu, till exempel ett reglage. Den behåller
    /// pekaren även när den glider utanför sin egen rektangel; annars
    /// hade reglaget släppt så fort man drog lite för långt.
    active: Option<String>,
    /// Fältet som tar emot tangenter.
    pub focused: Option<String>,
    /// Öppen dropdown, om någon.
    pub open: Option<String>,
    /// Markörens plats i det fokuserade fältet, räknat i tecken.
    pub caret: usize,
    /// Pekarens läge när dragningen började, och hur långt den flyttat
    /// sig. Ett sifferfält skiljer på att dras och att klickas, och
    /// skillnaden är just den här sträckan.
    drag_origin: Vec2,
    drag_distance: f32,
    /// Nedtrycket som pågår var det andra i ett dubbelklick. Släppet är
    /// det som räknas som klick, så flaggan måste överleva dit.
    pressed_double: bool,
    /// Hela utkastet i sifferfältet är markerat, så att första tecknet
    /// ersätter det. Biblioteket har ingen allmän markering, men just den
    /// här är inte en finess: ett fält där man klickar och skriver 7 ska
    /// bli 7, inte 1.507.
    select_all: bool,
    /// Pekarens senaste läge, så att ritaren kan hovra rader i en öppen
    /// dropdown utan att få pekaren skickad till sig separat.
    pointer_position: Vec2,
}

/// Vad som hände i gränssnittet den här framen.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Events {
    /// Id:n på knappar som klickades.
    pub clicked: Vec<String>,
    /// Id:n vars värde ändrades: kryssrutor, reglage, fält, dropdowns.
    pub changed: Vec<String>,
    /// Fält där Enter trycktes.
    pub submitted: Vec<String>,
    /// Noden under pekaren, om någon.
    pub hovered: Option<String>,
    /// Pekaren är över gränssnittet. Spelet bakom ska då inte reagera –
    /// samma problem som `UiFocus` löser för egui.
    pub pointer_over_ui: bool,
    /// Id:n som dubbelklickades. En dubbelklickad nod rapporteras även
    /// som klickad, så den som bara bryr sig om klick slipper veta.
    pub double_clicked: Vec<String>,
    /// Hjulet togs av en rullbar yta. Spelet – eller editorns kamera –
    /// ska då inte också zooma.
    pub scroll_consumed: bool,
    /// Ett fält har tangentbordsfokus. Spelet ska då inte tolka WASD som
    /// rörelse medan någon skriver sitt namn.
    pub keyboard_captured: bool,
}

impl Events {
    /// Dubbelklickades noden den här framen?
    pub fn was_double_clicked(&self, id: &str) -> bool {
        self.double_clicked.iter().any(|other| other == id)
    }

    pub fn was_clicked(&self, id: &str) -> bool {
        self.clicked.iter().any(|clicked| clicked == id)
    }

    pub fn was_changed(&self, id: &str) -> bool {
        self.changed.iter().any(|changed| changed == id)
    }

    pub fn was_submitted(&self, id: &str) -> bool {
        self.submitted.iter().any(|submitted| submitted == id)
    }
}

impl State {
    pub fn pointer_position(&self) -> Vec2 {
        self.pointer_position
    }

    /// Kortform när bara pekaren är intressant.
    pub fn update(
        &mut self,
        document: &mut Document,
        laid_out: &LaidOut,
        pointer: Pointer,
    ) -> Events {
        self.update_with(document, laid_out, &Input::from_pointer(pointer))
    }

    /// Matar in en frames indata och returnerar vad som hände.
    ///
    /// Dokumentet tas som `&mut` eftersom interaktion *är* en ändring av
    /// det: en ibockad ruta är ett nytt värde på noden, inte ett
    /// sidotillstånd som någon annan måste kopiera tillbaka.
    pub fn update_with(
        &mut self,
        document: &mut Document,
        laid_out: &LaidOut,
        input: &Input,
    ) -> Events {
        let pointer = input.pointer;
        let hit = hit_test(document, laid_out, pointer.position);

        let mut events = Events {
            hovered: hit.clone(),
            pointer_over_ui: hit.is_some() || self.open.is_some(),
            ..Default::default()
        };

        self.hovered = hit.clone();
        self.pointer_down = pointer.down;
        self.pointer_position = pointer.position;

        // Hjulet först, och skilt från klick: man ska kunna rulla en lista
        // utan att markeringen i den ändras.
        if pointer.scroll != Vec2::ZERO
            && let Some(id) = scroll_target(document, laid_out, pointer.position)
        {
            self.scroll_by(document, laid_out, &id, pointer.scroll);
            events.scroll_consumed = true;
            events.pointer_over_ui = true;
        }

        // --- öppen dropdown tar pekaren först ---------------------------
        if let Some(open_id) = self.open.clone() {
            if pointer.pressed {
                match self.dropdown_option_at(document, laid_out, &open_id, pointer.position) {
                    Some(index) => {
                        if let Some(Kind::Dropdown { selected, .. }) =
                            document.find_mut(&open_id).map(|n| &mut n.kind)
                            && *selected != Some(index)
                        {
                            *selected = Some(index);
                            events.changed.push(open_id.clone());
                        }
                        self.open = None;
                        return events;
                    }
                    None => {
                        // Klick utanför listan stänger den, utan att
                        // också räknas som ett klick på det som låg där.
                        self.open = None;
                        return events;
                    }
                }
            }
            events.pointer_over_ui = true;
        }

        // --- dragning ---------------------------------------------------
        if let Some(active) = self.active.clone() {
            if pointer.down {
                let delta = pointer.position - self.drag_origin;
                self.drag_distance += delta.x.abs() + delta.y.abs();
                if self.drag(document, laid_out, &active, pointer.position, delta) {
                    events.changed.push(active);
                }
                self.drag_origin = pointer.position;
                events.pointer_over_ui = true;
                events.keyboard_captured = self.focused.is_some();
                return events;
            }
            self.active = None;
        }

        // --- nedtryck ---------------------------------------------------
        if pointer.pressed {
            self.pressed = hit.clone();
            self.pressed_double = pointer.double;

            // Ett halvskrivet tal skrivs in när man klickar någon
            // annanstans. Att slänga det hade varit det vanligaste sättet
            // att tappa en inmatning.
            if let Some(previous) = self.focused.clone()
                && hit.as_deref() != Some(previous.as_str())
                && self.commit_number(document, &previous, &mut events)
            {
                // `commit_number` lade redan till händelsen.
            }

            // Fokus flyttas dit man tryckte, eller släpps.
            self.focused = hit.as_ref().and_then(|id| {
                document
                    .find(id)
                    .filter(|node| takes_keyboard(&node.kind))
                    .map(|_| id.clone())
            });
            if let Some(focused) = self.focused.clone() {
                self.caret = document
                    .find(&focused)
                    .and_then(|n| n.kind.text())
                    .map_or(0, |t| t.chars().count());
            }

            if let Some(id) = hit.clone()
                && matches!(
                    document.find(&id).map(|n| &n.kind),
                    Some(Kind::Slider { .. } | Kind::NumberField { .. } | Kind::Divider { .. })
                )
            {
                self.active = Some(id.clone());
                self.drag_origin = pointer.position;
                self.drag_distance = 0.0;
                // Reglaget hoppar dit man tryckte; sifferfältet och
                // avdelaren rör sig bara av själva dragningen, annars
                // hade ett klick slängt värdet någon helt annanstans.
                if matches!(
                    document.find(&id).map(|n| &n.kind),
                    Some(Kind::Slider { .. })
                ) && self.drag_slider(document, laid_out, &id, pointer.position)
                {
                    events.changed.push(id);
                }
            }
        }

        // --- släpp: klick och växlingar ----------------------------------
        if pointer.released {
            let pressed = self.pressed.take();
            if let (Some(pressed), Some(over)) = (pressed, hit.clone())
                && pressed == over
                && !pressed.is_empty()
            {
                if self.pressed_double {
                    events.double_clicked.push(pressed.clone());
                }
                self.activate(document, &pressed, &mut events);
                if matches!(
                    document.find(&pressed).map(|n| &n.kind),
                    Some(Kind::NumberField { .. })
                ) {
                    self.maybe_edit_number(document, &pressed);
                }
            }
        }

        // --- tangenter ---------------------------------------------------
        if let Some(focused) = self.focused.clone() {
            events.keyboard_captured = true;
            if self.type_into(document, &focused, input, &mut events) {
                events.changed.push(focused);
            }
        }

        events
    }

    /// Ett fullbordat klick på en nod: knapp, kryssruta, radio, dropdown.
    fn activate(&mut self, document: &mut Document, id: &str, events: &mut Events) {
        let Some(node) = document.find_mut(id) else {
            return;
        };

        match &mut node.kind {
            Kind::Button { .. } => events.clicked.push(id.to_string()),
            Kind::Checkbox { checked, .. } => {
                *checked = !*checked;
                events.changed.push(id.to_string());
            }
            Kind::Radio { checked, group, .. } => {
                if *checked {
                    // Att klicka på en redan vald radioknapp gör inget.
                    return;
                }
                let group = group.clone();
                *checked = true;
                // Resten av gruppen släcks. Det är radioknappens hela
                // innebörd, och den kan inte uttryckas i en ensam nod.
                clear_group(&mut document.root, &group, id);
                events.changed.push(id.to_string());
            }
            Kind::Dropdown { .. } => {
                self.open = Some(id.to_string());
            }
            Kind::Collapsible { open, .. } => {
                *open = !*open;
                events.changed.push(id.to_string());
            }
            _ => {}
        }
    }

    /// Ett klick på ett sifferfält som inte blev en dragning öppnar det
    /// för skrivning. Gränsen är ett par pixlar: en darrig hand ska inte
    /// betyda att man menade att dra.
    fn maybe_edit_number(&mut self, document: &mut Document, id: &str) {
        if self.drag_distance > 3.0 {
            return;
        }
        let Some(Kind::NumberField {
            value,
            step,
            editing,
            ..
        }) = document.find_mut(id).map(|n| &mut n.kind)
        else {
            return;
        };
        let text = crate::draw::format_number(*value, *step);
        self.caret = text.chars().count();
        *editing = Some(text);
        self.focused = Some(id.to_string());
        self.select_all = true;
    }

    /// Rullar en yta, klamrat till vad innehållet räcker till.
    ///
    /// Utan klampningen kan man rulla en kort lista långt ut i tomrummet
    /// och sedan behöva rulla tillbaka lika långt – innehållet ser ut att
    /// ha försvunnit.
    fn scroll_by(&self, document: &mut Document, laid_out: &LaidOut, id: &str, delta: Vec2) {
        let Some(placed) = laid_out.placed(id) else {
            return;
        };
        let max = Vec2::new(
            (placed.content.x - placed.inner.width).max(0.0),
            (placed.content.y - placed.inner.height).max(0.0),
        );
        let Some(Kind::Scroll { offset }) = document.find_mut(id).map(|n| &mut n.kind) else {
            return;
        };
        offset.x = (offset.x + delta.x).clamp(0.0, max.x);
        offset.y = (offset.y + delta.y).clamp(0.0, max.y);
    }

    /// Fördelar en dragning på den sort noden är.
    fn drag(
        &mut self,
        document: &mut Document,
        laid_out: &LaidOut,
        id: &str,
        position: Vec2,
        delta: Vec2,
    ) -> bool {
        match document.find(id).map(|n| &n.kind) {
            Some(Kind::Slider { .. }) => self.drag_slider(document, laid_out, id, position),
            Some(Kind::NumberField { .. }) => {
                // Bara i sidled. Ett fält som ändrades av lodrät rörelse
                // hade varit omöjligt att träffa exakt.
                drag_number(document, id, delta.x as f64)
            }
            Some(Kind::Divider { vertical, .. }) => {
                let along = if *vertical { delta.x } else { delta.y };
                drag_divider(document, id, along)
            }
            _ => false,
        }
    }

    /// Sätter reglagets värde efter pekarens x-läge.
    fn drag_slider(
        &self,
        document: &mut Document,
        laid_out: &LaidOut,
        id: &str,
        position: Vec2,
    ) -> bool {
        let Some(rect) = laid_out.rect(id) else {
            return false;
        };
        let Some(Kind::Slider {
            value,
            min,
            max,
            step,
        }) = document.find_mut(id).map(|n| &mut n.kind)
        else {
            return false;
        };

        // Greppet är brett, så det åkbara spannet är smalare än spåret.
        let track = (rect.width - crate::metrics::HANDLE).max(1.0);
        let t = ((position.x - rect.x - crate::metrics::HANDLE * 0.5) / track).clamp(0.0, 1.0);
        let mut new = *min + (*max - *min) * t;
        if *step > 0.0 {
            new = (new / *step).round() * *step;
        }
        let new = new.clamp(min.min(*max), max.max(*min));

        if (new - *value).abs() > f32::EPSILON {
            *value = new;
            true
        } else {
            false
        }
    }

    /// Samma som `type_into`, men mot sifferfältets utkasttext.
    ///
    /// Talet skrivs inte om tecken för tecken: `-` och `1.` är inte tal,
    /// och ett fält som vägrade dem gick inte att skriva i. Utkastet
    /// tolkas i stället när man trycker Enter eller lämnar fältet.
    fn type_into_number(
        &mut self,
        document: &mut Document,
        id: &str,
        input: &Input,
        events: &mut Events,
    ) -> bool {
        let Some(Kind::NumberField {
            value,
            step,
            editing,
            ..
        }) = document.find_mut(id).map(|n| &mut n.kind)
        else {
            return false;
        };

        let mut chars: Vec<char> = editing
            .clone()
            .unwrap_or_else(|| crate::draw::format_number(*value, *step))
            .chars()
            .collect();
        let mut caret = self.caret.min(chars.len());
        let mut commit = false;
        let mut abort = false;

        // Att skriva eller radera ersätter markeringen; att flytta
        // markören häver den bara.
        if self.select_all && (!input.text.is_empty() || raderar(&input.keys)) {
            chars.clear();
            caret = 0;
            self.select_all = false;
        } else if !input.keys.is_empty() || !input.text.is_empty() {
            self.select_all = false;
        }

        for key in &input.keys {
            match key {
                Key::Backspace if caret > 0 => {
                    chars.remove(caret - 1);
                    caret -= 1;
                }
                Key::Delete if caret < chars.len() => {
                    chars.remove(caret);
                }
                Key::Left => caret = caret.saturating_sub(1),
                Key::Right => caret = (caret + 1).min(chars.len()),
                Key::Home => caret = 0,
                Key::End => caret = chars.len(),
                Key::Enter | Key::Tab => commit = true,
                Key::Escape => abort = true,
                _ => {}
            }
        }
        for ch in input.text.chars() {
            if ch.is_control() {
                continue;
            }
            chars.insert(caret, ch);
            caret += 1;
        }

        let draft: String = chars.into_iter().collect();
        self.caret = caret;

        if abort {
            *editing = None;
            self.focused = None;
            return false;
        }
        *editing = Some(draft);
        if commit {
            self.focused = None;
            return self.commit_number(document, id, events);
        }
        false
    }

    /// Tolkar utkastet och skriver det som tal.
    ///
    /// Otolkbar text slängs och fältet visar det gamla värdet igen. Att
    /// nolla det i stället hade förvandlat ett tryckfel till förlorad
    /// data.
    fn commit_number(&mut self, document: &mut Document, id: &str, events: &mut Events) -> bool {
        let Some(Kind::NumberField {
            value,
            min,
            max,
            editing,
            ..
        }) = document.find_mut(id).map(|n| &mut n.kind)
        else {
            return false;
        };
        let Some(draft) = editing.take() else {
            return false;
        };
        let Ok(parsed) = draft.trim().replace(',', ".").parse::<f64>() else {
            return false;
        };
        let new = parsed.clamp(*min, *max);
        if (new - *value).abs() <= f64::EPSILON {
            return false;
        }
        *value = new;
        events.changed.push(id.to_string());
        true
    }

    /// Skriver in tecken och hanterar redigeringstangenter.
    fn type_into(
        &mut self,
        document: &mut Document,
        id: &str,
        input: &Input,
        events: &mut Events,
    ) -> bool {
        if matches!(
            document.find(id).map(|n| &n.kind),
            Some(Kind::NumberField { .. })
        ) {
            return self.type_into_number(document, id, input, events);
        }

        let Some(node) = document.find_mut(id) else {
            return false;
        };
        let multiline = matches!(node.kind, Kind::TextArea { .. });
        let text = match &mut node.kind {
            Kind::TextInput { text, .. } | Kind::TextArea { text, .. } => text,
            _ => return false,
        };

        // Arbeta i tecken, inte byte: en å är två byte och skulle annars
        // klyvas mitt itu av backsteg.
        let mut chars: Vec<char> = text.chars().collect();
        let mut caret = self.caret.min(chars.len());
        let mut changed = false;

        for key in &input.keys {
            match key {
                Key::Backspace if caret > 0 => {
                    chars.remove(caret - 1);
                    caret -= 1;
                    changed = true;
                }
                Key::Delete if caret < chars.len() => {
                    chars.remove(caret);
                    changed = true;
                }
                Key::Left => caret = caret.saturating_sub(1),
                Key::Right => caret = (caret + 1).min(chars.len()),
                Key::Home => caret = 0,
                Key::End => caret = chars.len(),
                Key::Enter if multiline => {
                    chars.insert(caret, '\n');
                    caret += 1;
                    changed = true;
                }
                Key::Enter => events.submitted.push(id.to_string()),
                Key::Escape => {
                    self.focused = None;
                }
                _ => {}
            }
        }

        for ch in input.text.chars() {
            // Styrtecken kommer som tangenter, inte som text.
            if ch.is_control() {
                continue;
            }
            chars.insert(caret, ch);
            caret += 1;
            changed = true;
        }

        self.caret = caret;
        if changed {
            *text = chars.into_iter().collect();
        }
        changed
    }

    /// Vilket alternativ i en öppen dropdown pekaren är över.
    fn dropdown_option_at(
        &self,
        document: &Document,
        laid_out: &LaidOut,
        id: &str,
        position: Vec2,
    ) -> Option<usize> {
        let rect = laid_out.rect(id)?;
        let node = document.find(id)?;
        let Kind::Dropdown { options, .. } = &node.kind else {
            return None;
        };
        let row = rect.height.max(1.0);
        for index in 0..options.len() {
            let option = Rect::new(rect.x, rect.bottom() + row * index as f32, rect.width, row);
            if option.contains(position) {
                return Some(index);
            }
        }
        None
    }
}

fn takes_keyboard(kind: &Kind) -> bool {
    // Sifferfältet tar tangenter först när någon klickat i det – fokus
    // sätts av `maybe_edit_number`, inte av nedtrycket. Annars hade en
    // dragning stulit tangentbordet från spelet.
    matches!(kind, Kind::TextInput { .. } | Kind::TextArea { .. })
}

/// Noden under en punkt: den översta träffbara, eller `None`.
///
/// Ritordningen är föräldrar före barn, så den sist besökta träffen är
/// den som ligger överst – samma regel som den som ritar följer.
pub fn hit_test(document: &Document, laid_out: &LaidOut, point: Vec2) -> Option<String> {
    let mut index = 0usize;
    let mut best: Option<(usize, String)> = None;
    visit(&document.root, laid_out, point, &mut index, &mut best);
    best.map(|(_, id)| id)
}

/// Den innersta rullbara ytan under punkten.
///
/// Innersta, inte yttersta: en lista inuti en panel som också rullar ska
/// ta hjulet själv, precis som i varje annat gränssnitt.
fn scroll_target(document: &Document, laid_out: &LaidOut, point: Vec2) -> Option<String> {
    let mut index = 0usize;
    let mut best: Option<(usize, String)> = None;
    find_scroll(&document.root, laid_out, point, &mut index, &mut best);
    best.map(|(_, id)| id)
}

fn find_scroll(
    node: &Node,
    laid_out: &LaidOut,
    point: Vec2,
    index: &mut usize,
    best: &mut Option<(usize, String)>,
) {
    if !node.style.visible {
        return;
    }
    let Some(placed) = laid_out.nodes.get(*index) else {
        return;
    };
    let (rect, clip, depth) = (placed.rect, placed.clip, placed.depth);
    *index += 1;

    if matches!(node.kind, Kind::Scroll { .. })
        && !node.id.is_empty()
        && rect.intersect(clip).contains(point)
        && best.as_ref().is_none_or(|(d, _)| depth >= *d)
    {
        *best = Some((depth, node.id.clone()));
    }

    for child in &node.children {
        find_scroll(child, laid_out, point, index, best);
    }
}

/// Innehåller tangenterna något som raderar?
fn raderar(keys: &[Key]) -> bool {
    keys.iter()
        .any(|key| matches!(key, Key::Backspace | Key::Delete))
}

/// Flyttar ett sifferfält `pixels` steg. Returnerar sant om det ändrades.
fn drag_number(document: &mut Document, id: &str, pixels: f64) -> bool {
    let Some(Kind::NumberField {
        value,
        step,
        min,
        max,
        editing,
    }) = document.find_mut(id).map(|n| &mut n.kind)
    else {
        return false;
    };
    let new = (*value + pixels * *step).clamp(*min, *max);
    if (new - *value).abs() <= f64::EPSILON {
        return false;
    }
    *value = new;
    // Dragningen vinner över en påbörjad inskrivning.
    *editing = None;
    true
}

/// Flyttar en avdelare `pixels` längs sin axel.
fn drag_divider(document: &mut Document, id: &str, pixels: f32) -> bool {
    let Some(Kind::Divider {
        value, min, max, ..
    }) = document.find_mut(id).map(|n| &mut n.kind)
    else {
        return false;
    };
    let new = (*value + pixels).clamp(*min, *max);
    if (new - *value).abs() <= f32::EPSILON {
        return false;
    }
    *value = new;
    true
}

/// Kan noden träffas av pekaren?
///
/// Allt interaktivt kan. En panel kan bara om den har en bakgrund – utan
/// den är den luft, och en helskärmsrot hade annars svalt varje klick i
/// spelet bakom.
fn hittable(node: &Node) -> bool {
    if !node.style.hit_test || node.id.is_empty() {
        return false;
    }
    match node.kind {
        Kind::Button { .. }
        | Kind::Checkbox { .. }
        | Kind::Radio { .. }
        | Kind::Slider { .. }
        | Kind::TextInput { .. }
        | Kind::TextArea { .. }
        | Kind::Dropdown { .. }
        | Kind::Collapsible { .. }
        | Kind::NumberField { .. }
        | Kind::Divider { .. } => true,
        _ => node.style.background.is_some(),
    }
}

fn visit(
    node: &Node,
    laid_out: &LaidOut,
    point: Vec2,
    index: &mut usize,
    best: &mut Option<(usize, String)>,
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

    // Klippet kommer ur layouten, samma värde ritaren använder. Det är
    // avsiktligt: det man ser är det man kan klicka. En rad som rullat ur
    // sin lista ritas inte, och ska då inte heller kunna träffas.
    // En hopfällbar nods träffyta är rubrikraden, som layouten lade i
    // `inner`. Hade hela den utfällda ytan räknats skulle ett klick i
    // tomrummet under innehållet fälla ihop avsnittet igen.
    let own = if matches!(node.kind, Kind::Collapsible { .. }) {
        placed.inner
    } else {
        rect
    };
    let visible_rect = own.intersect(placed.clip);

    if hittable(node) && visible_rect.contains(point) {
        let deeper = best
            .as_ref()
            .is_none_or(|(best_depth, _)| depth >= *best_depth);
        if deeper {
            *best = Some((depth, node.id.clone()));
        }
    }

    for child in &node.children {
        visit(child, laid_out, point, index, best);
    }
}
