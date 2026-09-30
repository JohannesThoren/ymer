//! Trädet, byggt ur världen.
//!
//! Allt här är en ren funktion av `EditorState` plus världen: inga
//! beslut, inga sidoeffekter. Det som användaren gjort har redan
//! verkställts när den här koden körs, och det som hör till vyn bärs över
//! efteråt av [`Document::carry_view_state_from`].

use bevy_ecs::prelude::*;
use serde_json::Value;
use ymer_scene::TypeRegistry;
use ymer_ui::prelude::*;

use super::fields::{self, Assets, Shape, field_id, shape_of};
use super::theme::{self, Theme};
use super::{Chrome, components, hierarchy_rows};
use crate::EditorState;

impl Chrome {
    pub(super) fn build(
        &self,
        editor: &EditorState,
        world: &World,
        registry: &TypeRegistry,
    ) -> Document {
        let theme = &self.theme;
        let mut rows: Vec<Node> = vec![
            toolbar(theme, editor, world, registry),
            middle(self, theme, editor, world, registry),
        ];

        if editor.browser.is_some() {
            rows.push(
                Node::divider_horizontal("split/bottom", self.bottom, 80.0, 500.0)
                    .from_end()
                    .with_style(
                        Style::default()
                            .with_size(Size::Fill, Size::Auto)
                            .with_background(theme.panel)
                            .with_color(theme.dim),
                    ),
            );
            rows.push(files(theme, editor, self.bottom));
        }

        Document::new(
            Node::panel()
                .with_id("rot")
                // Ingen bakgrund på roten. Scenen ritas *under*
                // gränssnittet, så en heltäckande rot hade målat över den
                // – och svalt varje klick som var menat för scenen.
                .with_style(Style::column().with_size(Size::Fill, Size::Fill))
                .with_children(rows),
        )
    }
}

fn toolbar(theme: &Theme, editor: &EditorState, world: &World, registry: &TypeRegistry) -> Node {
    let count = hierarchy_rows(world, registry).len();
    let mut items = vec![
        Node::button(
            "tb/play",
            // Bara tecken typsnittet har. En saknad glyf ritas som en
            // ruta, och en ruta i topplisten ser ut som ett programfel.
            if editor.playing { "Paus" } else { "Spela" },
        )
        .with_style(theme.button().with_background(theme.accent)),
    ];

    if editor.project.is_some() {
        items.push(Node::button("tb/export", "Exportera").with_style(theme.button()));
    }
    items.push(Node::button("tb/spawn", "+ Entitet").with_style(theme.button()));

    if editor.selected.is_some() {
        items.push(Node::button("tb/dup", "Duplicera").with_style(theme.button()));
        items.push(Node::button("tb/del", "Ta bort").with_style(theme.button()));
        items.push(Node::button("tb/prefab", "Prefab").with_style(theme.button()));
    }

    items.push(Node::button("tb/save", "Spara").with_style(theme.button()));
    items.push(Node::button("tb/load", "Ladda").with_style(theme.button()));
    items.push(
        Node::text_input("tb/scene", &editor.scene_path)
            .with_placeholder("scenfil")
            .with_style(theme.field(Size::Fixed(170.0))),
    );
    items.push(Node::label(format!("{count} entiteter")).with_style(theme.dim_label()));
    // Statusraden trycks ut åt höger av en utfyllnad.
    items.push(Node::spacer().with_style(Style::default().with_size(Size::Fill, Size::Fixed(1.0))));
    items.push(Node::label(&editor.status).with_style(theme.dim_label()));

    Node::panel()
        .with_style(
            Style::row()
                .with_size(Size::Fill, Size::Fixed(theme::TOOLBAR))
                .with_padding(Edges::symmetric(8.0, 5.0))
                .with_gap(6.0)
                .with_align(Align::Center)
                .with_background(theme.panel)
                .with_clip(true),
        )
        .with_children(items)
}

