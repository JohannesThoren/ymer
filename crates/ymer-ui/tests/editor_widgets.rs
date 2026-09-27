//! Widgetarna en editor inte klarar sig utan.
//!
//! Hopfällbara rubriker, sifferfält man drar eller skriver i, och
//! dragbara avdelare. Alla tre har samma fälla: de ser rätt ut i en
//! stillbild och är fel i rörelse. Testerna kör därför riktiga
//! pekarsekvenser – ned, flytta, upp – i stället för att kalla på
//! interna funktioner.

use ymer_ui::prelude::*;
use ymer_ui::{Input, Key, LaidOut, State};

fn metrics() -> MonospaceMetrics {
    MonospaceMetrics {
        advance: 0.5,
        line_height: 1.0,
    }
}

fn viewport() -> Rect {
    Rect::new(0.0, 0.0, 400.0, 400.0)
}

struct Prov {
    document: Document,
    state: State,
}

impl Prov {
    fn new(document: Document) -> Self {
        Self {
            document,
            state: State::default(),
        }
    }

    fn layout(&self) -> LaidOut {
        layout(&self.document, viewport(), &metrics())
    }

    fn mata(&mut self, input: Input) -> Events {
        let laid_out = self.layout();
        self.state
            .update_with(&mut self.document, &laid_out, &input)
    }

    fn peka(&mut self, pointer: Pointer) -> Events {
        self.mata(Input::from_pointer(pointer))
    }

    fn mitten(&self, id: &str) -> Vec2 {
        self.layout()
            .rect(id)
            .unwrap_or_else(|| panic!("{id} finns"))
            .center()
    }

    /// Rubrikraden i ett hopfällbart avsnitt – den enda delen av det som
    /// kan klickas. Layouten lägger den i `inner`.
    fn rubrik(&self, id: &str) -> Vec2 {
        self.layout()
            .placed(id)
            .unwrap_or_else(|| panic!("{id} finns"))
            .inner
            .center()
    }

    fn klicka_rubrik(&mut self, id: &str) -> Events {
        let punkt = self.rubrik(id);
        self.peka(Pointer {
            position: punkt,
            down: true,
            pressed: true,
            ..Default::default()
        });
        self.peka(Pointer {
            position: punkt,
            released: true,
            ..Default::default()
        })
    }

    /// Ned och upp på samma ställe: ett klick, ingen dragning.
    fn klicka(&mut self, id: &str) -> Events {
        let punkt = self.mitten(id);
        self.peka(Pointer {
            position: punkt,
            down: true,
            pressed: true,
            ..Default::default()
        });
        self.peka(Pointer {
            position: punkt,
            released: true,
            ..Default::default()
        })
    }

    /// Ned, flytta i steg, upp.
    fn dra(&mut self, id: &str, delta: Vec2) -> Events {
        let start = self.mitten(id);
        self.peka(Pointer {
            position: start,
            down: true,
            pressed: true,
            ..Default::default()
        });
        // I två steg, för att också pröva att rörelsen ackumuleras.
        let mut senast = Events::default();
        for steg in 1..=2 {
            senast = self.peka(Pointer {
                position: start + delta * (steg as f32 / 2.0),
                down: true,
                ..Default::default()
            });
        }
        self.peka(Pointer {
            position: start + delta,
            released: true,
            ..Default::default()
        });
        senast
    }

    fn skriv(&mut self, text: &str, keys: &[Key]) -> Events {
        self.mata(Input {
            text: text.to_string(),
            keys: keys.to_vec(),
            ..Default::default()
        })
    }

    fn tal(&self, id: &str) -> f64 {
        match self.document.value(id) {
            Some(NodeValue::Number(n)) => n as f64,
            annat => panic!("{id} bär inget tal: {annat:?}"),
        }
    }

    fn oppen(&self, id: &str) -> bool {
        matches!(self.document.value(id), Some(NodeValue::Bool(true)))
    }

