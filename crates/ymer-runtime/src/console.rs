//! Debug-konsolen. Toggla med `` ` `` (backtick/grave). Tillgänglig i alla
//! spel byggda på `App`, inte bara i editorn – `games/sandbox` får den
//! gratis så fort ett projekt aktiverar den med `App::with_debug_console`.
//!
//! Två saker hålls medvetet isär: `ConsoleLog` (i `ymer-core`) är bara
//! en delad buffert som `console.log` i skript skriver till – den finns
//! oavsett om någon UI visar den. Den här modulen är UI:t plus
//! kommandoexekveringen ovanpå.

use bevy_ecs::prelude::*;
use winit::event::{ElementState, WindowEvent};
use winit::keyboard::Key;
use ymer_core::{ConsoleLog, LogLevel};
use ymer_render::EguiOverlay;
use ymer_scene::TypeRegistry;

/// Vad konsolen behöver för att kunna köra kommandon. Utan den här visas
/// loggen ändå, men indata i inputfältet ger bara ett felmeddelande –
/// ett spel som inte bryr sig om skript ska inte tvingas bära en
/// TypeRegistry bara för att konsolen finns.
pub struct ConsoleScripting {
    pub wasm: Vec<u8>,
    pub registry: TypeRegistry,
}

pub struct DebugConsole {
    open: bool,
    input: String,
    history: Vec<String>,
    history_cursor: Option<usize>,
    scripting: Option<ConsoleScripting>,
    overlay: EguiOverlay,
    ctx: egui::Context,
    egui_winit: egui_winit::State,
    pixels_per_point: f32,
}

impl DebugConsole {
    pub fn new(
        device: &ymer_render::wgpu::Device,
        output_format: ymer_render::wgpu::TextureFormat,
        window: &winit::window::Window,
        scripting: Option<ConsoleScripting>,
    ) -> Self {
        let ctx = egui::Context::default();
        ctx.set_visuals(egui::Visuals::dark());

        Self {
            open: false,
            input: String::new(),
            history: Vec::new(),
            history_cursor: None,
            scripting,
            overlay: EguiOverlay::new(device, output_format),
            egui_winit: egui_winit::State::new(
                ctx.clone(),
                egui::ViewportId::ROOT,
                window,
                None,
                None,
                None,
            ),
            ctx,
            pixels_per_point: 1.0,
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Konsolen får se eventet före spelet. Returnerar `true` om den tog
    /// hand om det – spelet ska då inte reagera på samma tangenttryck
    /// (annars flyttar sig spelaren medan man skriver ett kommando).
    pub fn handle_window_event(
        &mut self,
        window: &winit::window::Window,
        event: &WindowEvent,
    ) -> bool {
        // Backtick öppnar/stänger alltid, även om egui inte har fokus än.
        if let WindowEvent::KeyboardInput {
            event: key_event, ..
        } = event
            && key_event.state == ElementState::Pressed
            && !key_event.repeat
            && matches!(&key_event.logical_key, Key::Character(c) if c == "`" || c == "§")
        {
            self.open = !self.open;
            return true;
        }

        if !self.open {
            return false;
        }

        let response = self.egui_winit.on_window_event(window, event);
        response.consumed
    }

    /// Bygger UI:t för framen. `dt` behövs inte av konsolen själv men
    /// hela `World` gör – kommandon kan behöva den.
    pub fn update(&mut self, window: &winit::window::Window, world: &mut World, size: (u32, u32)) {
        if !self.open {
            return;
        }

        let raw_input = self.egui_winit.take_egui_input(window);

        let mut submit: Option<String> = None;
        let mut history_delta = 0i32;

        let entries: Vec<(LogLevel, String, String)> = world
            .get_resource::<ConsoleLog>()
            .map(|log| {
                log.entries()
                    .map(|e| (e.level, e.message.clone(), e.source.clone()))
                    .collect()
            })
            .unwrap_or_default();

        let has_scripting = self.scripting.is_some();
        let input_field_id = egui::Id::new("debug_console_input");

        let mut output = self.ctx.run_ui(raw_input, |ui| {
            egui::Panel::top("debug_console")
                .default_size(320.0)
                .resizable(true)
                .show(ui, |ui| {
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Debug-konsol").strong());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(if has_scripting {
                                    "kommandon aktiverade"
                                } else {
                                    "bara loggvisning – ingen scripting konfigurerad"
                                })
                                .weak()
                                .small(),
                            );
                        });
                    });
                    ui.separator();