/// Hierarki, scen och inspector bredvid varandra.
fn middle(
    chrome: &Chrome,
    theme: &Theme,
    editor: &EditorState,
    world: &World,
    registry: &TypeRegistry,
) -> Node {
    Node::panel()
        .with_style(Style::row().with_size(Size::Fill, Size::Fill))
        .with_children([
            hierarchy(theme, editor, world, registry, chrome.left),
            Node::divider("split/left", chrome.left, 120.0, 500.0).with_style(
                Style::default()
                    .with_size(Size::Auto, Size::Fill)
                    .with_background(theme.panel)
                    .with_color(theme.dim),
            ),
            // Hålet där scenen syns. Utan träffyta: ett klick här hör
            // till scenen, inte till gränssnittet.
            Node::spacer().with_id("scen").with_style(
                Style::default()
                    .with_size(Size::Fill, Size::Fill)
                    .without_hit_test(),
            ),
            Node::divider("split/right", chrome.right, 200.0, 600.0)
                .from_end()
                .with_style(
                    Style::default()
                        .with_size(Size::Auto, Size::Fill)
                        .with_background(theme.panel)
                        .with_color(theme.dim),
                ),
            inspector(theme, editor, world, registry, chrome.right),
        ])
}

fn hierarchy(
    theme: &Theme,
    editor: &EditorState,
    world: &World,
    registry: &TypeRegistry,
    width: f32,
) -> Node {
    let rows = hierarchy_rows(world, registry)
        .into_iter()
        .map(|(entity, name, depth)| {
            let selected = editor.selected == Some(entity);
            // Indraget görs med utfyllnad, inte med ett tomrum framför:
            // träffytan ska följa hela raden, annars går det inte att klicka
            // på ett djupt liggande barn i högerkanten.
            Node::button(
                format!("hier/{}", entity.to_bits()),
                if depth > 0 {
                    format!("└ {name}")
                } else {
                    name
                },
            )
            .with_style(theme.row(selected).with_padding(Edges::new(
                6.0 + depth as f32 * theme::INDENT,
                6.0,
                2.0,
                2.0,
            )))
        });

    panel_with_title(
        theme,
        "Hierarki",
        Size::Fixed(width),
        Node::scroll("hier/scroll")
            .with_style(
                Style::column()
                    .with_size(Size::Fill, Size::Fill)
                    .with_clip(true),
            )
            .with_children(rows),
    )
}

fn files(theme: &Theme, editor: &EditorState, height: f32) -> Node {
    let Some(browser) = editor.browser.as_ref() else {
        return Node::spacer();
    };

    let head = Node::panel()
        .with_style(
            Style::row()
                .with_size(Size::Fill, Size::Fixed(26.0))
                .with_gap(6.0)
                .with_align(Align::Center),
        )
        .with_children([
            Node::button("files/up", "Upp").with_style(theme.button()),
            Node::label(browser.breadcrumb()).with_style(theme.dim_label()),
            Node::text_input("files/name", &browser.new_name)
                .with_placeholder("namn.ts")
                .with_style(theme.field(Size::Fixed(140.0))),
            Node::button("files/new-file", "Ny fil").with_style(theme.button()),
            Node::button("files/new-dir", "Ny mapp").with_style(theme.button()),
            Node::button("files/delete", "Ta bort").with_style(theme.button()),
            Node::spacer().with_style(Style::default().with_size(Size::Fill, Size::Fixed(1.0))),
            Node::label(&browser.status).with_style(theme.dim_label()),
        ]);

    let rows = browser
        .entries()
        .into_iter()
        .enumerate()
        .map(|(index, entry)| {
            let selected = browser.selected.as_deref() == Some(entry.path.as_path());
            // Mappar känns igen på snedstrecket, som i en terminal.
            // Ikoner hade krävt emoji, och dem har typsnittet inte.
            let name = if entry.is_dir {
                format!("{}/", entry.name)
            } else {
                entry.name.clone()
            };
            Node::button(format!("files/{index}"), name).with_style(theme.row(selected))
        });

    Node::panel()
        .with_style(
            theme
                .panel(Size::Fill, Size::Fixed(height))
                .with_padding(Edges::all(8.0))
                .with_gap(4.0),
        )
        .with_children([
            head,
            Node::scroll("files/scroll")
                .with_style(
                    Style::column()
                        .with_size(Size::Fill, Size::Fill)
                        .with_clip(true),
                )
                .with_children(rows),
        ])
}

