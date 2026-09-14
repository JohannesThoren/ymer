//! Limmet: äger fönstret, ECS-världen, renderaren och spelloopen.

use std::sync::Arc;

use bevy_ecs::prelude::*;
use bevy_ecs::schedule::{IntoScheduleConfigs, ScheduleLabel};
use bevy_ecs::system::ScheduleSystem;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};
use ymer_core::{
    Camera, GlobalTransform, Input, Mat4, MeshInstance, Time, Vec2, Vec3, propagate_transforms,
};
use ymer_render::{Assets, DrawItem, RenderList, Renderer};

pub mod console;
/// Demoscen för exempel och tester. Inte en del av motorn – bakom en
/// feature-flagga så att den inte följer med i bibliotek eller spel.
#[cfg(feature = "demo")]
pub mod demo;
pub mod scripts;
pub use scripts::ScriptHost;

pub mod prelude {
    pub use crate::{App, AppConfig, Update, build_render_list};
    pub use bevy_ecs::prelude::*;
    pub use ymer_core::{
        Camera, Color, GlobalTransform, Mat4, MeshId, MeshInstance, Quat, Time, Transform, Vec3,
        propagate_transforms,
    };
    pub use ymer_render::{MeshData, Renderer, primitives};
}

/// Schemat som körs en gång per frame, före rendering.
#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Update;

pub struct AppConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            title: "engine".into(),
            width: 1280,
            height: 720,
        }
    }
}

type SetupFn = Box<dyn FnOnce(&mut World, &mut Renderer, &mut Assets)>;

/// Byggs upp innan event-loopen startar; fönster och GPU skapas först i `resumed`.
pub struct App {
    config: AppConfig,
    world: World,
    schedule: Schedule,
    setup: Option<SetupFn>,
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    assets: Assets,
    console_enabled: bool,
    console_scripting: Option<console::ConsoleScripting>,
    console: Option<console::DebugConsole>,
    scripts: Option<scripts::ScriptHost>,
    script_registry: Option<ymer_scene::TypeRegistry>,
}

impl App {
    pub fn new(config: AppConfig) -> Self {
        let mut world = World::new();
        world.insert_resource(Time::new());
        world.insert_resource(Input::default());
        world.insert_resource(ymer_core::ConsoleLog::default());
        world.insert_resource(ymer_core::Gravity::default());

        let mut schedule = Schedule::new(Update);
        // Hierarkin måste räknas om efter att spelsystemen flyttat saker.
        // Sprite-animationen kör före, så att rutbytet syns samma frame.
        // Fysiken före hierarkin, så att GlobalTransform räknas om med
        // de slutliga positionerna.
        schedule.add_systems(
            (
                ymer_core::animate_sprites,
                ymer_core::step_physics,
                propagate_transforms,
            )
                .chain(),
        );

        Self {
            config,
            world,
            schedule,
            setup: None,
            window: None,
            renderer: None,
            assets: Assets::default(),
            console_enabled: false,
            console_scripting: None,
            console: None,
            scripts: None,
            script_registry: None,
        }
    }

    /// Scenbygge som körs när GPU:n finns – meshes måste laddas upp någonstans.
    pub fn with_setup(
        mut self,
        setup: impl FnOnce(&mut World, &mut Renderer, &mut Assets) + 'static,
    ) -> Self {
        self.setup = Some(Box::new(setup));
        self
    }

    pub fn add_systems<M>(mut self, systems: impl IntoScheduleConfigs<ScheduleSystem, M>) -> Self {
        self.schedule.add_systems(systems);
        self
    }

    /// Kör TypeScript-skript varje frame. Skripten hämtas från värden,
    /// som i sin tur läser från disk eller ur ett arkiv.
    pub fn with_scripts(
        mut self,
        host: scripts::ScriptHost,
        registry: ymer_scene::TypeRegistry,
    ) -> Self {
        self.scripts = Some(host);
        self.script_registry = Some(registry);
        self
    }

