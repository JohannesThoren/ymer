//! Ritar ett `ymer-ui`-dokument genom motorns riktiga renderare.
//!
//! Skillnaden mot `ymer-ui`-exemplen är hela poängen: där rastrerades
//! ritlistan i mjukvara, här går den genom wgpu i skärmrymd, ovanpå en
//! 3D-scen. Det är den vägen ett spel tar.
//!
//!     cargo run -p ymer-runtime --example ui_gpu --features demo -- ut.png

use ymer_render::{Assets, Renderer};
use ymer_runtime::prelude::*;
use ymer_runtime::ui_backend::UiBackend;
use ymer_runtime::{build_render_list, demo, init_logging};
// Prelude:erna har varsin Color; gränssnittets vinner här.
use ymer_ui::prelude::*;
use ymer_ui::{Color, FontAtlas, draw_with};

const WIDTH: u32 = 900;
const HEIGHT: u32 = 520;

const PANEL: Color = Color::rgba(0.11, 0.13, 0.17, 0.88);
const ACCENT: Color = Color::rgb(0.29, 0.56, 0.90);
const DIM: Color = Color::rgb(0.62, 0.66, 0.74);

fn main() -> anyhow::Result<()> {
    init_logging();
    let output = std::env::args().nth(1).unwrap_or("ui_gpu.png".to_string());
    let font_path = std::env::args()
        .nth(2)
        .unwrap_or("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf".to_string());

    let mut renderer = pollster::block_on(Renderer::new_offscreen(WIDTH, HEIGHT))?;
    let assets = Assets::new(&mut renderer);

    // En vanlig 3D-scen under gränssnittet.
    let mut world = World::new();
    world.insert_resource(Time::new());
    demo::setup(&mut world);
    let mut schedule = Schedule::new(Update);
    schedule.add_systems(propagate_transforms);
    schedule.run(&mut world);

    let atlas = FontAtlas::from_font_bytes(std::fs::read(&font_path)?, 512, 512)
        .map_err(|err| anyhow::anyhow!(err))?;
    let mut backend = UiBackend::new(atlas, &mut renderer, &assets);

    let mut document = hud();
    let viewport = Rect::new(0.0, 0.0, WIDTH as f32, HEIGHT as f32);
    let mut state = State::default();

    // Rulla listan en bit, så att klippningen faktiskt prövas.
    let laid_out = layout(&document, viewport, backend.atlas());
    state.update(
        &mut document,
        &laid_out,
        Pointer {
            position: laid_out.rect("lista").unwrap().center(),
            scroll: Vec2::new(0.0, 30.0),
            ..Default::default()
        },
    );

    // Hovra en knapp, så att det syns att interaktionen lever.
    let laid_out = layout(&document, viewport, backend.atlas());
    if let Some(rect) = laid_out.rect("spela") {
        state.update(
            &mut document,
            &laid_out,
            Pointer {
                position: rect.center(),
                ..Default::default()
            },
        );
    }

    let laid_out = layout(&document, viewport, backend.atlas());
    let commands = draw_with(&document, &laid_out, &state, backend.atlas());

    let mut list = build_render_list(&mut world, renderer.aspect_ratio(), &assets);
    // Ordningen spelar roll: build rastrerar de glyfer framen behöver,
    // upload skickar atlasen till GPU:n. Tvärtom ritas varje tecken
    // första gången det används som tomrum.
    list.ui_items = backend.build(&commands, &assets);
    backend.upload(&mut renderer);

    renderer.render(&list)?;

    let pixels = renderer.capture_rgba()?;
    image::RgbaImage::from_raw(WIDTH, HEIGHT, pixels)
        .ok_or_else(|| anyhow::anyhow!("fel bildstorlek"))?
        .save(&output)?;

    println!(
        "{} 3D-objekt, {} ui-kvadrater -> {output}",
        list.items.len(),
        list.ui_items.len()
    );
    Ok(())
}

