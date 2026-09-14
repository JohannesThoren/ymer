//! Launchern. Första vyn: välj ett projekt eller skapa ett nytt.

use std::path::PathBuf;

use crate::project::{self, ProjectHandle};

pub struct LauncherState {
    pub projects_dir: PathBuf,
    pub projects: Vec<ProjectHandle>,
    pub new_name: String,
    pub status: String,
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
        }
    }

    pub fn refresh(&mut self) {
        self.projects = project::list(&self.projects_dir);
    }
}

/// Ritar launchern. Returnerar projektet användaren valde, om något.
pub fn run_ui(
    ctx: &egui::Context,
    raw_input: egui::RawInput,
    state: &mut LauncherState,
) -> (egui::FullOutput, Option<ProjectHandle>) {
    let mut chosen: Option<ProjectHandle> = None;
    let mut create = false;

    let output = ctx.run_ui(raw_input, |ui| {
        egui::Panel::top("launcher_head").show(ui, |ui| {
            ui.add_space(10.0);
            ui.heading("Projekt");
            ui.label(
                egui::RichText::new(state.projects_dir.display().to_string())
                    .weak()
                    .monospace(),
            );
            ui.add_space(10.0);
        });

        egui::Panel::bottom("launcher_new").show(ui, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut state.new_name)
                        .hint_text("nytt projektnamn")
                        .desired_width(240.0),
                );
                if ui.button("Skapa projekt").clicked() {
                    create = true;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(&state.status).weak());
                });
            });
            ui.add_space(8.0);
        });

        egui::ScrollArea::vertical().show(ui, |ui| {
            if state.projects.is_empty() {
                ui.add_space(24.0);
                ui.label("Inga projekt än. Skapa ett nedan.");
                return;
            }

            for handle in &state.projects {
                ui.add_space(6.0);
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(
                                egui::RichText::new(&handle.project.name)
                                    .strong()
                                    .size(16.0),
                            );
                            ui.label(
                                egui::RichText::new(handle.root.display().to_string())
                                    .weak()
                                    .monospace()
                                    .size(11.0),
                            );
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Öppna").clicked() {
                                chosen = Some(handle.clone());
                            }
                        });
                    });
                });
            }
        });
    });

    if create {
        match project::create(&state.projects_dir, &state.new_name.clone()) {
            Ok(handle) => {
                state.status = format!("skapade {}", handle.project.name);
                state.new_name.clear();
                state.refresh();
                // Öppna direkt – man vill alltid in i projektet man just skapat.
                chosen = Some(handle);
            }
            Err(err) => state.status = format!("{err}"),
        }
    }

    (output, chosen)
}