    /// Aktiverar debug-konsolen: `` ` `` togglar en overlay som visar
    /// `console.log` från skript. Utan `with_console_scripting` visar den
    /// bara loggen – kommandoraden svarar med ett felmeddelande.
    pub fn with_debug_console(mut self) -> Self {
        self.console_enabled = true;
        self
    }

    /// Som `with_debug_console`, men kommandoraden kan också köra kod:
    /// varje rad blir en engångs-QuickJS-körning med samma `engine.*`-API
    /// som vanliga skript. `wasm` är innehållet i `script_host.wasm`.
    pub fn with_console_scripting(
        mut self,
        wasm: Vec<u8>,
        registry: ymer_scene::TypeRegistry,
    ) -> Self {
        self.console_enabled = true;
        self.console_scripting = Some(console::ConsoleScripting { wasm, registry });
        self
    }

    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    pub fn run(mut self) -> anyhow::Result<()> {
        let event_loop = EventLoop::new()?;
        event_loop.set_control_flow(ControlFlow::Poll);
        event_loop.run_app(&mut self)?;
        Ok(())
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attributes = Window::default_attributes()
            .with_title(self.config.title.clone())
            .with_inner_size(LogicalSize::new(self.config.width, self.config.height));

        let window = match event_loop.create_window(attributes) {
            Ok(window) => Arc::new(window),
            Err(err) => {
                log::error!("kunde inte skapa fönster: {err}");
                event_loop.exit();
                return;
            }
        };

        let mut renderer = match pollster::block_on(Renderer::new_windowed(window.clone())) {
            Ok(renderer) => renderer,
            Err(err) => {
                log::error!("kunde inte initiera renderaren: {err:#}");
                event_loop.exit();
                return;
            }
        };

        self.assets = Assets::new(&mut renderer);

        if let Some(setup) = self.setup.take() {
            setup(&mut self.world, &mut renderer, &mut self.assets);
        }

        if self.console_enabled {
            self.console = Some(console::DebugConsole::new(
                renderer.device(),
                renderer.output_format(),
                &window,
                self.console_scripting.take(),
            ));
        }

        self.renderer = Some(renderer);
        self.window = Some(window);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        // Konsolen ser eventet först. Öppen eller ej, den kan konsumera
        // (backtick togglar alltid; när öppen tar den all input så att
        // spelet inte rör sig medan man skriver ett kommando).
        if let Some(window) = self.window.clone()
            && let Some(console) = &mut self.console
            && console.handle_window_event(&window, &event)
        {
            window.request_redraw();
            return;
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    // KeyCode:s Debug-namn är samma som webbens KeyboardEvent.code.
                    let name = format!("{code:?}");
                    let mut input = self.world.resource_mut::<Input>();
                    match event.state {
                        ElementState::Pressed => input.press(name),
                        ElementState::Released => input.release(name),
                    }
                }
                if let KeyEvent {
                    physical_key: PhysicalKey::Code(KeyCode::Escape),
                    state: ElementState::Pressed,
                    ..
                } = event
                {
                    event_loop.exit();
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                let position = Vec2::new(position.x as f32, position.y as f32);
                self.world
                    .resource_mut::<Input>()
                    .set_mouse_position(position);
            }

            WindowEvent::MouseInput { state, button, .. } => {
                let name = format!("{button:?}");
                let mut input = self.world.resource_mut::<Input>();
                match state {
                    ElementState::Pressed => input.press_mouse(name),
                    ElementState::Released => input.release_mouse(&name),
                }
            }

            WindowEvent::Resized(size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(size.width, size.height);
                }
            }

