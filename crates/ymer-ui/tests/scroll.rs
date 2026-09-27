//! Rullning och klippning.
//!
//! Som de andra testfilerna importerar den här ingenting ur Ymer, och
//! mäter med monospace så att felaktiga tal syns som tal.
//!
//! Klippning är lätt att tro på och svår att lita på: en bortrullad rad
//! *har* fortfarande en rektangel, och det enda som skiljer en riktig
//! implementation från en trasig är att raden varken ritas eller kan
//! klickas där den ligger. Det är vad de här testerna mäter.

use ymer_ui::prelude::*;

fn metrics() -> MonospaceMetrics {
    MonospaceMetrics {
        advance: 0.5,
        line_height: 1.0,
    }
}

fn viewport() -> Rect {
    Rect::new(0.0, 0.0, 200.0, 100.0)
}

/// Tio rader om 20 px i en yta som rymmer fem.
fn lista() -> Document {
    Document::new(
        Node::scroll("lista")
            .with_style(
                Style::column()
                    .with_size(Size::Fixed(200.0), Size::Fixed(100.0))
                    .with_clip(true)
                    .with_background(Color::rgb(0.1, 0.1, 0.1)),
            )
            .with_children((0..10).map(|i| {
                Node::button(format!("rad{i}"), format!("rad {i}")).with_style(
                    Style::default()
                        .with_size(Size::Fill, Size::Fixed(20.0))
                        .with_background(Color::rgb(0.2, 0.2, 0.2))
                        .with_font_size(20.0),
                )
            })),
    )
}

fn rulla(document: &mut Document, state: &mut State, delta: f32) -> Events {
    let laid_out = layout(document, viewport(), &metrics());
    state.update(
        document,
        &laid_out,
        Pointer {
            position: Vec2::new(100.0, 50.0),
            scroll: Vec2::new(0.0, delta),
            ..Default::default()
        },
    )
}

#[test]
fn innehallet_ar_hogre_an_ytan() {
    let document = lista();
    let laid_out = layout(&document, viewport(), &metrics());
    let placed = laid_out.placed("lista").expect("listan finns");

    assert_eq!(placed.rect.height, 100.0, "ytan är den man gav den");
    assert_eq!(
        placed.content.y, 200.0,
        "innehållet mäts fritt: tio rader om 20 px"
    );
}

#[test]
fn en_rullbar_yta_vaxer_inte_efter_sitt_innehall() {
    // Auto-höjd på en rullbar yta ger noll, inte 200. En yta som mäter sig
    // efter innehållet behöver aldrig rullas, och då är den inte rullbar.
    let document = Document::new(
        Node::panel()
            .with_id("rot")
            .with_style(Style::column().with_size(Size::Fixed(200.0), Size::Fixed(100.0)))
            .with_child(
                Node::scroll("lista")
                    .with_style(Style::column().with_size(Size::Fill, Size::Auto))
                    .with_children((0..10).map(|i| {
                        Node::label(format!("rad {i}"))
                            .with_id(format!("rad{i}"))
                            .with_style(Style::default().with_size(Size::Fill, Size::Fixed(20.0)))
                    })),
            ),
    );
    let laid_out = layout(&document, viewport(), &metrics());
    assert_eq!(laid_out.rect("lista").unwrap().height, 0.0);
}

#[test]
fn rader_under_kanten_klipps_bort() {
    let document = lista();
    let laid_out = layout(&document, viewport(), &metrics());

    // Rad 0–4 ligger inom de första 100 px, rad 5–9 under.
    assert_eq!(laid_out.visible_rect("rad4").unwrap().height, 20.0);
    assert!(
        laid_out.visible_rect("rad5").unwrap().is_empty(),
        "rad 5 börjar precis vid kanten och syns inte alls"
    );
    assert!(laid_out.visible_rect("rad9").unwrap().is_empty());

    // Rektangeln finns kvar – det är bara klippet som gömmer den. Testet
    // skulle vara meningslöst om raden inte hade en plats att gömmas på.
    assert_eq!(laid_out.rect("rad9").unwrap().y, 180.0);
}

#[test]
fn en_bortklippt_rad_kan_inte_klickas() {
    let document = lista();
    let laid_out = layout(&document, viewport(), &metrics());

    let inne = laid_out.rect("rad2").unwrap().center();
    assert_eq!(
        hit_test(&document, &laid_out, inne).as_deref(),
        Some("rad2")
    );

    // Punkten ligger *på* rad 7, men utanför listan.
    let ute = laid_out.rect("rad7").unwrap().center();
    assert_eq!(
        hit_test(&document, &laid_out, ute),
        None,
        "det man inte ser ska man inte kunna klicka"
    );
}