    fn syns(&self, id: &str) -> bool {
        self.layout().rect(id).is_some()
    }
}

// ------------------------------------------------------- hopfällbart

fn avsnitt() -> Document {
    Document::new(
        Node::panel()
            .with_id("rot")
            .with_style(Style::column().with_size(Size::Fixed(300.0), Size::Fill))
            .with_children([
                Node::collapsible("transform", "Transform", true)
                    .with_style(
                        Style::column()
                            .with_size(Size::Fill, Size::Auto)
                            .with_gap(4.0),
                    )
                    .with_children([Node::label("x").with_id("x"), Node::label("y").with_id("y")]),
                Node::label("efter").with_id("efter"),
            ]),
    )
}

#[test]
fn hopfallt_avsnitt_tar_ingen_plats() {
    let mut prov = Prov::new(avsnitt());

    let utfalld = prov.layout().rect("transform").unwrap().height;
    assert!(prov.syns("x") && prov.syns("y"));
    let efter_utfalld = prov.layout().rect("efter").unwrap().y;

    prov.klicka_rubrik("transform");
    assert!(!prov.oppen("transform"));

    assert!(
        !prov.syns("x"),
        "barnen ska inte finnas i layouten alls, inte bara vara osynliga"
    );
    let hopfalld = prov.layout().rect("transform").unwrap().height;
    assert!(
        hopfalld < utfalld,
        "rubriken ska krympa till sin egen rad: {hopfalld} mot {utfalld}"
    );
    assert!(
        prov.layout().rect("efter").unwrap().y < efter_utfalld,
        "det som ligger under ska flytta upp"
    );
}

#[test]
fn ett_klick_i_innehallet_faller_inte_ihop_avsnittet() {
    // Rubriken kan klickas, inte hela det utfällda avsnittet. Annars
    // stänger varje klick på ett fält sektionen det ligger i.
    let mut prov = Prov::new(avsnitt());
    let punkt = prov.mitten("y");

    prov.peka(Pointer {
        position: punkt,
        down: true,
        pressed: true,
        ..Default::default()
    });
    prov.peka(Pointer {
        position: punkt,
        released: true,
        ..Default::default()
    });

    assert!(prov.oppen("transform"), "avsnittet ska stå kvar öppet");
}

#[test]
fn rubriken_rapporterar_att_den_vaxlade() {
    let mut prov = Prov::new(avsnitt());
    let events = prov.klicka_rubrik("transform");
    assert!(events.changed.contains(&"transform".to_string()));
}

#[test]
fn ett_avsnitt_gar_att_falla_fran_kod() {
    // Editorn ska kunna minnas vilka avsnitt som var öppna.
    let mut document = avsnitt();
    assert!(document.set_checked("transform", false));
    assert_eq!(document.value("transform"), Some(NodeValue::Bool(false)));
    assert!(
        layout(&document, viewport(), &metrics())
            .rect("x")
            .is_none(),
        "innehållet ska vara borta ur layouten"
    );
}

// ---------------------------------------------------------- sifferfält

fn falt() -> Document {
    Document::new(
        Node::panel()
            .with_id("rot")
            .with_style(
                Style::row()
                    .with_size(Size::Fill, Size::Fixed(30.0))
                    .with_gap(4.0),
            )
            .with_children([
                Node::number("x", 1.5, 0.05),
                Node::number_in("andel", 50.0, 1.0, 0.0, 100.0),
            ]),
    )
}

#[test]
fn draget_andrar_talet_i_steg() {
    let mut prov = Prov::new(falt());
    assert_eq!(prov.tal("x"), 1.5);

    // 20 px åt höger, steg 0.05: +1.0.
    let events = prov.dra("x", Vec2::new(20.0, 0.0));
    assert!(events.changed.contains(&"x".to_string()));
    assert!((prov.tal("x") - 2.5).abs() < 1e-6, "{}", prov.tal("x"));

    prov.dra("x", Vec2::new(-40.0, 0.0));
    assert!((prov.tal("x") - 0.5).abs() < 1e-6, "{}", prov.tal("x"));
}

