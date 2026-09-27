//! Gränssnittet som träd.
//!
//! Ett `Document` är hela UI:t. Det serialiseras till en fil, laddas av
//! spelet, redigeras i editorn och ändras av skript – samma data hela
//! vägen. Det är därför biblioteket är retained mode och inte immediate:
//! ett träd går att spara och peka på, en funktion gör det inte.

use serde::{Deserialize, Serialize};

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
}

impl Kind {
    /// Texten noden visar, om någon. Det här är vad TypeScript ändrar
    /// oftast – en HUD är mest siffror som byts varje frame.
    pub fn text(&self) -> Option<&str> {
        match self {
            Kind::Label { text } | Kind::Button { text } => Some(text),
            _ => None,
        }
    }

    pub fn set_text(&mut self, new: impl Into<String>) -> bool {
        match self {
            Kind::Label { text } | Kind::Button { text } => {
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

    pub fn bar(value: f32, fill: crate::geom::Color) -> Self {
        Self::new(Kind::Bar { value, fill })
    }

    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
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

    /// Värdet på en `Bar`.
    pub fn set_value(&mut self, id: &str, value: f32) -> bool {
        match self.find_mut(id).map(|node| &mut node.kind) {
            Some(Kind::Bar { value: slot, .. }) => {
                *slot = value.clamp(0.0, 1.0);
                true
            }
            _ => false,
        }
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
