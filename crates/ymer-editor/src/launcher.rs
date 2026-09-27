//! Launchern. Första vyn: välj ett projekt eller skapa ett nytt.
//!
//! Byggd av `ymer-ui` som resten av editorn, men mycket enklare: den har
//! ingen värld att läsa och inget som måste läsas tillbaka annat än
//! namnfältet. Trädet byggs ändå om varje frame, av samma skäl som i
//! editorn – listan ändras när man skapar ett projekt.

use std::path::PathBuf;

use ymer_ui::prelude::*;
use ymer_ui::{Input, State};

use crate::chrome::Theme;
use crate::project::{self, ProjectHandle};

pub struct LauncherState {
    pub projects_dir: PathBuf,
    pub projects: Vec<ProjectHandle>,
    pub new_name: String,
    pub status: String,
    document: Document,
    ui: State,
    theme: Theme,
}

impl LauncherState {
    pub fn new(projects_dir: impl Into<PathBuf>) -> Self {
        let projects_dir = projects_dir.into();
        let projects = project::list(&projects_dir);
        Self {
            status: format!("{} projekt hittade", projects.len()),
            projects_dir,
            projects,
            new_name: String::new(),
            document: Document::new(Node::panel()),
            ui: State::default(),
            theme: Theme::dark(),
        }
    }

    pub fn refresh(&mut self) {
        self.projects = project::list(&self.projects_dir);
    }

    pub fn document(&self) -> &Document {
        &self.document
    }

    /// En frame. Returnerar ritlistan och projektet som valdes, om något.
    pub fn frame(
        &mut self,
        input: &Input,
        window: Rect,
        text: &dyn TextMeasure,
    ) -> (DrawList, Option<ProjectHandle>) {
        let laid_out = layout(&self.document, window, text);
        let events = self.ui.update_with(&mut self.document, &laid_out, input);

        if let Some(NodeValue::Text(name)) = self.document.value("namn") {
            self.new_name = name;
        }

        let mut chosen = None;
        for id in &events.clicked {
            if id == "skapa" {
                match project::create(&self.projects_dir, &self.new_name.clone()) {
                    Ok(handle) => {
                        self.status = format!("skapade {}", handle.project.name);
                        self.new_name.clear();
                        self.refresh();
                        // Öppna direkt – man vill alltid in i projektet
                        // man just skapat.
                        chosen = Some(handle);
                    }
                    Err(err) => self.status = format!("{err}"),
                }
            } else if let Some(index) = id
                .strip_prefix("oppna/")
                .and_then(|i| i.parse::<usize>().ok())
            {
                chosen = self.projects.get(index).cloned();
            }
        }

        let previous = std::mem::replace(&mut self.document, Document::new(Node::panel()));
        self.document = self.build();
        self.document.carry_view_state_from(&previous);

        let laid_out = layout(&self.document, window, text);
        let list = draw_with(&self.document, &laid_out, &self.ui, text);
        (list, chosen)
    }

    fn build(&self) -> Document {
        let theme = &self.theme;

        let rows: Vec<Node> = if self.projects.is_empty() {
            vec![Node::label("Inga projekt än. Skapa ett nedan.").with_style(theme.dim_label())]
        } else {
            self.projects
                .iter()
                .enumerate()
                .map(|(index, handle)| {
                    Node::panel()
                        .with_style(
                            Style::row()
                                .with_size(Size::Fill, Size::Fixed(52.0))
                                .with_padding(Edges::all(10.0))
                                .with_gap(10.0)
                                .with_align(Align::Center)
                                .with_background(theme.panel)
                                .with_radius(4.0),
                        )
                        .with_children([
                            Node::panel()
                                .with_style(Style::column().with_gap(2.0))
                                .with_children([
                                    Node::label(&handle.project.name)
                                        .with_style(theme.label().with_font_size(16.0)),
                                    Node::label(handle.root.display().to_string())
                                        .with_style(theme.dim_label()),
                                ]),
                            Node::spacer().with_style(
                                Style::default().with_size(Size::Fill, Size::Fixed(1.0)),
                            ),
                            Node::button(format!("oppna/{index}"), "Öppna")
                                .with_style(theme.button().with_background(theme.accent)),
                        ])
                })
                .collect()
        };

        Document::new(
            Node::panel()
                .with_id("rot")
                .with_style(
                    Style::column()
                        .with_size(Size::Fill, Size::Fill)
                        .with_padding(Edges::all(20.0))
                        .with_gap(12.0)
                        .with_background(theme.well),
                )
                .with_children([
                    Node::label("Projekt").with_style(theme.label().with_font_size(22.0)),
                    Node::label(self.projects_dir.display().to_string())
                        .with_style(theme.dim_label()),
                    Node::scroll("lista")
                        .with_style(
                            Style::column()
                                .with_size(Size::Fill, Size::Fill)
                                .with_gap(8.0)
                                .with_clip(true),
                        )
                        .with_children(rows),
                    Node::panel()
                        .with_style(
                            Style::row()
                                .with_size(Size::Fill, Size::Fixed(30.0))
                                .with_gap(10.0)
                                .with_align(Align::Center),
                        )
                        .with_children([
                            Node::text_input("namn", &self.new_name)
                                .with_placeholder("nytt projektnamn")
                                .with_style(theme.field(Size::Fixed(260.0))),
                            Node::button("skapa", "Skapa projekt")
                                .with_style(theme.button().with_background(theme.accent)),
                            Node::spacer().with_style(
                                Style::default().with_size(Size::Fill, Size::Fixed(1.0)),
                            ),
                            Node::label(&self.status).with_style(theme.dim_label()),
                        ]),
                ]),
        )
    }
}