            WindowEvent::RedrawRequested => {
                if let Some(mut time) = self.world.get_resource_mut::<Time>() {
                    time.tick();
                }
                // Skripten körs före hierarkipropageringen i schemat, så
                // att det de flyttar syns samma frame.
                if let (Some(host), Some(registry)) = (&mut self.scripts, &self.script_registry) {
                    let dt = self.world.resource::<Time>().delta_seconds();
                    if let Err(err) = host.tick(&mut self.world, registry, dt) {
                        log::error!("skriptfel: {err:#}");
                    }
                }

                // Schemat kör efteråt och propagerar hierarkin, så att
                // det skripten flyttade syns samma frame.
                self.schedule.run(&mut self.world);
                self.world.resource_mut::<Input>().end_frame();

                if let Some(renderer) = &mut self.renderer {
                    let list =
                        build_render_list(&mut self.world, renderer.aspect_ratio(), &self.assets);

                    let render_result = match (&mut self.console, &self.window) {
                        (Some(console), Some(window)) if console.is_open() => {
                            console.update(window, &mut self.world, renderer.size());
                            renderer.render_with_overlay(&list, Some(console.overlay_mut()))
                        }
                        _ => renderer.render(&list),
                    };

                    if let Err(err) = render_result {
                        // Milstolpe 2 återskapar inte surfacen ännu.
                        log::error!("renderfel: {err}");
                        event_loop.exit();
                    }
                }
            }

            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

/// Plockar ut allt ritbart ur världen. Det här är gränssnittet mellan ECS
/// och GPU – renderaren ser aldrig en `World`.
pub fn build_render_list(world: &mut World, aspect: f32, assets: &Assets) -> RenderList {
    let mut list = RenderList::default();

    let camera = {
        let mut query = world.query::<(&Camera, &GlobalTransform)>();
        query
            .iter(world)
            .next()
            .map(|(camera, global)| (*camera, *global))
    };

    match camera {
        Some((camera, global)) => {
            // Vyn är kamerans världstransform inverterad.
            list.view_proj = camera.projection(aspect) * global.0.inverse();
        }
        None => log::warn!("ingen kamera i världen – renderar från origo"),
    }

    let mut query = world.query::<(&MeshInstance, &GlobalTransform)>();
    for (instance, global) in query.iter(world) {
        list.items.push(DrawItem::new(
            // Namnen slås upp här, en gång per frame och entitet.
            assets.mesh(&instance.mesh),
            assets.texture(&instance.texture),
            global.0,
            instance.color,
        ));
    }

    // --- sprites ---------------------------------------------------------
    // Kvadraten är 1x1, så storleken blir en skalning ovanpå entitetens
    // egen transform. Det gör att en sprite kan vara barn till något och
    // ärva rotation som vanligt.
    let quad = assets.mesh(ymer_core::BUILTIN_QUAD);
    let camera_position = camera
        .map(|(_, global)| global.translation())
        .unwrap_or(Vec3::ZERO);

    let mut sprites: Vec<(f32, DrawItem)> = Vec::new();
    let mut query = world.query::<(&ymer_core::Sprite, &GlobalTransform)>();
    for (sprite, global) in query.iter(world) {
        let scale = Mat4::from_scale(Vec3::new(sprite.size.x, sprite.size.y, 1.0));
        let transform = global.0 * scale;

        let distance = (global.translation() - camera_position).length_squared();
        sprites.push((
            distance,
            DrawItem {
                mesh: quad,
                texture: assets.texture(&sprite.texture),
                transform,
                color: sprite.color,
                uv_transform: sprite.uv_transform(),
            },
        ));
    }

    // Bakifrån och fram: genomskinlighet blandas bara rätt om det som är
    // längst bort ritas först. Sorteringen sker här, inte i renderaren,
    // eftersom det är här kamerans position är känd.
    sprites.sort_by(|a, b| b.0.total_cmp(&a.0));
    list.sprite_items = sprites.into_iter().map(|(_, item)| item).collect();

    list
}

/// Initierar loggning; `RUST_LOG=info` som standard.
pub fn init_logging() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
}