#[test]
fn lodratt_drag_lamnar_talet_ifred() {
    // Ett fält som ändrades av lodrät rörelse går inte att träffa exakt.
    let mut prov = Prov::new(falt());
    prov.dra("x", Vec2::new(0.0, 60.0));
    assert_eq!(prov.tal("x"), 1.5);
}

#[test]
fn spannet_haller() {
    let mut prov = Prov::new(falt());
    prov.dra("andel", Vec2::new(500.0, 0.0));
    assert_eq!(prov.tal("andel"), 100.0);
    prov.dra("andel", Vec2::new(-5000.0, 0.0));
    assert_eq!(prov.tal("andel"), 0.0);
}

#[test]
fn ett_klick_oppnar_faltet_for_skrivning() {
    let mut prov = Prov::new(falt());
    prov.klicka("x");

    prov.skriv("12.25", &[]);
    assert_eq!(
        prov.tal("x"),
        1.5,
        "talet ska inte ändras medan man skriver"
    );

    let events = prov.skriv("", &[Key::Enter]);
    assert_eq!(prov.tal("x"), 12.25);
    assert!(events.changed.contains(&"x".to_string()));
}

#[test]
fn halvskrivna_tal_far_stå_kvar_i_fältet() {
    // "-" och "1." är inte tal. Ett fält som vägrade dem gick inte att
    // skriva i: man kommer aldrig förbi första tecknet.
    let mut prov = Prov::new(falt());
    prov.klicka("x");
    prov.skriv("-", &[]);
    prov.skriv("3", &[]);
    prov.skriv(".", &[]);
    prov.skriv("5", &[]);
    prov.skriv("", &[Key::Enter]);
    assert_eq!(prov.tal("x"), -3.5);
}

#[test]
fn otolkbar_text_lamnar_det_gamla_talet() {
    let mut prov = Prov::new(falt());
    prov.klicka("x");
    prov.skriv("hej", &[]);
    prov.skriv("", &[Key::Enter]);
    assert_eq!(
        prov.tal("x"),
        1.5,
        "ett tryckfel ska inte förvandlas till förlorad data"
    );
}

#[test]
fn escape_slanger_inskrivningen() {
    let mut prov = Prov::new(falt());
    prov.klicka("x");
    prov.skriv("99", &[]);
    prov.skriv("", &[Key::Escape]);
    assert_eq!(prov.tal("x"), 1.5);
}

#[test]
fn ett_klick_nagon_annanstans_skriver_in_talet() {
    let mut prov = Prov::new(falt());
    prov.klicka("x");
    prov.skriv("7", &[]);

    // Klicka i det andra fältet. Inskrivningen sker vid *nedtrycket*,
    // alltså i samma frame som fokus flyttas – inte när knappen släpps.
    let punkt = prov.mitten("andel");
    let events = prov.peka(Pointer {
        position: punkt,
        down: true,
        pressed: true,
        ..Default::default()
    });
    assert_eq!(prov.tal("x"), 7.0, "det påbörjade talet ska skrivas in");
    assert!(events.changed.contains(&"x".to_string()));
}

#[test]
fn ett_drag_efter_ett_klick_skriver_inte() {
    // En dragning ska inte också öppna fältet för skrivning bara för att
    // pekaren släpptes över det.
    let mut prov = Prov::new(falt());
    prov.dra("x", Vec2::new(20.0, 0.0));
    let events = prov.skriv("9", &[]);
    assert!(
        !events.keyboard_captured,
        "fältet ska inte ha tangentbordet efter ett drag"
    );
    assert!((prov.tal("x") - 2.5).abs() < 1e-6);
}

