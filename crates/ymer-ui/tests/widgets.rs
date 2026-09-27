//! De interaktiva fälten: kryssrutor, radioknappar, reglage, textfält
//! och dropdowns.
//!
//! Som resten av testerna: ingenting härifrån rör Ymer, och mätningen är
//! monospace så att talen går att räkna för hand.

use ymer_ui::prelude::*;
use ymer_ui::{Input, Key};

fn metrics() -> MonospaceMetrics {
    MonospaceMetrics {
        advance: 0.5,
        line_height: 1.0,
    }
}

fn viewport() -> Rect {
    Rect::new(0.0, 0.0, 400.0, 300.0)
}

/// Bygger, lägger ut och returnerar allt som behövs för att klicka.
struct Harness {
    document: Document,
    state: State,
}

impl Harness {
    fn new(root: Node) -> Self {
        Self {
            document: Document::new(root),
            state: State::default(),
        }
    }

    fn laid_out(&self) -> ymer_ui::LaidOut {
        layout(&self.document, viewport(), &metrics())
    }

    /// Ett helt klick: ner och upp på samma punkt.
    fn click(&mut self, at: Vec2) -> Events {
        let laid_out = self.laid_out();
        self.state.update(
            &mut self.document,
            &laid_out,
            Pointer {
                position: at,
                down: true,
                pressed: true,
                ..Default::default()
            },
        );
        self.state.update(
            &mut self.document,
            &laid_out,
            Pointer {
                position: at,
                released: true,
                ..Default::default()
            },
        )
    }

    fn click_on(&mut self, id: &str) -> Events {
        let at = self
            .laid_out()
            .rect(id)
            .unwrap_or_else(|| panic!("{id} placerades inte"))
            .center();
        self.click(at)
    }

    fn type_text(&mut self, text: &str) -> Events {
        let laid_out = self.laid_out();
        self.state.update_with(
            &mut self.document,
            &laid_out,
            &Input {
                text: text.to_string(),
                ..Default::default()
            },
        )
    }

    fn press(&mut self, key: Key) -> Events {
        let laid_out = self.laid_out();
        self.state.update_with(
            &mut self.document,
            &laid_out,
            &Input {
                keys: vec![key],
                ..Default::default()
            },
        )
    }

    fn text_of(&self, id: &str) -> String {
        self.document
            .find(id)
            .and_then(|n| n.kind.text())
            .unwrap_or_default()
            .to_string()
    }

    fn checked(&self, id: &str) -> bool {
        match self.document.find(id).map(|n| &n.kind) {
            Some(Kind::Checkbox { checked, .. }) | Some(Kind::Radio { checked, .. }) => *checked,
            _ => panic!("{id} är ingen kryssruta eller radioknapp"),
        }
    }

    fn slider_value(&self, id: &str) -> f32 {
        match self.document.find(id).map(|n| &n.kind) {
            Some(Kind::Slider { value, .. }) => *value,
            _ => panic!("{id} är inget reglage"),
        }
    }
}

fn kolumn(children: impl IntoIterator<Item = Node>) -> Node {
    Node::panel()
        .with_style(
            Style::column()
                .with_size(Size::Fixed(300.0), Size::Fill)
                .with_gap(4.0),
        )
        .with_children(children)
}

// ------------------------------------------------------------ kryssruta

#[test]
fn kryssruta_vaxlar_och_rapporterar() {
    let mut h = Harness::new(kolumn([Node::checkbox("ljud", "Ljud på", false)]));
    assert!(!h.checked("ljud"));

    let events = h.click_on("ljud");
    assert!(h.checked("ljud"));
    assert!(events.was_changed("ljud"));

    h.click_on("ljud");
    assert!(!h.checked("ljud"), "ett andra klick ska bocka ur");
}

#[test]
fn etiketten_ar_en_del_av_traffytan() {
    // Att bara rutan går att träffa är en klassisk irritation.
    let mut h = Harness::new(kolumn([Node::checkbox("ljud", "Ljud på", false)]));
    let rect = h.laid_out().rect("ljud").unwrap();
    // En bit in i etiketten, klart till höger om rutan.
    h.click(Vec2::new(rect.right() - 4.0, rect.center().y));
    assert!(h.checked("ljud"));
}

// ---------------------------------------------------------- radioknappar

