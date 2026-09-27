//! Layout, träffdetektering och serialisering.
//!
//! Den här filen importerar med flit *ingenting* ur Ymer. Går den att
//! köra är gränsen intakt: biblioteket fungerar utanför motorn.
//!
//! Mätningen är monospace, inte ett riktigt typsnitt. En layoutbugg ska
//! synas som fel tal, inte som att någon bytte teckensnitt.

use ymer_ui::prelude::*;

fn metrics() -> MonospaceMetrics {
    // 10 px bred, 20 px hög per tecken vid storlek 20.
    MonospaceMetrics {
        advance: 0.5,
        line_height: 1.0,
    }
}

fn viewport() -> Rect {
    Rect::new(0.0, 0.0, 800.0, 600.0)
}

#[test]
fn fill_tar_hela_ytan() {
    let document = Document::new(
        Node::panel()
            .with_id("rot")
            .with_style(Style::default().with_size(Size::Fill, Size::Fill)),
    );
    let laid_out = layout(&document, viewport(), &metrics());
    assert_eq!(laid_out.rect("rot"), Some(viewport()));
}

#[test]
fn kolumn_staplar_barnen_med_mellanrum() {
    let document = Document::new(
        Node::panel()
            .with_style(
                Style::column()
                    .with_size(Size::Fixed(200.0), Size::Fill)
                    .with_gap(10.0),
            )
            .with_children([
                Node::panel()
                    .with_id("a")
                    .with_style(Style::default().with_size(Size::Fill, Size::Fixed(30.0))),
                Node::panel()
                    .with_id("b")
                    .with_style(Style::default().with_size(Size::Fill, Size::Fixed(50.0))),
            ]),
    );
    let laid_out = layout(&document, viewport(), &metrics());

    let a = laid_out.rect("a").unwrap();
    let b = laid_out.rect("b").unwrap();
    assert_eq!(a.y, 0.0);
    assert_eq!(a.height, 30.0);
    assert_eq!(b.y, 40.0, "30 hög plus 10 mellanrum");
    assert_eq!(b.height, 50.0);
    assert_eq!(a.width, 200.0, "Fill i en kolumn fyller bredden");
}

#[test]
fn padding_krymper_innerytan() {
    let document = Document::new(
        Node::panel()
            .with_style(
                Style::column()
                    .with_size(Size::Fixed(100.0), Size::Fixed(100.0))
                    .with_padding(Edges::all(8.0)),
            )
            .with_child(
                Node::panel()
                    .with_id("inuti")
                    .with_style(Style::default().with_size(Size::Fill, Size::Fixed(10.0))),
            ),
    );
    let laid_out = layout(&document, viewport(), &metrics());
    let inuti = laid_out.rect("inuti").unwrap();
    assert_eq!(inuti.x, 8.0);
    assert_eq!(inuti.y, 8.0);
    assert_eq!(inuti.width, 84.0, "100 minus 8 på varje sida");
}

#[test]
fn tva_fill_delar_lika() {
    let document = Document::new(
        Node::panel()
            .with_style(Style::row().with_size(Size::Fixed(300.0), Size::Fixed(50.0)))
            .with_children([
                Node::panel()
                    .with_id("v")
                    .with_style(Style::default().with_size(Size::Fill, Size::Fill)),
                Node::panel()
                    .with_id("h")
                    .with_style(Style::default().with_size(Size::Fill, Size::Fill)),
            ]),
    );

    let laid_out = layout(&document, viewport(), &metrics());
    assert_eq!(laid_out.rect("v").unwrap().width, 150.0);
    assert_eq!(laid_out.rect("h").unwrap().width, 150.0);
    assert_eq!(laid_out.rect("h").unwrap().x, 150.0);
}

#[test]
fn auto_vaxer_med_texten() {
    let document = Document::new(
        Node::panel()
            .with_id("rot")
            .with_style(Style::column().with_padding(Edges::all(4.0)))
            .with_child(Node::label("hej").with_style(Style::default().with_font_size(20.0))),
    );
    let laid_out = layout(&document, viewport(), &metrics());
    let rot = laid_out.rect("rot").unwrap();
    // "hej" = 3 tecken * 0.5 * 20 = 30 bred, 20 hög, plus 4 padding runt.
    assert_eq!(rot.width, 38.0);
    assert_eq!(rot.height, 28.0);
}

