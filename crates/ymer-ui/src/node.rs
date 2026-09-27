//! Gränssnittet som träd.
//!
//! Ett `Document` är hela UI:t. Det serialiseras till en fil, laddas av
//! spelet, redigeras i editorn och ändras av skript – samma data hela
//! vägen. Det är därför biblioteket är retained mode och inte immediate:
//! ett träd går att spara och peka på, en funktion gör det inte.

use serde::{Deserialize, Serialize};

use crate::geom::Vec2;
use crate::style::Style;

/// Vad noden faktiskt är.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Kind {
    /// Bara en yta med bakgrund och barn.
    Panel,
    Label {
        text: String,
    },
    Button {
        text: String,
    },
    /// Namnet på en textur; vad det betyder avgör den som ritar.
    Image {
        source: String,
    },
    /// Tom yta som tar plats. Med `Size::Fill` trycker den isär syskon.
    Spacer,
    /// Vågrät eller lodrät stapel som visar ett värde i 0..1.
    Bar {
        value: f32,
        fill: crate::geom::Color,
    },

    // --- interaktiva fält ---------------------------------------------
    //
    // Värdet bor i noden, inte vid sidan om. Det är hela poängen med ett
    // retained-läge: editorn sätter ett startvärde, spelaren ändrar det,
    // och TypeScript läser samma fält – utan att någon behöver hålla en
    // parallell kopia i synk.
    Checkbox {
        label: String,
        checked: bool,
    },
    /// Flera med samma `group` hör ihop; bara en kan vara vald.
    Radio {
        label: String,
        group: String,
        checked: bool,
    },
    Slider {
        value: f32,
        min: f32,
        max: f32,
        /// Avrundning. 0 betyder steglöst.
        step: f32,
    },
    /// Enradigt textfält.
    TextInput {
        text: String,
        /// Visas grått när fältet är tomt.
        placeholder: String,
    },
    /// Flerradigt fält. `rows` styr bara höjden när `Size::Auto` används.
    TextArea {
        text: String,
        rows: u32,
    },
    /// Ett fönster in i ett större innehåll.
    ///
    /// Förskjutningen bor i noden, som alla andra värden: en editor som
    /// bygger om sitt dokument varje frame ska inte tappa var listan var
    /// rullad. Barnen läggs som en kolumn och klipps till nodens inneryta.
    Scroll {
        offset: Vec2,
    },
    Dropdown {
        options: Vec<String>,
        /// Index i `options`, eller None när inget är valt.
        selected: Option<usize>,
        placeholder: String,
    },
}

impl Kind {
    /// Texten noden visar, om någon. Det här är vad TypeScript ändrar
    /// oftast – en HUD är mest siffror som byts varje frame.
    pub fn text(&self) -> Option<&str> {
        match self {
            Kind::Label { text }
            | Kind::Button { text }
            | Kind::TextInput { text, .. }
            | Kind::TextArea { text, .. } => Some(text),
            Kind::Checkbox { label, .. } | Kind::Radio { label, .. } => Some(label),
            Kind::Dropdown {
                options, selected, ..
            } => selected.and_then(|i| options.get(i)).map(String::as_str),
            _ => None,
        }
    }

