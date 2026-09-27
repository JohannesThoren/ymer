//! En inställningsruta med alla fälttyper, ritad till en PNG.
//!
//! Utan Ymer och utan GPU, som `standalone`. Skriptet klickar och skriver
//! åt en själv, så bilden visar ifyllda värden och en öppen dropdown.
//!
//!     cargo run -p ymer-ui --example widgets -- ut.png [typsnitt.ttf]

use ymer_ui::prelude::*;
use ymer_ui::{FontAtlas, Input, Key, draw_with};

#[path = "common/canvas.rs"]
mod canvas;
use canvas::Canvas;

const WIDTH: u32 = 520;
const HEIGHT: u32 = 500;

const PANEL: Color = Color::rgba(0.16, 0.18, 0.23, 1.0);
const FIELD: Color = Color::rgb(0.11, 0.12, 0.16);
const ACCENT: Color = Color::rgb(0.29, 0.56, 0.90);
const DIM: Color = Color::rgb(0.60, 0.64, 0.72);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args().nth(1).unwrap_or("widgets.png".to_string());
    let font_path = std::env::args()
        .nth(2)
        .unwrap_or("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf".to_string());

    let mut atlas = FontAtlas::from_font_bytes(std::fs::read(&font_path)?, 512, 512)?;
    let mut document = installningar();
    let viewport = Rect::new(0.0, 0.0, WIDTH as f32, HEIGHT as f32);
    let mut state = State::default();

    // Spela in några handgrepp, så att bilden visar fält i bruk och inte
    // bara tomma rutor.
    let laid_out = layout(&document, viewport, &atlas);
    click(&mut document, &laid_out, &mut state, "skarmskakning");
    click(&mut document, &laid_out, &mut state, "svar");

    // Skriv ett namn.
    click(&mut document, &laid_out, &mut state, "namn");
    feed(
        &mut document,
        &laid_out,
        &mut state,
        Input {
            text: "Johannes".into(),
            ..Default::default()
        },
    );

    // Och några rader i textrutan.
    click(&mut document, &laid_out, &mut state, "anteckning");
    for (index, rad) in ["Testar radbrytning", "i en textarea."].iter().enumerate() {
        if index > 0 {
            feed(
                &mut document,
                &laid_out,
                &mut state,
                Input {
                    keys: vec![Key::Enter],
                    ..Default::default()
                },
            );
        }
        feed(
            &mut document,
            &laid_out,
            &mut state,
            Input {
                text: (*rad).into(),
                ..Default::default()
            },
        );
    }

    // Dra volymen till ungefär 70.
    let track = laid_out.rect("volym").unwrap();
    drag(
        &mut document,
        &laid_out,
        &mut state,
        Vec2::new(track.x + track.width * 0.7, track.center().y),
    );

    // Öppna dropdownen och lämna den öppen i bilden.
    click(&mut document, &laid_out, &mut state, "upplosning");
    let over = laid_out.rect("upplosning").unwrap();
    state.update(
        &mut document,
        &laid_out,
        Pointer {
            position: Vec2::new(over.center().x, over.bottom() + over.height * 1.5),
            ..Default::default()
        },
    );

    // Layouta om: texten har ändrats, så måtten kan ha gjort det också.
    let laid_out = layout(&document, viewport, &atlas);
    let commands = draw_with(&document, &laid_out, &state, &atlas);

    let mut canvas = Canvas::new(WIDTH, HEIGHT, Color::rgb(0.09, 0.10, 0.13));
    canvas.run(&commands, &mut atlas);
    canvas.save(&output)?;

    println!("{} ritkommandon -> {output}", commands.len());
    Ok(())
}

fn click(document: &mut Document, laid_out: &ymer_ui::LaidOut, state: &mut State, id: &str) {
    let Some(rect) = laid_out.rect(id) else {
        eprintln!("hittade inte {id}");
        return;
    };
    let at = rect.center();
    state.update(
        document,
        laid_out,
        Pointer {
            position: at,
            down: true,
            pressed: true,
            ..Default::default()
        },
    );
    state.update(
        document,
        laid_out,
        Pointer {
            position: at,
            released: true,
            ..Default::default()
        },
    );
}