#[test]
fn radio_slacker_sina_syskon() {
    let mut h = Harness::new(kolumn([
        Node::radio("latt", "svarighet", "Lätt", true),
        Node::radio("normal", "svarighet", "Normal", false),
        Node::radio("svar", "svarighet", "Svår", false),
    ]));

    let events = h.click_on("svar");
    assert!(h.checked("svar"));
    assert!(!h.checked("latt"), "den förra ska ha släckts");
    assert!(!h.checked("normal"));
    assert!(events.was_changed("svar"));
}

#[test]
fn radio_ror_inte_en_annan_grupp() {
    let mut h = Harness::new(kolumn([
        Node::radio("latt", "svarighet", "Lätt", true),
        Node::radio("fonster", "lage", "Fönster", true),
    ]));
    h.click_on("fonster");
    assert!(h.checked("latt"), "en annan grupp ska vara orörd");
}

#[test]
fn klick_pa_redan_vald_radio_andrar_inget() {
    let mut h = Harness::new(kolumn([Node::radio("latt", "svarighet", "Lätt", true)]));
    let events = h.click_on("latt");
    assert!(h.checked("latt"));
    assert!(
        !events.was_changed("latt"),
        "inget ändrades, alltså ingen händelse"
    );
}

// --------------------------------------------------------------- reglage

fn reglage_harness() -> Harness {
    Harness::new(kolumn([Node::slider("volym", 0.0, 0.0, 100.0).with_style(
        Style::default().with_size(Size::Fixed(200.0), Size::Fixed(20.0)),
    )]))
}

#[test]
fn reglaget_foljer_pekaren() {
    let mut h = reglage_harness();
    let rect = h.laid_out().rect("volym").unwrap();

    // Klick mitt på spåret ska ge ungefär halva värdet.
    h.click(Vec2::new(rect.center().x, rect.center().y));
    let value = h.slider_value("volym");
    assert!((value - 50.0).abs() < 1.0, "väntade ~50, fick {value}");
}

#[test]
fn reglaget_klampar_i_andarna() {
    let mut h = reglage_harness();
    let rect = h.laid_out().rect("volym").unwrap();

    h.click(Vec2::new(rect.x - 50.0, rect.center().y));
    assert_eq!(h.slider_value("volym"), 0.0);

    // Dra långt förbi högerkanten.
    let laid_out = h.laid_out();
    h.state.update(
        &mut h.document,
        &laid_out,
        Pointer {
            position: Vec2::new(rect.center().x, rect.center().y),
            down: true,
            pressed: true,
            ..Default::default()
        },
    );
    h.state.update(
        &mut h.document,
        &laid_out,
        Pointer {
            position: Vec2::new(rect.right() + 500.0, rect.center().y),
            down: true,
            ..Default::default()
        },
    );
    assert_eq!(h.slider_value("volym"), 100.0);
}

#[test]
fn dragning_haller_kvar_reglaget_utanfor_ytan() {
    // Drar man ut pekaren lodrätt ska reglaget inte släppa – det är vad
    // alla gränssnitt gör, och utan det känns ett reglage trasigt.
    let mut h = reglage_harness();
    let rect = h.laid_out().rect("volym").unwrap();
    let laid_out = h.laid_out();

    h.state.update(
        &mut h.document,
        &laid_out,
        Pointer {
            position: rect.center(),
            down: true,
            pressed: true,
            ..Default::default()
        },
    );
    let events = h.state.update(
        &mut h.document,
        &laid_out,
        Pointer {
            // Långt under reglaget, men fortfarande nedtryckt.
            position: Vec2::new(rect.x + rect.width * 0.25, rect.bottom() + 200.0),
            down: true,
            ..Default::default()
        },
    );
    assert!(events.was_changed("volym"));
    let value = h.slider_value("volym");
    assert!(
        value < 40.0,
        "reglaget skulle ha följt med åt vänster, står på {value}"
    );
}

#[test]
fn steg_avrundar() {
    let mut h = Harness::new(kolumn([Node::slider("niva", 0.0, 0.0, 10.0)
        .with_step(1.0)
        .with_style(Style::default().with_size(Size::Fixed(200.0), Size::Fixed(20.0)))]));
    let rect = h.laid_out().rect("niva").unwrap();
    h.click(Vec2::new(rect.x + rect.width * 0.47, rect.center().y));
    let value = h.slider_value("niva");
    assert_eq!(value, value.round(), "steg 1.0 ska ge heltal, fick {value}");
}

// -------------------------------------------------------------- textfält