    pub fn set_text(&mut self, new: impl Into<String>) -> bool {
        match self {
            Kind::Label { text }
            | Kind::Button { text }
            | Kind::TextInput { text, .. }
            | Kind::TextArea { text, .. } => {
                *text = new.into();
                true
            }
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Node {
    /// Stabilt namn. Editorn visar det, skript slår upp på det, och
    /// klickhändelser rapporteras med det. Tomt id går bra för noder
    /// ingen behöver peka på.
    #[serde(default)]
    pub id: String,
    pub kind: Kind,
    #[serde(default)]
    pub style: Style,
    #[serde(default)]
    pub children: Vec<Node>,
}

impl Node {
    pub fn new(kind: Kind) -> Self {
        Self {
            id: String::new(),
            kind,
            style: Style::default(),
            children: Vec::new(),
        }
    }

    pub fn panel() -> Self {
        Self::new(Kind::Panel)
    }

    pub fn label(text: impl Into<String>) -> Self {
        Self::new(Kind::Label { text: text.into() })
    }

    pub fn button(id: impl Into<String>, text: impl Into<String>) -> Self {
        Self::new(Kind::Button { text: text.into() }).with_id(id)
    }

    pub fn spacer() -> Self {
        Self::new(Kind::Spacer)
    }

    pub fn image(source: impl Into<String>) -> Self {
        Self::new(Kind::Image {
            source: source.into(),
        })
    }

    pub fn checkbox(id: impl Into<String>, label: impl Into<String>, checked: bool) -> Self {
        Self::new(Kind::Checkbox {
            label: label.into(),
            checked,
        })
        .with_id(id)
    }

    pub fn radio(
        id: impl Into<String>,
        group: impl Into<String>,
        label: impl Into<String>,
        checked: bool,
    ) -> Self {
        Self::new(Kind::Radio {
            label: label.into(),
            group: group.into(),
            checked,
        })
        .with_id(id)
    }

    pub fn slider(id: impl Into<String>, value: f32, min: f32, max: f32) -> Self {
        Self::new(Kind::Slider {
            value,
            min,
            max,
            step: 0.0,
        })
        .with_id(id)
    }

    pub fn text_input(id: impl Into<String>, text: impl Into<String>) -> Self {
        Self::new(Kind::TextInput {
            text: text.into(),
            placeholder: String::new(),
        })
        .with_id(id)
    }

    pub fn text_area(id: impl Into<String>, text: impl Into<String>, rows: u32) -> Self {
        Self::new(Kind::TextArea {
            text: text.into(),
            rows: rows.max(1),
        })
        .with_id(id)
    }

    /// En rullbar yta. Den *måste* få en storlek – dess egenstorlek är
    /// noll, eftersom en yta som mäter sig efter sitt innehåll aldrig
    /// behöver rullas.
    pub fn scroll(id: impl Into<String>) -> Self {
        Self::new(Kind::Scroll { offset: Vec2::ZERO })
            .with_id(id)
            .with_style(crate::style::Style::column().with_clip(true))
    }

    pub fn dropdown(
        id: impl Into<String>,
        options: impl IntoIterator<Item = impl Into<String>>,
        selected: Option<usize>,
    ) -> Self {
        Self::new(Kind::Dropdown {
            options: options.into_iter().map(Into::into).collect(),
            selected,
            placeholder: "Välj…".to_string(),
        })
        .with_id(id)
    }

    pub fn bar(value: f32, fill: crate::geom::Color) -> Self {
        Self::new(Kind::Bar { value, fill })
    }

    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Platshållartexten för ett fält eller en dropdown.
    pub fn with_placeholder(mut self, text: impl Into<String>) -> Self {
        match &mut self.kind {
            Kind::TextInput { placeholder, .. } | Kind::Dropdown { placeholder, .. } => {
                *placeholder = text.into();
            }
            _ => {}
        }
        self
    }

    /// Stegar ett reglage. 0 är steglöst.
    pub fn with_step(mut self, new_step: f32) -> Self {
        if let Kind::Slider { step, .. } = &mut self.kind {
            *step = new_step;
        }
        self
    }

    pub fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn with_child(mut self, child: Node) -> Self {
        self.children.push(child);
        self
    }

    pub fn with_children(mut self, children: impl IntoIterator<Item = Node>) -> Self {
        self.children.extend(children);
        self
    }

    /// Noden med ett visst id, var som helst i trädet.
    pub fn find(&self, id: &str) -> Option<&Node> {
        if self.id == id {
            return Some(self);
        }
        self.children.iter().find_map(|child| child.find(id))
    }

    pub fn find_mut(&mut self, id: &str) -> Option<&mut Node> {
        if self.id == id {
            return Some(self);
        }
        self.children
            .iter_mut()
            .find_map(|child| child.find_mut(id))
    }

    /// Nodens värde, om den bär något.
    pub fn value(&self) -> Option<NodeValue> {
        match &self.kind {
            Kind::TextInput { text, .. } | Kind::TextArea { text, .. } => {
                Some(NodeValue::Text(text.clone()))
            }
            Kind::Checkbox { checked, .. } | Kind::Radio { checked, .. } => {
                Some(NodeValue::Bool(*checked))
            }
            Kind::Slider { value, .. } => Some(NodeValue::Number(*value)),
            Kind::Bar { value, .. } => Some(NodeValue::Number(*value)),
            Kind::Dropdown { selected, .. } => Some(NodeValue::Index(*selected)),
            // Etiketter och knappar har text men inget *värde* – de är
            // utdata, inte inmatning.
            _ => None,
        }
    }

    /// Alla noder i trädet, föräldrar före barn.
    pub fn walk(&self) -> impl Iterator<Item = &Node> {
        let mut stack = vec![self];
        std::iter::from_fn(move || {
            let node = stack.pop()?;
            // Bakifrån, så att barnen kommer i ordning.
            stack.extend(node.children.iter().rev());
            Some(node)
        })
    }
}

/// Släcker alla radioknappar i gruppen utom den valda.
///
/// Bor här och inte hos interaktionen, för både ett klick och ett skript
/// som sätter värdet måste följa samma regel: en grupp har en vald.
pub(crate) fn clear_group(node: &mut Node, group: &str, keep: &str) {
    if let Kind::Radio {
        group: g, checked, ..
    } = &mut node.kind
        && g == group
        && node.id != keep
    {
        *checked = false;
    }
    for child in &mut node.children {
        clear_group(child, group, keep);
    }
}

/// Värdet på en nod, i den form den som frågar vill ha det.
///
/// Finns för att en läsare – ett skript, en editor, ett test – ska kunna
/// hämta *vad noden är värd* utan att först räkna ut vilken sorts nod det
/// är. Vilken variant man får avgörs av noden, inte av frågan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NodeValue {
    Text(String),
    Bool(bool),
    Number(f32),
    /// Valt alternativ i en dropdown.
    Index(Option<usize>),
}

/// Hela gränssnittet, som det sparas på fil.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Document {
    pub root: Node,
}

impl Document {
    pub fn new(root: Node) -> Self {
        Self { root }
    }