#[test]
fn ankare_haller_noden_i_hornet() {
    let document = Document::new(
        Node::panel()
            .with_style(Style::stack().with_size(Size::Fill, Size::Fill))
            .with_child(
                Node::panel().with_id("hud").with_style(
                    Style::default()
                        .with_size(Size::Fixed(100.0), Size::Fixed(40.0))
                        .with_anchor(Anchor::BottomRight)
                        .with_offset(Vec2::new(-16.0, -16.0)),
                ),
            ),
    );
    let laid_out = layout(&document, viewport(), &metrics());
    let hud = laid_out.rect("hud").unwrap();
    assert_eq!(hud.x, 800.0 - 100.0 - 16.0);
    assert_eq!(hud.y, 600.0 - 40.0 - 16.0);
}

#[test]
fn osynlig_nod_tar_ingen_plats() {
    let mut document = Document::new(
        Node::panel()
            .with_style(Style::column().with_size(Size::Fixed(100.0), Size::Fill))
            .with_children([
                Node::panel()
                    .with_id("dold")
                    .with_style(Style::default().with_size(Size::Fill, Size::Fixed(30.0))),
                Node::panel()
                    .with_id("efter")
                    .with_style(Style::default().with_size(Size::Fill, Size::Fixed(30.0))),
            ]),
    );

    assert!(document.set_visible("dold", false));
    let laid_out = layout(&document, viewport(), &metrics());
    assert_eq!(laid_out.rect("dold"), None, "dolda noder placeras inte");
    assert_eq!(
        laid_out.rect("efter").unwrap().y,
        0.0,
        "efterföljaren ska flytta upp"
    );
}

#[test]
fn justify_center_centrerar_pa_huvudaxeln() {
    let document = Document::new(
        Node::panel()
            .with_style(
                Style::row()
                    .with_size(Size::Fixed(300.0), Size::Fixed(50.0))
                    .with_justify(Justify::Center),
            )
            .with_child(
                Node::panel()
                    .with_id("mitten")
                    .with_style(Style::default().with_size(Size::Fixed(100.0), Size::Fill)),
            ),
    );
    let laid_out = layout(&document, viewport(), &metrics());
    assert_eq!(laid_out.rect("mitten").unwrap().x, 100.0);
}

// --------------------------------------------------------- interaktion

fn knapp_dokument() -> Document {
    Document::new(
        Node::panel()
            .with_style(Style::stack().with_size(Size::Fill, Size::Fill))
            .with_child(
                Node::button("spela", "Spela").with_style(
                    Style::default()
                        .with_size(Size::Fixed(100.0), Size::Fixed(40.0))
                        .with_background(Color::rgb(0.2, 0.4, 0.8)),
                ),
            ),
    )
}

#[test]
fn klick_kraver_ner_och_upp_pa_samma_nod() {
    let document = knapp_dokument();
    let laid_out = layout(&document, viewport(), &metrics());
    let mut state = State::default();
    let pa_knappen = Vec2::new(50.0, 20.0);

    let ner = state.update(
        &document,
        &laid_out,
        Pointer {
            position: pa_knappen,
            down: true,
            pressed: true,
            ..Default::default()
        },
    );
    assert!(ner.clicked.is_empty(), "nedtryck är inte ett klick än");

    let upp = state.update(
        &document,
        &laid_out,
        Pointer {
            position: pa_knappen,
            released: true,
            ..Default::default()
        },
    );
    assert!(upp.was_clicked("spela"));
}

#[test]
fn slappt_utanfor_ger_inget_klick() {
    let document = knapp_dokument();
    let laid_out = layout(&document, viewport(), &metrics());
    let mut state = State::default();

    state.update(
        &document,
        &laid_out,
        Pointer {
            position: Vec2::new(50.0, 20.0),
            down: true,
            pressed: true,
            ..Default::default()
        },
    );
    let upp = state.update(
        &document,
        &laid_out,
        Pointer {
            position: Vec2::new(400.0, 400.0),
            released: true,
            ..Default::default()
        },
    );
    assert!(
        upp.clicked.is_empty(),
        "drar man ut pekaren ska klicket ångras"
    );
}