fn drag(document: &mut Document, laid_out: &ymer_ui::LaidOut, state: &mut State, to: Vec2) {
    state.update(
        document,
        laid_out,
        Pointer {
            position: to,
            down: true,
            pressed: true,
            ..Default::default()
        },
    );
    state.update(
        document,
        laid_out,
        Pointer {
            position: to,
            released: true,
            ..Default::default()
        },
    );
}

fn feed(document: &mut Document, laid_out: &ymer_ui::LaidOut, state: &mut State, input: Input) {
    state.update_with(document, laid_out, &input);
}

fn installningar() -> Document {
    Document::new(
        Node::panel()
            .with_style(Style::stack().with_size(Size::Fill, Size::Fill))
            .with_child(
                Node::panel()
                    .with_style(
                        Style::column()
                            .with_size(Size::Fixed(480.0), Size::Fixed(455.0))
                            .with_anchor(Anchor::Center)
                            .with_padding(Edges::all(18.0))
                            .with_gap(12.0)
                            .with_background(PANEL)
                            .with_radius(8.0),
                    )
                    .with_children([
                        Node::label("Inställningar")
                            .with_style(Style::default().with_font_size(18.0)),
                        rad("Namn", falt("namn", "Ditt namn")),
                        rad(
                            "Volym",
                            Node::slider("volym", 30.0, 0.0, 100.0).with_style(
                                Style::default()
                                    .with_size(Size::Fill, Size::Fixed(20.0))
                                    .with_color(ACCENT),
                            ),
                        ),
                        Node::label("Svårighetsgrad")
                            .with_style(Style::default().with_font_size(13.0).with_color(DIM)),
                        Node::panel()
                            .with_style(Style::row().with_gap(18.0))
                            .with_children([
                                Node::radio("latt", "svarighet", "Lätt", true)
                                    .with_style(radio_stil()),
                                Node::radio("normal", "svarighet", "Normal", false)
                                    .with_style(radio_stil()),
                                Node::radio("svar", "svarighet", "Svår", false)
                                    .with_style(radio_stil()),
                            ]),
                        Node::checkbox("skarmskakning", "Skärmskakning", false)
                            .with_style(radio_stil()),
                        Node::checkbox("vsync", "V-sync", true).with_style(radio_stil()),
                        Node::label("Anteckning")
                            .with_style(Style::default().with_font_size(13.0).with_color(DIM)),
                        Node::text_area("anteckning", "", 3).with_style(
                            Style::default()
                                .with_size(Size::Fill, Size::Fixed(54.0))
                                .with_padding(Edges::all(7.0))
                                .with_background(FIELD)
                                .with_radius(4.0)
                                .with_font_size(13.0),
                        ),
                        rad(
                            "Upplösning",
                            Node::dropdown(
                                "upplosning",
                                ["1280x720", "1920x1080", "2560x1440"],
                                Some(1),
                            )
                            .with_style(
                                Style::default()
                                    .with_size(Size::Fill, Size::Fixed(26.0))
                                    .with_padding(Edges::symmetric(8.0, 5.0))
                                    .with_background(FIELD)
                                    .with_radius(4.0)
                                    .with_font_size(13.0),
                            ),
                        ),
                    ]),
            ),
    )
}

fn rad(etikett: &str, falt: Node) -> Node {
    Node::panel()
        .with_style(
            Style::row()
                .with_size(Size::Fill, Size::Fixed(26.0))
                .with_gap(10.0)
                .with_align(Align::Center),
        )
        .with_children([
            Node::label(etikett).with_style(
                Style::default()
                    .with_size(Size::Fixed(90.0), Size::Auto)
                    .with_font_size(13.0)
                    .with_color(DIM),
            ),
            falt,
        ])
}

fn falt(id: &str, placeholder: &str) -> Node {
    Node::text_input(id, "")
        .with_placeholder(placeholder)
        .with_style(
            Style::default()
                .with_size(Size::Fill, Size::Fixed(26.0))
                .with_padding(Edges::symmetric(8.0, 5.0))
                .with_background(FIELD)
                .with_radius(4.0)
                .with_font_size(13.0),
        )
}

fn radio_stil() -> Style {
    Style::default().with_font_size(14.0)
}
