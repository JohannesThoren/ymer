//! Spelets gränssnitt: HUD på toppen, byggpalett till vänster och en
//! panel för den markerade byggnaden.
//!
//! Siffrorna kommer ur `Stadskassa`, som skriptet fyller i. UI:t räknar
//! alltså aldrig ut något själv – ändras en regel i `ekonomi.ts` följer
//! HUD:en med utan att den här filen rörs.

use bevy_ecs::prelude::*;

use crate::{Byggnad, Kind, Val, kassa};

/// Vad spelaren bad om i UI:t och som spelet måste verkställa efteråt.
/// `with_ui` lånar världen, så rivning kan inte ske mitt i ritandet.
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct UiKommando {
    pub riv: Option<crate::Tile>,
}

pub fn rita(ui_root: &mut egui::Ui, world: &mut World) {
    let k = kassa(world);
    let val = world.resource::<Val>().clone();
    let antal = {
        let mut query = world.query::<&Byggnad>();
        query.iter(world).count()
    };
    let markerad_byggnad = val
        .markerad
        .and_then(|tile| crate::byggnad_pa(world, tile).map(|(_, b)| b));

    let mut vald = val.vald;
    let mut markerad = val.markerad;
    let mut riv: Option<crate::Tile> = None;

    // --- HUD ----------------------------------------------------------
    egui::Panel::top("hud").show(ui_root, |ui| {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.heading("Staden");
            ui.separator();

            nyckeltal(ui, "Guld", format!("{:.0}", k.guld));
            nyckeltal(
                ui,
                "Invånare",
                format!("{:.0} / {}", k.invanare.floor(), k.platser),
            );
            nyckeltal(ui, "Jobb", format!("{} / {}", k.sysselsatta, k.jobb));
            nyckeltal(ui, "Inkomst", format!("{:.1} guld/s", k.inkomst));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(egui::RichText::new(format!("{antal} byggnader")).weak());
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
                let har_rad = k.guld >= kind.kostnad();
                let knapp =
                    egui::Button::new(format!("{}  –  {:.0} guld", kind.namn(), kind.kostnad()))
                        .selected(kind == vald);

                // Går att välja även utan råd, så att man kan läsa
                // beskrivningen och se vad man sparar till.
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
                    "Klicka på marken för att bygga.\nKlicka på ett hus för att markera det.\n\nReglerna ligger i scripts/ekonomi.ts.",
                )
                .small()
                .weak(),
            );
        });

    // --- markerad byggnad ---------------------------------------------
    if let (Some(tile), Some(b)) = (markerad, markerad_byggnad) {
        egui::Window::new("Byggnad")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::RIGHT_BOTTOM, [-16.0, -48.0])
            .show(ui_root.ctx(), |ui| {
                ui.label(egui::RichText::new(b.kind.namn()).strong().size(16.0));
                ui.label(format!("Ruta {}, {}", tile.0, tile.1));
                ui.separator();

                // Siffrorna är skriptets, inte Rusts.
                egui::Grid::new("byggnadsdata").show(ui, |ui| {
                    ui.label("Platser");
                    ui.label(format!("{}", b.platser));
                    ui.end_row();
                    ui.label("Jobb");
                    ui.label(format!("{} / {}", b.bemanning, b.jobb));
                    ui.end_row();
                    ui.label("Inkomst");
                    let andel = if b.jobb == 0 {
                        0.0
                    } else {
                        b.inkomst * b.bemanning as f32 / b.jobb as f32
                    };
                    ui.label(format!("{andel:.1} av {:.1} guld/s", b.inkomst));
                    ui.end_row();
                });

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui
                        .button(format!("Riv (+{:.0} guld)", b.kind.kostnad() * 0.5))
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
            ui.label(egui::RichText::new(&val.status).weak());
        });
    });

    let mut val = world.resource_mut::<Val>();
    val.vald = vald;
    val.markerad = markerad;
    drop(val);
    world.insert_resource(UiKommando { riv });
}

fn nyckeltal(ui: &mut egui::Ui, etikett: &str, varde: String) {
    ui.vertical(|ui| {
        ui.label(egui::RichText::new(etikett).small().weak());
        ui.label(egui::RichText::new(varde).strong());
    });
    ui.add_space(10.0);
}