fn hud() -> Document {
    Document::new(
        Node::panel()
            .with_style(Style::stack().with_size(Size::Fill, Size::Fill))
            // Topplist.
            .with_child(
                Node::panel()
                    .with_style(
                        Style::row()
                            .with_size(Size::Fill, Size::Fixed(46.0))
                            .with_padding(Edges::symmetric(16.0, 9.0))
                            .with_gap(22.0)
                            .with_align(Align::Center)
                            .with_background(PANEL),
                    )
                    .with_children([
                        Node::label("Ymer").with_style(Style::default().with_font_size(18.0)),
                        nyckeltal("Guld", "1 170"),
                        nyckeltal("Invånare", "20 / 20"),
                        Node::spacer()
                            .with_style(Style::default().with_size(Size::Fill, Size::Fixed(1.0))),
                        Node::label("ritad i skärmrymd av wgpu")
                            .with_style(Style::default().with_font_size(12.0).with_color(DIM)),
                    ]),
            )
            // Sidopanel med fält.
            .with_child(
                Node::panel()
                    .with_style(
                        Style::column()
                            .with_size(Size::Fixed(230.0), Size::Fixed(250.0))
                            .with_anchor(Anchor::CenterLeft)
                            .with_offset(Vec2::new(18.0, 10.0))
                            .with_padding(Edges::all(14.0))
                            .with_gap(10.0)
                            .with_background(PANEL)
                            .with_radius(8.0),
                    )
                    .with_children([
                        Node::label("Bygg").with_style(Style::default().with_font_size(15.0)),
                        Node::button("spela", "Hus – 50 guld").with_style(
                            Style::default()
                                .with_size(Size::Fill, Size::Fixed(28.0))
                                .with_padding(Edges::symmetric(10.0, 6.0))
                                .with_background(ACCENT)
                                .with_radius(4.0)
                                .with_font_size(13.0),
                        ),
                        Node::checkbox("rutnat", "Visa rutnät", true)
                            .with_style(Style::default().with_font_size(13.0)),
                        Node::radio("latt", "grad", "Lätt", false)
                            .with_style(Style::default().with_font_size(13.0)),
                        Node::radio("svar", "grad", "Svår", true)
                            .with_style(Style::default().with_font_size(13.0)),
                        Node::label("Volym")
                            .with_style(Style::default().with_font_size(12.0).with_color(DIM)),
                        Node::slider("volym", 65.0, 0.0, 100.0).with_style(
                            Style::default()
                                .with_size(Size::Fill, Size::Fixed(18.0))
                                .with_color(ACCENT),
                        ),
                        Node::text_input("namn", "Johannes").with_style(
                            Style::default()
                                .with_size(Size::Fill, Size::Fixed(26.0))
                                .with_padding(Edges::symmetric(8.0, 5.0))
                                .with_background(Color::rgb(0.07, 0.08, 0.11))
                                .with_radius(4.0)
                                .with_font_size(13.0),
                        ),
                    ]),
            )
            // En rullad lista, för att visa klippningen. Den är rullad
            // 30 px, så första raden är halvt avskuren i överkanten och
            // den nedersta klipps mitt i sina glyfer.
            .with_child(
                Node::panel()
                    .with_style(
                        Style::column()
                            .with_size(Size::Fixed(240.0), Size::Fixed(160.0))
                            .with_anchor(Anchor::CenterRight)
                            .with_offset(Vec2::new(-18.0, 10.0))
                            .with_padding(Edges::all(10.0))
                            .with_gap(8.0)
                            .with_background(PANEL)
                            .with_radius(8.0),
                    )
                    .with_children([
                        Node::label("Hierarki").with_style(Style::default().with_font_size(15.0)),
                        Node::scroll("lista")
                            .with_style(
                                Style::column()
                                    .with_size(Size::Fill, Size::Fill)
                                    .with_clip(true)
                                    .with_gap(2.0),
                            )
                            .with_children((0..12).map(|i| {
                                Node::button(format!("rad{i}"), format!("entitet {i}")).with_style(
                                    Style::default()
                                        .with_size(Size::Fill, Size::Fixed(20.0))
                                        .with_padding(Edges::symmetric(6.0, 3.0))
                                        .with_background(Color::rgb(0.17, 0.19, 0.24))
                                        .with_font_size(12.0),
                                )
                            })),
                    ]),
            )
            // Statusrad.
            .with_child(
                Node::panel()
                    .with_style(
                        Style::row()
                            .with_size(Size::Fill, Size::Fixed(28.0))
                            .with_anchor(Anchor::BottomLeft)
                            .with_padding(Edges::symmetric(16.0, 6.0))
                            .with_background(PANEL),
                    )
                    .with_child(
                        Node::label("Samma dokument som mjukvaruexemplet – annan backend.")
                            .with_style(Style::default().with_font_size(12.0).with_color(DIM)),
                    ),
            ),
    )
}

fn nyckeltal(etikett: &str, varde: &str) -> Node {
    Node::panel()
        .with_style(Style::column().with_gap(1.0))
        .with_children([
            Node::label(etikett).with_style(Style::default().with_font_size(10.0).with_color(DIM)),
            Node::label(varde).with_style(Style::default().with_font_size(15.0)),
        ])
}