fn inspector(
    theme: &Theme,
    editor: &EditorState,
    world: &World,
    registry: &TypeRegistry,
    width: f32,
) -> Node {
    let Some(entity) = editor.selected.filter(|e| world.entities().contains(*e)) else {
        return panel_with_title(
            theme,
            "Inspector",
            Size::Fixed(width),
            Node::label("Ingen entitet vald.").with_style(theme.dim_label()),
        );
    };

    let assets = Assets {
        scripts: &editor.scripts,
        textures: &editor.textures,
        meshes: &editor.available_meshes,
    };
    let (present, missing) = components(world, registry, entity);

    let mut body: Vec<Node> =
        vec![Node::checkbox("insp/raw", "RON-läge", editor.raw_mode).with_style(theme.label())];

    for (name, value) in &present {
        let content: Vec<Node> = if editor.raw_mode {
            let draft = editor
                .drafts
                .get(&(entity, name.clone()))
                .cloned()
                .unwrap_or_default();
            vec![
                Node::text_area(format!("raw/{name}"), draft, 3)
                    .with_style(theme.field(Size::Fill).with_size(Size::Fill, Size::Auto)),
                Node::panel()
                    .with_style(Style::row().with_gap(6.0))
                    .with_children([
                        Node::button(format!("raw/{name}/apply"), "Applicera")
                            .with_style(theme.button()),
                        Node::button(format!("insp/{name}/remove"), "Ta bort")
                            .with_style(theme.button()),
                    ]),
            ]
        } else {
            let mut nodes = vec![field(theme, name, &mut Vec::new(), None, value, &assets)];
            nodes.push(
                Node::button(format!("insp/{name}/remove"), "Ta bort").with_style(theme.button()),
            );
            nodes
        };

        body.push(
            Node::collapsible(format!("insp/{name}"), name, true)
                .with_style(
                    Style::column()
                        .with_size(Size::Fill, Size::Auto)
                        .with_gap(4.0)
                        .with_color(theme.text)
                        .with_font_size(theme.font),
                )
                .with_children(content),
        );
    }

    if !missing.is_empty() {
        body.push(Node::label("Lägg till komponent:").with_style(theme.dim_label()));
        body.push(
            Node::panel()
                .with_style(
                    Style::wrap()
                        .with_size(Size::Fill, Size::Auto)
                        .with_gap(4.0),
                )
                .with_children(missing.iter().map(|name| {
                    Node::button(format!("add/{name}"), format!("+ {name}"))
                        .with_style(theme.button().with_font_size(theme.small))
                })),
        );
    }

    panel_with_title(
        theme,
        "Inspector",
        Size::Fixed(width),
        Node::scroll("insp/scroll")
            .with_style(
                Style::column()
                    .with_size(Size::Fill, Size::Fill)
                    .with_gap(6.0)
                    .with_clip(true),
            )
            .with_children(body),
    )
}

fn panel_with_title(theme: &Theme, title: &str, width: Size, content: Node) -> Node {
    Node::panel()
        .with_style(
            theme
                .panel(width, Size::Fill)
                .with_padding(Edges::all(8.0))
                .with_gap(6.0),
        )
        .with_children([
            Node::label(title).with_style(theme.label().with_font_size(theme.font + 1.0)),
            content,
        ])
}