#[test]
fn en_bortklippt_rad_ritas_inte() {
    let document = lista();
    let laid_out = layout(&document, viewport(), &metrics());
    let list = draw(&document, &laid_out, &State::default());

    let texter: Vec<&str> = list
        .commands
        .iter()
        .filter_map(|c| match &c.command {
            Command::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();

    assert!(texter.contains(&"rad 0"));
    assert!(texter.contains(&"rad 4"));
    assert!(
        !texter.contains(&"rad 9"),
        "bortklippta kommandon ska inte ens hamna i listan – det är det \
         som gör en lång lista billig att rita"
    );
}

#[test]
fn hjulet_flyttar_innehallet() {
    let mut document = lista();
    let mut state = State::default();

    let events = rulla(&mut document, &mut state, 40.0);
    assert!(events.scroll_consumed, "hjulet ska ätas av listan");
    assert!(events.pointer_over_ui, "spelet bakom ska inte också zooma");

    let laid_out = layout(&document, viewport(), &metrics());
    assert_eq!(
        laid_out.rect("rad0").unwrap().y,
        -40.0,
        "raderna flyttas upp, inte listan"
    );
    assert_eq!(laid_out.rect("rad2").unwrap().y, 0.0);
    assert!(
        laid_out.visible_rect("rad0").unwrap().is_empty(),
        "rad 0 har rullat ur bild"
    );
    assert_eq!(laid_out.visible_rect("rad6").unwrap().height, 20.0);
}

#[test]
fn rullningen_klampas_i_bada_andar() {
    let mut document = lista();
    let mut state = State::default();

    rulla(&mut document, &mut state, -100.0);
    let laid_out = layout(&document, viewport(), &metrics());
    assert_eq!(
        laid_out.rect("rad0").unwrap().y,
        0.0,
        "det går inte att rulla ovanför första raden"
    );

    // Innehållet är 200 px i en yta på 100: mest 100 px rullning.
    rulla(&mut document, &mut state, 5000.0);
    let laid_out = layout(&document, viewport(), &metrics());
    assert_eq!(laid_out.rect("rad0").unwrap().y, -100.0);
    assert_eq!(
        laid_out.rect("rad9").unwrap().bottom(),
        100.0,
        "sista raden slutar i kanten – inte längre ut i tomrummet"
    );
}

#[test]
fn hjulet_utanfor_listan_rors_inte() {
    let mut document = lista();
    let mut state = State::default();
    let laid_out = layout(&document, viewport(), &metrics());

    let events = state.update(
        &mut document,
        &laid_out,
        Pointer {
            position: Vec2::new(100.0, 400.0),
            scroll: Vec2::new(0.0, 40.0),
            ..Default::default()
        },
    );
    assert!(!events.scroll_consumed);

    let laid_out = layout(&document, viewport(), &metrics());
    assert_eq!(laid_out.rect("rad0").unwrap().y, 0.0);
}

#[test]
fn den_innersta_listan_tar_hjulet() {
    // En lista i en lista: hjulet ska röra den man pekar i, inte den
    // yttre. Annars kan man inte rulla en inspector inuti en panel.
    let mut document = Document::new(
        Node::scroll("yttre")
            .with_style(
                Style::column()
                    .with_size(Size::Fixed(200.0), Size::Fixed(100.0))
                    .with_background(Color::rgb(0.1, 0.1, 0.1)),
            )
            .with_children([
                Node::scroll("inre")
                    .with_style(
                        Style::column()
                            .with_size(Size::Fill, Size::Fixed(60.0))
                            .with_background(Color::rgb(0.2, 0.2, 0.2)),
                    )
                    .with_children((0..6).map(|i| {
                        Node::label(format!("i{i}"))
                            .with_id(format!("i{i}"))
                            .with_style(Style::default().with_size(Size::Fill, Size::Fixed(20.0)))
                    })),
                Node::label("efter")
                    .with_id("efter")
                    .with_style(Style::default().with_size(Size::Fill, Size::Fixed(200.0))),
            ]),
    );

    let mut state = State::default();
    let laid_out = layout(&document, viewport(), &metrics());
    // Punkten ligger i den inre listan.
    state.update(
        &mut document,
        &laid_out,
        Pointer {
            position: Vec2::new(100.0, 30.0),
            scroll: Vec2::new(0.0, 20.0),
            ..Default::default()
        },
    );

    let laid_out = layout(&document, viewport(), &metrics());
    assert_eq!(laid_out.rect("i0").unwrap().y, -20.0, "inre listan rullade");
    assert_eq!(
        laid_out.rect("inre").unwrap().y,
        0.0,
        "den yttre stod still"
    );
}

#[test]
fn rullningslisten_syns_bara_nar_det_finns_nagot_att_rulla() {
    fn staplar(document: &Document) -> usize {
        let laid_out = layout(document, viewport(), &metrics());
        draw(document, &laid_out, &State::default())
            .commands
            .iter()
            .filter(|c| matches!(c.command, Command::Rect { .. }))
            .count()
    }

    // Tio rader: bakgrund + fem synliga rader + två för listen.
    assert_eq!(staplar(&lista()), 8);

    let kort = Document::new(
        Node::scroll("lista")
            .with_style(
                Style::column()
                    .with_size(Size::Fixed(200.0), Size::Fixed(100.0))
                    .with_background(Color::rgb(0.1, 0.1, 0.1)),
            )
            .with_child(
                Node::button("rad0", "rad 0").with_style(
                    Style::default()
                        .with_size(Size::Fill, Size::Fixed(20.0))
                        .with_background(Color::rgb(0.2, 0.2, 0.2)),
                ),
            ),
    );
    assert_eq!(staplar(&kort), 2, "bakgrund + en rad, ingen list");
}

#[test]
fn en_panel_med_clip_haller_sina_barn_inne() {
    // Klippning är inte bara för rullning: en panel som klipper ska inte
    // låta ett barn med ett ankare rinna ut ur den.
    let document = Document::new(
        Node::panel()
            .with_id("panel")
            .with_style(
                Style::stack()
                    .with_size(Size::Fixed(100.0), Size::Fixed(40.0))
                    .with_clip(true)
                    .with_background(Color::rgb(0.1, 0.1, 0.1)),
            )
            .with_child(
                Node::button("ute", "ute").with_style(
                    Style::default()
                        .with_size(Size::Fixed(40.0), Size::Fixed(20.0))
                        .with_offset(Vec2::new(120.0, 0.0))
                        .with_background(Color::rgb(0.3, 0.3, 0.3)),
                ),
            ),
    );
    let laid_out = layout(&document, viewport(), &metrics());

    assert_eq!(laid_out.rect("ute").unwrap().x, 120.0);
    assert!(laid_out.visible_rect("ute").unwrap().is_empty());
    assert_eq!(
        hit_test(&document, &laid_out, Vec2::new(140.0, 10.0)),
        None,
        "knappen syns inte och ska inte kunna klickas"
    );
}

#[test]
fn utan_clip_ritas_barnet_utanfor() {
    // Motprovet till testet ovan: klippningen ska bero på `clip`, inte
    // vara något som alltid händer.
    let document = Document::new(
        Node::panel()
            .with_id("panel")
            .with_style(
                Style::stack()
                    .with_size(Size::Fixed(100.0), Size::Fixed(40.0))
                    .with_background(Color::rgb(0.1, 0.1, 0.1)),
            )
            .with_child(
                Node::button("ute", "ute").with_style(
                    Style::default()
                        .with_size(Size::Fixed(40.0), Size::Fixed(20.0))
                        .with_offset(Vec2::new(120.0, 0.0))
                        .with_background(Color::rgb(0.3, 0.3, 0.3)),
                ),
            ),
    );
    let laid_out = layout(&document, viewport(), &metrics());

    assert_eq!(laid_out.visible_rect("ute").unwrap().width, 40.0);
    assert_eq!(
        hit_test(&document, &laid_out, Vec2::new(140.0, 10.0)).as_deref(),
        Some("ute")
    );
}

#[test]
fn rullning_overlever_att_dokumentet_byggs_om() {
    // Editorn bygger om sitt dokument varje frame. Förskjutningen bor i
    // noden, så den följer med när trädet byggs om – men bara om den som
    // bygger bär över den. Testet visar vägen: läs ut och sätt tillbaka.
    let mut document = lista();
    let mut state = State::default();
    rulla(&mut document, &mut state, 60.0);

    let Some(Kind::Scroll { offset }) = document.find("lista").map(|n| &n.kind) else {
        panic!("listan är rullbar");
    };
    let sparad = *offset;
    assert_eq!(sparad.y, 60.0);

    let mut nytt = lista();
    if let Some(Kind::Scroll { offset }) = nytt.find_mut("lista").map(|n| &mut n.kind) {
        *offset = sparad;
    }
    let laid_out = layout(&nytt, viewport(), &metrics());
    assert_eq!(laid_out.rect("rad0").unwrap().y, -60.0);
}