#[test]
fn pekaren_over_ui_flaggas() {
    let document = knapp_dokument();
    let laid_out = layout(&document, viewport(), &metrics());
    let mut state = State::default();

    let over = state.update(
        &document,
        &laid_out,
        Pointer {
            position: Vec2::new(50.0, 20.0),
            ..Default::default()
        },
    );
    assert!(over.pointer_over_ui, "spelet bakom ska inte reagera");

    let bredvid = state.update(
        &document,
        &laid_out,
        Pointer {
            position: Vec2::new(400.0, 400.0),
            ..Default::default()
        },
    );
    assert!(!bredvid.pointer_over_ui);
}

#[test]
fn genomskinlig_panel_blockerar_inte() {
    // En panel utan bakgrund är luft. Annars hade en helskärmsrot
    // svalt varje klick i spelet bakom.
    let document = Document::new(
        Node::panel()
            .with_id("rot")
            .with_style(Style::stack().with_size(Size::Fill, Size::Fill)),
    );
    let laid_out = layout(&document, viewport(), &metrics());
    let mut state = State::default();
    let events = state.update(
        &document,
        &laid_out,
        Pointer {
            position: Vec2::new(400.0, 300.0),
            ..Default::default()
        },
    );
    assert!(!events.pointer_over_ui);
}

// ------------------------------------------------------ data och skript

#[test]
fn dokumentet_gar_igenom_ron() {
    let document = knapp_dokument();
    let text = ron::ser::to_string_pretty(&document, ron::ser::PrettyConfig::default()).unwrap();
    let tillbaka: Document = ron::from_str(&text).unwrap();
    assert_eq!(
        document, tillbaka,
        "ett UI ska överleva en vända till fil och tillbaka"
    );
}

#[test]
fn text_gar_att_satta_pa_id() {
    // Det här är vägen TypeScript tar: slå upp på id, skriv texten.
    let mut document =
        Document::new(Node::panel().with_child(Node::label("0 guld").with_id("guld")));
    assert!(document.set_text("guld", "1200 guld"));
    assert_eq!(
        document.find("guld").unwrap().kind.text(),
        Some("1200 guld")
    );

    assert!(
        !document.set_text("finns-inte", "x"),
        "en felstavad id ska rapporteras, inte tigas ihjäl"
    );
}

#[test]
fn ritlistan_foljer_traedet() {
    let document = knapp_dokument();
    let laid_out = layout(&document, viewport(), &metrics());
    let state = State::default();
    let list = draw(&document, &laid_out, &state);

    // Knappen ger en bakgrund och en text; roten saknar bakgrund.
    let rects = list
        .commands
        .iter()
        .filter(|c| matches!(c, Command::Rect { .. }))
        .count();
    let texts = list
        .commands
        .iter()
        .filter(|c| matches!(c, Command::Text { .. }))
        .count();
    assert_eq!(rects, 1);
    assert_eq!(texts, 1);
}

#[test]
fn hovrad_knapp_ritas_ljusare() {
    let document = knapp_dokument();
    let laid_out = layout(&document, viewport(), &metrics());

    let vilande = draw(&document, &laid_out, &State::default());
    let mut state = State::default();
    state.update(
        &document,
        &laid_out,
        Pointer {
            position: Vec2::new(50.0, 20.0),
            ..Default::default()
        },
    );
    let hovrad = draw(&document, &laid_out, &state);

    assert_ne!(
        vilande.commands[0], hovrad.commands[0],
        "hovring ska synas utan att dokumentet ändras"
    );
}

#[test]
fn farg_ur_hex() {
    assert_eq!(Color::from_hex("#ffffff"), Some(Color::WHITE));
    assert_eq!(
        Color::from_hex("#00000080").map(|c| (c.a * 255.0).round()),
        Some(128.0)
    );
    assert_eq!(Color::from_hex("inte en färg"), None);
}