/// Ett värde som fält. Rekursiv, och parallell med `read::field`.
fn field(
    theme: &Theme,
    component: &str,
    path: &mut Vec<String>,
    label: Option<&str>,
    value: &Value,
    assets: &Assets,
) -> Node {
    let id = field_id(component, path);
    match shape_of(component, label, value, assets) {
        Shape::Bool => {
            Node::checkbox(id, "", value.as_bool().unwrap_or(false)).with_style(theme.label())
        }

        Shape::Int => Node::number_in(id, value.as_f64().unwrap_or(0.0), 1.0, -1.0e9, 1.0e9)
            .with_style(theme.field(Size::Fixed(70.0))),

        Shape::Float { step } => Node::number(id, value.as_f64().unwrap_or(0.0), step)
            .with_style(theme.field(Size::Fixed(70.0))),

        Shape::Text => Node::text_input(id, value.as_str().unwrap_or_default())
            .with_style(theme.field(Size::Fill)),

        Shape::Asset {
            options,
            placeholder,
        } => {
            let current = value.as_str().unwrap_or_default();
            let selected = options.iter().position(|option| option == current);
            Node::dropdown(id, options.iter().cloned(), selected)
                .with_placeholder(placeholder)
                .with_style(theme.field(Size::Fill))
        }

        Shape::Vector { axes } => {
            let items = value.as_array().cloned().unwrap_or_default();
            let mut children = Vec::new();
            for (index, axis) in axes.iter().enumerate() {
                path.push(index.to_string());
                children.push(Node::label(*axis).with_style(theme.dim_label()));
                children.push(
                    Node::number(
                        field_id(component, path),
                        items.get(index).and_then(Value::as_f64).unwrap_or(0.0),
                        label.map(super::fields::vector_step).unwrap_or(0.05),
                    )
                    .with_style(theme.field(Size::Fixed(64.0))),
                );
                path.pop();
            }
            Node::panel()
                .with_style(Style::row().with_gap(3.0).with_align(Align::Center))
                .with_children(children)
        }

        Shape::Euler => {
            let items = value.as_array().cloned().unwrap_or_default();
            let degrees = fields::euler_degrees(&items);
            let mut children = Vec::new();
            for (index, axis) in ["x", "y", "z"].iter().enumerate() {
                path.push(format!("deg{index}"));
                children.push(Node::label(*axis).with_style(theme.dim_label()));
                children.push(
                    Node::number(field_id(component, path), degrees[index] as f64, 1.0)
                        .with_style(theme.field(Size::Fixed(64.0))),
                );
                path.pop();
            }
            Node::panel()
                .with_style(Style::row().with_gap(3.0).with_align(Align::Center))
                .with_children(children)
        }

        Shape::Color => {
            let map = value.as_object().cloned().unwrap_or_default();
            let rgba: Vec<f32> = ["r", "g", "b", "a"]
                .iter()
                .map(|key| map.get(*key).and_then(Value::as_f64).unwrap_or(1.0) as f32)
                .collect();
            let mut children = vec![
                // Provbiten: den enda delen som visar vad talen betyder.
                Node::panel().with_style(
                    Style::default()
                        .with_size(Size::Fixed(22.0), Size::Fixed(18.0))
                        .with_background(Color::rgb(rgba[0], rgba[1], rgba[2]))
                        .with_radius(3.0),
                ),
            ];
            for (index, key) in ["r", "g", "b", "a"].iter().enumerate() {
                path.push((*key).to_string());
                children.push(
                    Node::number_in(
                        field_id(component, path),
                        rgba[index] as f64,
                        0.01,
                        0.0,
                        1.0,
                    )
                    .with_style(theme.field(Size::Fixed(52.0))),
                );
                path.pop();
            }
            Node::panel()
                .with_style(Style::row().with_gap(3.0).with_align(Align::Center))
                .with_children(children)
        }

        Shape::Object => {
            let map = value.as_object().cloned().unwrap_or_default();
            Node::panel()
                .with_style(
                    Style::column()
                        .with_size(Size::Fill, Size::Auto)
                        .with_gap(3.0),
                )
                .with_children(map.iter().map(|(key, inner)| {
                    path.push(key.clone());
                    let node = Node::panel()
                        .with_style(
                            Style::row()
                                .with_size(Size::Fill, Size::Auto)
                                .with_gap(6.0)
                                .with_align(Align::Center),
                        )
                        .with_children([
                            Node::label(key).with_style(
                                theme.dim_label().with_size(Size::Fixed(78.0), Size::Auto),
                            ),
                            field(theme, component, path, Some(key), inner, assets),
                        ]);
                    path.pop();
                    node
                }))
        }

        Shape::Array => {
            let items = value.as_array().cloned().unwrap_or_default();
            Node::panel()
                .with_style(
                    Style::column()
                        .with_size(Size::Fill, Size::Auto)
                        .with_gap(3.0),
                )
                .with_children(items.iter().enumerate().map(|(index, inner)| {
                    path.push(index.to_string());
                    let node = Node::panel()
                        .with_style(Style::row().with_gap(6.0).with_align(Align::Center))
                        .with_children([
                            Node::label(format!("[{index}]")).with_style(theme.dim_label()),
                            field(theme, component, path, None, inner, assets),
                        ]);
                    path.pop();
                    node
                }))
        }

        Shape::Null => Node::label("null").with_style(theme.dim_label()),
    }
}