#[test]
fn ett_tal_gar_att_satta_fran_kod() {
    let mut document = falt();
    assert!(document.set_value("x", 4.0));
    assert_eq!(document.value("x"), Some(NodeValue::Number(4.0)));
    assert!(document.set_value("andel", 500.0));
    assert_eq!(
        document.value("andel"),
        Some(NodeValue::Number(100.0)),
        "spannet gäller även utifrån"
    );
}

// ----------------------------------------------------------- avdelare

fn delad() -> Document {
    Document::new(
        Node::panel()
            .with_id("rot")
            .with_style(Style::row().with_size(Size::Fill, Size::Fill))
            .with_children([
                Node::panel().with_id("vanster").with_style(
                    Style::column()
                        .with_size(Size::Fixed(120.0), Size::Fill)
                        .with_background(Color::rgb(0.2, 0.2, 0.2)),
                ),
                Node::divider("delare", 120.0, 80.0, 300.0)
                    .with_style(Style::default().with_size(Size::Auto, Size::Fill)),
                Node::panel().with_id("hoger").with_style(
                    Style::column()
                        .with_size(Size::Fill, Size::Fill)
                        .with_background(Color::rgb(0.3, 0.3, 0.3)),
                ),
            ]),
    )
}

#[test]
fn avdelaren_foljer_pekaren_en_pixel_i_taget() {
    let mut prov = Prov::new(delad());
    assert_eq!(prov.tal("delare"), 120.0);

    prov.dra("delare", Vec2::new(40.0, 0.0));
    assert_eq!(prov.tal("delare"), 160.0, "en pixel dragning är en pixel");

    prov.dra("delare", Vec2::new(-30.0, 0.0));
    assert_eq!(prov.tal("delare"), 130.0);
}

#[test]
fn avdelaren_stannar_i_sitt_spann() {
    let mut prov = Prov::new(delad());
    prov.dra("delare", Vec2::new(1000.0, 0.0));
    assert_eq!(prov.tal("delare"), 300.0);
    prov.dra("delare", Vec2::new(-1000.0, 0.0));
    assert_eq!(prov.tal("delare"), 80.0);
}

#[test]
fn ett_klick_pa_avdelaren_flyttar_den_inte() {
    // Till skillnad från ett reglage, där greppet hoppar dit man tryckte.
    // En panel som bytte bredd av ett oavsiktligt klick vore obrukbar.
    let mut prov = Prov::new(delad());
    prov.klicka("delare");
    assert_eq!(prov.tal("delare"), 120.0);
}

#[test]
fn en_vagrat_avdelare_dras_i_hojdled() {
    let mut prov = Prov::new(Document::new(
        Node::panel()
            .with_id("rot")
            .with_style(Style::column().with_size(Size::Fill, Size::Fill))
            .with_child(
                Node::divider_horizontal("delare", 100.0, 40.0, 200.0)
                    .with_style(Style::default().with_size(Size::Fill, Size::Auto)),
            ),
    ));

    prov.dra("delare", Vec2::new(0.0, 25.0));
    assert_eq!(prov.tal("delare"), 125.0);
    prov.dra("delare", Vec2::new(60.0, 0.0));
    assert_eq!(prov.tal("delare"), 125.0, "sidled rör den inte");
}

#[test]
fn allt_overlever_en_vanda_till_fil() {
    // Editorn sparar dokument. Ett halvskrivet tal ska däremot inte följa
    // med – det är ingen data.
    let mut document = Document::new(Node::panel().with_id("rot").with_children([
        Node::collapsible("avsnitt", "Transform", false),
        Node::number_in("x", 2.5, 0.05, -10.0, 10.0),
        Node::divider("delare", 140.0, 80.0, 300.0),
    ]));

    let mut state = State::default();
    let laid_out = layout(&document, viewport(), &metrics());
    let punkt = laid_out.rect("x").unwrap().center();
    state.update(
        &mut document,
        &laid_out,
        Pointer {
            position: punkt,
            down: true,
            pressed: true,
            ..Default::default()
        },
    );
    state.update(
        &mut document,
        &laid_out,
        Pointer {
            position: punkt,
            released: true,
            ..Default::default()
        },
    );

    let text = ron::ser::to_string_pretty(&document, ron::ser::PrettyConfig::default()).unwrap();
    assert!(
        !text.contains("editing: Some"),
        "ett halvskrivet tal är inte ett värde och ska inte sparas"
    );
    let tillbaka: Document = ron::from_str(&text).unwrap();
    assert_eq!(tillbaka.value("x"), Some(NodeValue::Number(2.5)));
    assert_eq!(tillbaka.value("delare"), Some(NodeValue::Number(140.0)));
    assert_eq!(tillbaka.value("avsnitt"), Some(NodeValue::Bool(false)));
}