fn falt_harness() -> Harness {
    Harness::new(kolumn([Node::text_input("namn", "")
        .with_placeholder("Ditt namn")
        .with_style(
            Style::default()
                .with_size(Size::Fixed(200.0), Size::Fixed(24.0))
                .with_background(Color::rgb(0.1, 0.1, 0.1)),
        )]))
}

#[test]
fn text_skrivs_in_nar_faltet_har_fokus() {
    let mut h = falt_harness();
    // Utan fokus går texten ingenstans.
    h.type_text("hej");
    assert_eq!(h.text_of("namn"), "");

    h.click_on("namn");
    let events = h.type_text("hej");
    assert_eq!(h.text_of("namn"), "hej");
    assert!(events.was_changed("namn"));
    assert!(events.keyboard_captured, "spelet ska inte se tangenterna");
}

#[test]
fn backsteg_klyver_inte_tecken() {
    // "å" är två byte. Räknar man i byte i stället för tecken går den
    // sönder, och strängen blir ogiltig UTF-8 eller tappar fel tecken.
    let mut h = falt_harness();
    h.click_on("namn");
    h.type_text("Jönköping");
    h.press(Key::Backspace);
    assert_eq!(h.text_of("namn"), "Jönköpin");

    for _ in 0..4 {
        h.press(Key::Backspace);
    }
    assert_eq!(h.text_of("namn"), "Jönk");
}

#[test]
fn markoren_flyttas_och_infogar_mitt_i() {
    let mut h = falt_harness();
    h.click_on("namn");
    h.type_text("ab");
    h.press(Key::Left);
    h.type_text("X");
    assert_eq!(h.text_of("namn"), "aXb");

    h.press(Key::Home);
    h.type_text("0");
    assert_eq!(h.text_of("namn"), "0aXb");

    h.press(Key::End);
    h.type_text("9");
    assert_eq!(h.text_of("namn"), "0aXb9");
}

#[test]
fn enter_i_ett_enradigt_falt_skickar_in() {
    let mut h = falt_harness();
    h.click_on("namn");
    h.type_text("klart");
    let events = h.press(Key::Enter);
    assert!(events.was_submitted("namn"));
    assert_eq!(
        h.text_of("namn"),
        "klart",
        "Enter ska inte lägga till något"
    );
}

#[test]
fn fokus_slapps_utanfor() {
    let mut h = falt_harness();
    h.click_on("namn");
    assert_eq!(h.state.focused.as_deref(), Some("namn"));

    h.click(Vec2::new(380.0, 280.0));
    assert!(h.state.focused.is_none());
    h.type_text("x");
    assert_eq!(h.text_of("namn"), "", "utan fokus tar fältet inget");
}

#[test]
fn textarea_tar_radbrytning() {
    let mut h = Harness::new(kolumn([Node::text_area("brev", "", 3).with_style(
        Style::default()
            .with_size(Size::Fixed(200.0), Size::Fixed(60.0))
            .with_background(Color::rgb(0.1, 0.1, 0.1)),
    )]));
    h.click_on("brev");
    h.type_text("rad ett");
    h.press(Key::Enter);
    h.type_text("rad två");
    assert_eq!(h.text_of("brev"), "rad ett\nrad två");
}

// -------------------------------------------------------------- dropdown

fn dropdown_harness() -> Harness {
    Harness::new(kolumn([Node::dropdown(
        "upplosning",
        ["1280x720", "1920x1080", "2560x1440"],
        None,
    )
    .with_style(
        Style::default()
            .with_size(Size::Fixed(180.0), Size::Fixed(24.0))
            .with_background(Color::rgb(0.15, 0.16, 0.2)),
    )]))
}

fn valt(h: &Harness, id: &str) -> Option<usize> {
    match h.document.find(id).map(|n| &n.kind) {
        Some(Kind::Dropdown { selected, .. }) => *selected,
        _ => panic!("{id} är ingen dropdown"),
    }
}

#[test]
fn dropdown_oppnar_och_valjer() {
    let mut h = dropdown_harness();
    assert_eq!(valt(&h, "upplosning"), None);

    h.click_on("upplosning");
    assert_eq!(
        h.state.open.as_deref(),
        Some("upplosning"),
        "klicket ska ha öppnat listan"
    );

    // Andra raden under fältet.
    let rect = h.laid_out().rect("upplosning").unwrap();
    let laid_out = h.laid_out();
    let events = h.state.update(
        &mut h.document,
        &laid_out,
        Pointer {
            position: Vec2::new(rect.center().x, rect.bottom() + rect.height * 1.5),
            down: true,
            pressed: true,
            ..Default::default()
        },
    );

    assert_eq!(valt(&h, "upplosning"), Some(1));
    assert!(events.was_changed("upplosning"));
    assert!(h.state.open.is_none(), "listan ska ha stängts");
    assert_eq!(h.text_of("upplosning"), "1920x1080");
}