    pub fn find(&self, id: &str) -> Option<&Node> {
        self.root.find(id)
    }

    pub fn find_mut(&mut self, id: &str) -> Option<&mut Node> {
        self.root.find_mut(id)
    }

    /// Sätter texten på en nod. Returnerar false om noden saknas eller
    /// inte är av en sort som bär text – det är den vanligaste vägen in
    /// från skript, och ett tyst misslyckande vore svårt att felsöka.
    pub fn set_text(&mut self, id: &str, text: impl Into<String>) -> bool {
        self.find_mut(id)
            .is_some_and(|node| node.kind.set_text(text))
    }

    pub fn set_visible(&mut self, id: &str, visible: bool) -> bool {
        match self.find_mut(id) {
            Some(node) => {
                node.style.visible = visible;
                true
            }
            None => false,
        }
    }

    /// Värdet på ett reglage eller en mätare.
    ///
    /// Reglaget klamras till sitt eget spann och kvantiseras till sitt
    /// steg, precis som en dragning gör. Annars kunde ett skript lämna
    /// greppet på en position spelaren själv inte kan nå, och nästa
    /// dragning hade hoppat.
    pub fn set_value(&mut self, id: &str, value: f32) -> bool {
        match self.find_mut(id).map(|node| &mut node.kind) {
            Some(Kind::Slider {
                value: slot,
                min,
                max,
                step,
            }) => {
                let mut new = value;
                if *step > 0.0 {
                    new = (new / *step).round() * *step;
                }
                *slot = new.clamp(min.min(*max), max.max(*min));
                true
            }
            // Mätaren är normaliserad, inte ett spann.
            Some(Kind::Bar { value: slot, .. }) => {
                *slot = value.clamp(0.0, 1.0);
                true
            }
            _ => false,
        }
    }

    /// Bockar i eller ur en kryssruta eller radioknapp.
    ///
    /// En radioknapp som bockas i släcker resten av sin grupp, precis som
    /// ett klick hade gjort – annars kunde ett skript lämna två valda.
    pub fn set_checked(&mut self, id: &str, value: bool) -> bool {
        let group = match self.find_mut(id).map(|node| &mut node.kind) {
            Some(Kind::Checkbox { checked, .. }) => {
                *checked = value;
                None
            }
            Some(Kind::Radio { checked, group, .. }) => {
                *checked = value;
                value.then(|| group.clone())
            }
            _ => return false,
        };
        if let Some(group) = group {
            clear_group(&mut self.root, &group, id);
        }
        true
    }

    /// Väljer ett alternativ i en dropdown.
    pub fn set_selected(&mut self, id: &str, index: Option<usize>) -> bool {
        match self.find_mut(id).map(|node| &mut node.kind) {
            Some(Kind::Dropdown {
                options, selected, ..
            }) => {
                // Ett index utanför listan vore osynligt fel; hellre nej.
                if index.is_some_and(|i| i >= options.len()) {
                    return false;
                }
                *selected = index;
                true
            }
            _ => false,
        }
    }

    /// Vad noden är värd, för den som vill läsa utan att veta sorten.
    pub fn value(&self, id: &str) -> Option<NodeValue> {
        self.find(id).and_then(|node| node.value())
    }

    /// Alla namngivna noder som bär ett värde.
    ///
    /// Det här är vad ett skript får skickat till sig varje frame: en
    /// platt lista, inte hela trädet. Ett gränssnitt kan ha hundratals
    /// noder och en handfull värden.
    pub fn values(&self) -> Vec<(&str, NodeValue)> {
        self.root
            .walk()
            .filter(|node| !node.id.is_empty())
            .filter_map(|node| node.value().map(|value| (node.id.as_str(), value)))
            .collect()
    }

    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.root
            .walk()
            .map(|node| node.id.as_str())
            .filter(|id| !id.is_empty())
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new(Node::panel().with_style(crate::style::Style {
            width: crate::style::Size::Fill,
            height: crate::style::Size::Fill,
            layout: crate::style::Layout::Stack,
            ..Default::default()
        }))
    }
}