// -------------------------------------------------- radbrytning

fn knappar(bredder: &[f32]) -> Document {
    Document::new(
        Node::panel()
            .with_id("rot")
            .with_style(
                Style::wrap()
                    .with_size(Size::Fixed(100.0), Size::Auto)
                    .with_gap(4.0),
            )
            .with_children(bredder.iter().enumerate().map(|(i, bredd)| {
                Node::button(format!("k{i}"), "x")
                    .with_style(Style::default().with_size(Size::Fixed(*bredd), Size::Fixed(20.0)))
            })),
    )
}

#[test]
fn knappar_bryter_rad_nar_bredden_tar_slut() {
    // 40 + 4 + 40 = 84 ryms i 100; nästa gör 128 och hamnar på rad två.
    let prov = Prov::new(knappar(&[40.0, 40.0, 40.0]));
    let laid_out = prov.layout();

    assert_eq!(laid_out.rect("k0").unwrap().y, 0.0);
    assert_eq!(laid_out.rect("k1").unwrap().y, 0.0);
    assert_eq!(laid_out.rect("k1").unwrap().x, 44.0, "mellanrummet räknas");
    assert_eq!(laid_out.rect("k2").unwrap().y, 24.0, "ny rad");
    assert_eq!(laid_out.rect("k2").unwrap().x, 0.0);
}

#[test]
fn hojden_foljer_antalet_rader() {
    let en_rad = Prov::new(knappar(&[40.0, 40.0]))
        .layout()
        .rect("rot")
        .unwrap()
        .height;
    let tva_rader = Prov::new(knappar(&[40.0, 40.0, 40.0]))
        .layout()
        .rect("rot")
        .unwrap()
        .height;

    assert_eq!(en_rad, 20.0);
    assert_eq!(tva_rader, 44.0, "två rader om 20 plus mellanrummet");
}

#[test]
fn en_for_bred_knapp_far_en_egen_rad() {
    // Utan undantaget hade den tvingat fram en tom rad före sig.
    let prov = Prov::new(knappar(&[300.0, 40.0]));
    let laid_out = prov.layout();

    assert_eq!(laid_out.rect("k0").unwrap().y, 0.0);
    assert_eq!(laid_out.rect("k1").unwrap().y, 24.0);
}

// -------------------------------------------------- dubbelklick

#[test]
fn dubbelklick_rapporteras_separat() {
    let mut prov = Prov::new(knappar(&[40.0]));
    let punkt = prov.mitten("k0");

    prov.peka(Pointer {
        position: punkt,
        down: true,
        pressed: true,
        double: true,
        ..Default::default()
    });
    let events = prov.peka(Pointer {
        position: punkt,
        released: true,
        ..Default::default()
    });

    assert!(events.was_double_clicked("k0"));
    assert!(
        events.was_clicked("k0"),
        "en dubbelklickad knapp är också klickad – den som bara bryr sig \
         om klick ska slippa veta"
    );
}

#[test]
fn ett_vanligt_klick_ar_inget_dubbelklick() {
    let mut prov = Prov::new(knappar(&[40.0]));
    let events = prov.klicka("k0");
    assert!(events.was_clicked("k0"));
    assert!(!events.was_double_clicked("k0"));
}
