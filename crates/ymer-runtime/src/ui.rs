//! Spelets eget gränssnitt.
//!
//! Debug-konsolen är motorns UI; det här är spelets. Ett spel som vill ha
//! en HUD ger `App::with_ui` en stängning som ritar den, och får en
//! `egui::Context` och hela `World` varje frame:
//!
//! ```ignore
//! App::new(config)
//!     .with_ui(|ui, world| {
//!         let guld = world.resource::<Kassa>().guld;
//!         egui::Panel::top("hud").show(ui, |ui| ui.label(format!("{guld} guld")));
//!     })
//!     .run()
//! ```
//!
//! Konsolen och spelets UI delar inte `egui::Context`. De behöver inte
//! det heller: konsolen tar all input när den är öppen, och då ritas den
//! i stället för HUD:en. Ett spel kan alltså inte visa båda samtidigt,
//! vilket är rimligt för en debug-overlay som lägger sig över allt.

use bevy_ecs::prelude::*;
use ymer_render::EguiOverlay;

/// Vem som äger pekaren och tangentbordet den här framen.
///
/// Utan den här skulle ett klick på en knapp i HUD:en *också* nå spelet,
/// så att man placerade ett hus under panelen man just tryckte på.
/// Resursen finns alltid när `with_ui` används; ett spel läser den innan
/// det tolkar musklick.
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct UiFocus {
    /// egui är under muspekaren – spelet ska ignorera klick.
    pub pointer: bool,
    /// egui läser tangentbordet, t.ex. ett textfält med fokus.
    pub keyboard: bool,
}

impl UiFocus {
    /// Får spelet reagera på musen den här framen?
    pub fn game_has_pointer(&self) -> bool {
        !self.pointer
    }
}

/// Stängningen får ett `Ui` som täcker hela fönstret. Paneler visas i
/// det (`Panel::top(..).show(ui, ..)`); fönster och popup:er vill ha en
/// `Context` i stället, och den når man med `ui.ctx()`.
pub type UiFn = Box<dyn FnMut(&mut egui::Ui, &mut World)>;

pub struct GameUi {
    overlay: EguiOverlay,
    ctx: egui::Context,
    egui_winit: egui_winit::State,
}

impl GameUi {
    pub fn new(
        device: &ymer_render::wgpu::Device,
        output_format: ymer_render::wgpu::TextureFormat,
        window: &winit::window::Window,
    ) -> Self {
        let ctx = egui::Context::default();
        ctx.set_visuals(egui::Visuals::dark());

        Self {
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
        }
    }

    /// Låter egui se eventet. Returnerar `true` om det togs om hand –
    /// spelet ska då inte se samma klick eller tangenttryck.
    pub fn handle_window_event(
        &mut self,
        window: &winit::window::Window,
        event: &winit::event::WindowEvent,
    ) -> bool {
        self.egui_winit.on_window_event(window, event).consumed
    }

    /// Ritar framens UI och uppdaterar `UiFocus` i världen.
    pub fn update(
        &mut self,
        window: &winit::window::Window,
        world: &mut World,
        size: (u32, u32),
        ui: &mut UiFn,
    ) {
        let raw_input = self.egui_winit.take_egui_input(window);

        // run_ui lånar världen i stängningen, så den kan inte också nås
        // härifrån – därför läses fokus ut efteråt, ur kontexten.
        let output = self.ctx.run_ui(raw_input, |egui_ui| ui(egui_ui, world));

        self.egui_winit
            .handle_platform_output(window, output.platform_output.clone());

        // `egui_wants_pointer_input` svarar nej så fort en knapp hålls
        // nere, för att dragningar som börjat utanför ska få fortsätta.
        // För ett klick-att-bygga-spel är det fel håll att gissa åt, så
        // "pekaren är över egui" räknas också som upptaget.
        world.insert_resource(UiFocus {
            pointer: self.ctx.is_pointer_over_egui() || self.ctx.egui_wants_pointer_input(),
            keyboard: self.ctx.egui_wants_keyboard_input(),
        });

        self.overlay.accept(&self.ctx, output, size);
    }

    pub fn overlay_mut(&mut self) -> &mut dyn ymer_render::Overlay {
        &mut self.overlay
    }
}