#[test]
fn klick_utanfor_stanger_utan_att_valja() {
    let mut h = dropdown_harness();
    h.click_on("upplosning");

    let laid_out = h.laid_out();
    h.state.update(
        &mut h.document,
        &laid_out,
        Pointer {
            position: Vec2::new(350.0, 290.0),
            down: true,
            pressed: true,
            ..Default::default()
        },
    );
    assert!(h.state.open.is_none());
    assert_eq!(valt(&h, "upplosning"), None);
}

#[test]
fn oppen_lista_tar_pekaren_fran_spelet() {
    let mut h = dropdown_harness();
    h.click_on("upplosning");

    let laid_out = h.laid_out();
    let events = h.state.update(
        &mut h.document,
        &laid_out,
        Pointer {
            position: Vec2::new(350.0, 290.0),
            ..Default::default()
        },
    );
    assert!(
        events.pointer_over_ui,
        "med en öppen lista ska spelet bakom ligga still"
    );
}

#[test]
fn oppen_lista_ritas_ovanpa() {
    let mut h = dropdown_harness();
    h.click_on("upplosning");

    let laid_out = h.laid_out();
    let list = draw(&h.document, &laid_out, &h.state);
    let texter: Vec<&str> = list
        .commands
        .iter()
        .filter_map(|c| match c {
            Command::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert!(
        texter.contains(&"2560x1440"),
        "alla alternativ ska ritas, fick {texter:?}"
    );
}

// ------------------------------------------------------------ som data

#[test]
fn alla_falt_overlever_ron() {
    let document = Document::new(kolumn([
        Node::checkbox("ljud", "Ljud", true),
        Node::radio("latt", "grupp", "Lätt", true),
        Node::slider("volym", 42.0, 0.0, 100.0).with_step(1.0),
        Node::text_input("namn", "Johannes").with_placeholder("Namn"),
        Node::text_area("brev", "rad\nrad", 3),
        Node::dropdown("val", ["a", "b"], Some(1)),
    ]));

    let text = ron::ser::to_string_pretty(&document, ron::ser::PrettyConfig::default()).unwrap();
    let tillbaka: Document = ron::from_str(&text).unwrap();
    assert_eq!(
        document, tillbaka,
        "värdena ska överleva en vända till fil – det är dem editorn sätter"
    );
}

#[test]
fn varden_gar_att_lasa_och_skriva_pa_id() {
    // Vägen TypeScript tar: peka på id, läs eller skriv.
    let mut document = Document::new(kolumn([
        Node::text_input("namn", "gammalt"),
        Node::dropdown("val", ["a", "b"], Some(0)),
    ]));

    assert!(document.set_text("namn", "nytt"));
    assert_eq!(document.find("namn").unwrap().kind.text(), Some("nytt"));
    assert_eq!(document.find("val").unwrap().kind.text(), Some("a"));
}

#[test]
fn set_value_klamras_till_reglagets_spann() {
    // Ett skript som skriver ett reglage ska inte kunna lämna greppet
    // utanför spannet. Tidigare träffade set_value bara Bar, så varje
    // ui.setValue mot ett reglage föll tyst bort.
    let mut document = Document::new(kolumn([
        Node::slider("volym", 50.0, 0.0, 100.0),
        Node::bar(0.5, Color::rgb(1.0, 1.0, 1.0)).with_id("liv"),
    ]));

    assert!(document.set_value("volym", 250.0));
    assert_eq!(document.value("volym"), Some(NodeValue::Number(100.0)));

    assert!(document.set_value("volym", -1.0));
    assert_eq!(document.value("volym"), Some(NodeValue::Number(0.0)));

    // Mätaren är normaliserad, inte ett spann.
    assert!(document.set_value("liv", 2.0));
    assert_eq!(document.value("liv"), Some(NodeValue::Number(1.0)));

    assert!(
        !document.set_value("saknas", 1.0),
        "okänt id ska rapportera fel"
    );
}
