//! Spelets gränssnitt: HUD på toppen, byggpalett till vänster och en
//! panel för den markerade byggnaden.
//!
//! Allt ritas ur `World` och skriver tillbaka dit. Knapparna sätter bara
//! `Stad::vald` eller flaggar för rivning; själva bygget sker i spelets
//! egen kod, som äger `World` utanför UI-stängningen.

use bevy_ecs::prelude::*;

use crate::{Kind, Stad};

/// Vad spelaren bad om i UI:t den här framen och som spelet måste
/// verkställa efteråt. `with_ui` lånar världen, så rivning kan inte ske
/// mitt i ritandet – den sparas här och utförs av spelloopen.
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct UiKommando {
    pub riv: Option<crate::Tile>,
}

pub fn rita(ui_root: &mut egui::Ui, world: &mut World) {
    let (stad, tal) = {
        let stad = world.resource::<Stad>();
        (stad.clone(), stad.nyckeltal())
    };

    let mut vald = stad.vald;
    let mut markerad = stad.markerad;
    let mut riv: Option<crate::Tile> = None;

    // --- HUD ----------------------------------------------------------
    egui::Panel::top("hud").show(ui_root, |ui| {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.heading("🏙 Staden");
            ui.separator();

            nyckeltal(ui, "Guld", format!("{:.0}", stad.guld));
            nyckeltal(
                ui,
                "Invånare",
                format!("{:.0} / {}", stad.invanare.floor(), tal.platser),
            );
            nyckeltal(ui, "Jobb", format!("{} / {}", tal.sysselsatta, tal.jobb));
            nyckeltal(ui, "Inkomst", format!("{:.1} guld/s", tal.inkomst));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(egui::RichText::new(format!("{} byggnader", stad.antal())).weak());
            });
        });
        ui.add_space(4.0);
    });

    // --- byggpalett ---------------------------------------------------
    egui::Panel::left("bygg")
        .resizable(false)
        .exact_size(230.0)
        .show(ui_root, |ui| {
            ui.add_space(6.0);
            ui.label(egui::RichText::new("Bygg").strong());
            ui.separator();

            for kind in Kind::ALL {
                let har_rad = stad.guld >= kind.kostnad();
                let är_vald = kind == vald;

                let text = format!("{}  –  {:.0} guld", kind.namn(), kind.kostnad());
                let knapp = egui::Button::new(text).selected(är_vald);

                // Knappen går att välja även utan råd, så att man kan
                // läsa beskrivningen och se vad man sparar till.
                if ui.add(knapp).clicked() {
                    vald = kind;
                }
                ui.label(
                    egui::RichText::new(kind.beskrivning())
                        .small()
                        .color(if har_rad {
                            egui::Color32::from_gray(170)
                        } else {
                            egui::Color32::from_rgb(200, 120, 110)
                        }),
                );
                ui.add_space(8.0);
            }

            ui.separator();
            ui.label(
                egui::RichText::new(
                    "Klicka på marken för att bygga.\nKlicka på ett hus för att markera det.",
                )
                .small()
                .weak(),
            );
        });

    // --- markerad byggnad ---------------------------------------------
    if let Some(tile) = markerad
        && let Some(kind) = stad.upptagen(tile)
    {
        egui::Window::new("Byggnad")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::RIGHT_BOTTOM, [-16.0, -48.0])
            .show(ui_root.ctx(), |ui| {
                ui.label(egui::RichText::new(kind.namn()).strong().size(16.0));
                ui.label(format!("Ruta {}, {}", tile.0, tile.1));
                ui.separator();
                ui.label(kind.beskrivning());
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui
                        .button(format!("Riv (+{:.0} guld)", kind.kostnad() * 0.5))
                        .clicked()
                    {
                        riv = Some(tile);
                    }
                    if ui.button("Stäng").clicked() {
                        markerad = None;
                    }
                });
            });
    }

    // --- statusrad ----------------------------------------------------
    egui::Panel::bottom("status").show(ui_root, |ui| {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(&stad.status).weak());
        });
    });

    let mut stad = world.resource_mut::<Stad>();
    stad.vald = vald;
    stad.markerad = markerad;
    drop(stad);
    world.insert_resource(UiKommando { riv });
}

fn nyckeltal(ui: &mut egui::Ui, etikett: &str, varde: String) {
    ui.vertical(|ui| {
        ui.label(egui::RichText::new(etikett).small().weak());
        ui.label(egui::RichText::new(varde).strong());
    });
    ui.add_space(10.0);
}