                    egui::ScrollArea::vertical()
                        .max_height(220.0)
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            for (level, message, source) in &entries {
                                let color = match level {
                                    LogLevel::Error => egui::Color32::from_rgb(240, 90, 90),
                                    LogLevel::Warn => egui::Color32::from_rgb(230, 190, 80),
                                    LogLevel::Info => egui::Color32::from_rgb(120, 180, 240),
                                    LogLevel::Log => ui.visuals().text_color(),
                                };
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new(format!("[{source}]"))
                                            .monospace()
                                            .weak()
                                            .small(),
                                    );
                                    ui.label(egui::RichText::new(message).monospace().color(color));
                                });
                            }
                        });

                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label(">");
                        let field = ui.add(
                            egui::TextEdit::singleline(&mut self.input)
                                .id(input_field_id)
                                .desired_width(f32::INFINITY)
                                .hint_text("engine.find(\"Player\") ... – Enter kör, ↑/↓ bläddrar historik"),
                        );

                        // Enter skickar oavsett fokusstatus: fältet är den enda
                        // interaktiva widgeten i konsolen, så "vilken widget har
                        // fokus" är aldrig tvetydigt. lost_focus()-idiomet som
                        // egui själv rekommenderar visade sig opålitligt i denna
                        // testmiljö (syntetict tangentbord via XTest) – troligen
                        // en kapplöpning mellan intern fokusövergång och
                        // XTest-eventens leverans, snarare än ett fel i widgeten.
                        if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            submit = Some(self.input.clone());
                        }
                        if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                            history_delta = -1;
                        }
                        if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                            history_delta = 1;
                        }
                        // Pinnad varje frame: det finns inget annat att
                        // fokusera i konsolen, så tvetydigheten är noll.
                        field.request_focus();
                    });
                });
        });

        self.pixels_per_point = self.ctx.pixels_per_point();
        self.egui_winit
            .handle_platform_output(window, std::mem::take(&mut output.platform_output));
        self.overlay.accept(&self.ctx, output, size);

        if history_delta != 0 {
            self.navigate_history(history_delta);
        }

        if let Some(command) = submit {
            self.input.clear();
            if !command.trim().is_empty() {
                self.run_command(world, &command);
                self.history.push(command);
                self.history_cursor = None;
            }
        }
    }

    fn navigate_history(&mut self, delta: i32) {
        if self.history.is_empty() {
            return;
        }
        let next = match self.history_cursor {
            None if delta < 0 => self.history.len() - 1,
            Some(index) => index
                .saturating_add_signed(delta as isize)
                .min(self.history.len() - 1),
            None => return,
        };
        self.history_cursor = Some(next);
        self.input = self.history[next].clone();
    }

    fn run_command(&mut self, world: &mut World, command: &str) {
        // Ekot skrivs oavsett scripting, så man ser vad man körde.
        if let Some(mut log) = world.get_resource_mut::<ConsoleLog>() {
            log.push(LogLevel::Log, format!("> {command}"), "console");
        }

        let Some(scripting) = &self.scripting else {
            if let Some(mut log) = world.get_resource_mut::<ConsoleLog>() {
                log.push(
                    LogLevel::Error,
                    "ingen scripting konfigurerad – App::with_debug_console saknar den",
                    "console",
                );
            }
            return;
        };

        let result =
            ymer_script::run_console_command(world, &scripting.wasm, &scripting.registry, command);
        if let Err(err) = result
            && let Some(mut log) = world.get_resource_mut::<ConsoleLog>()
        {
            log.push(LogLevel::Error, format!("{err:#}"), "console");
        }
    }

    /// Bara det renderaren behöver – anropas alltid, oavsett om konsolen
    /// är öppen, så att overlayen kan rensa sin egen textur-delta-kö.
    pub fn overlay_mut(&mut self) -> &mut EguiOverlay {
        &mut self.overlay
    }
}
