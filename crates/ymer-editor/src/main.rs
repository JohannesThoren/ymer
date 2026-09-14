//! Editorn. Startar i launchern, öppnar ett projekt, och redigerar det.
//!
//!     cargo run -p ymer-editor --bin ymer
//!     cargo run -p ymer-editor --bin ymer -- projects/demo-spel-1

use std::path::Path;
use std::sync::Arc;

use bevy_ecs::prelude::*;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};
use ymer_core::{GlobalTransform, Input, MeshId, Time, Vec3};
use ymer_editor::browser::FileBrowser;
use ymer_editor::gizmo::{self, Drag};
use ymer_editor::launcher::{self, LauncherState};
use ymer_editor::project::{self, ProjectHandle};
use ymer_editor::{EditorState, EguiOverlay, PlayMode, run_ui};
use ymer_render::{Assets, Renderer};
use ymer_runtime::{Update, build_render_list};
use ymer_scene::{Scene, TypeRegistry, clear_scene, register_builtin_types};

/// Var script_host.wasm ligger. Den hör till motorn, inte till projektet.
const SCRIPT_HOST: &str = "assets/script_host.wasm";
const PROJECTS_DIR: &str = "projects";

enum Mode {
    Launcher,
    Editing,
}

struct Editor {
    mode: Mode,
    launcher: LauncherState,
    world: World,
    schedule: Schedule,
    registry: TypeRegistry,
    state: EditorState,
    egui_ctx: egui::Context,
    egui_winit: Option<egui_winit::State>,
    overlay: Option<EguiOverlay>,
    renderer: Option<Renderer>,
    window: Option<Arc<Window>>,
    assets: Assets,
    gizmo_mesh: MeshId,
    drag: Option<Drag>,
    cursor: (f32, f32),
    play: Option<PlayMode>,
    was_playing: bool,
}

impl Editor {
    fn new() -> Self {
        let mut registry = TypeRegistry::new();
        register_builtin_types(&mut registry);

        let mut world = World::new();
        world.insert_resource(Time::new());
        world.insert_resource(Input::default());
        world.insert_resource(ymer_core::ConsoleLog::default());

        Self {
            mode: Mode::Launcher,
            launcher: LauncherState::new(PROJECTS_DIR),
            world,
            schedule: Schedule::new(Update),
            registry,
            state: EditorState::default(),
            egui_ctx: egui::Context::default(),
            egui_winit: None,
            overlay: None,
            renderer: None,
            window: None,
            assets: Assets::default(),
            gizmo_mesh: MeshId(0),
            drag: None,
            cursor: (0.0, 0.0),
            play: None,
            was_playing: false,
        }
    }

    fn refresh_mesh_names(&mut self) {
        self.state.available_meshes = self.assets.mesh_names().map(str::to_string).collect();
    }

    /// Laddar projektets startscen, texturer och skriptkatalog.
    fn open_project(&mut self, handle: ProjectHandle) {
        // Inte clear_entities: den raderar resurserna med.
        clear_scene(&mut self.world, &self.registry);

        // Texturer namnges efter sin sökväg relativt projektroten.
        if let Some(renderer) = self.renderer.as_mut() {
            let count = self
                .assets
                .load_textures(renderer, &handle.root, &handle.path("textures"));
            log::info!("{count} texturer laddade");

            // Modeller registreras om varje gång projektet öppnas – annars
            // pekar en scen på ett meshnamn som inte finns i det här körda
            // programmet, eftersom mesh-handtag inte överlever en omstart.
            let mut model_count = 0;
            for path in find_models(&handle.root) {
                if ymer_editor::gltf_import::reload(renderer, &mut self.assets, &handle.root, &path)
                    .is_ok()
                {
                    model_count += 1;
                }
            }
            log::info!("{model_count} modeller laddade");
            self.refresh_mesh_names();
        }

        match Scene::load(handle.start_scene()).and_then(|scene| {
            scene
                .spawn_into(&mut self.world, &self.registry)
                .map(|m| m.len())
        }) {
            Ok(count) => self.state.status = format!("laddade {count} entiteter"),
            Err(err) => self.state.status = format!("{err}"),
        }

        // Skriv om typdeklarationerna varje gång projektet öppnas, så att
        // editorns autocomplete alltid matchar registret.
        if let Err(err) = write_typescript_support(&handle, &self.registry) {
            log::warn!("kunde inte skriva typdefinitioner: {err:#}");
        }

        self.play = PlayMode::new(SCRIPT_HOST, handle.scripts_dir())
            .map_err(|err| log::error!("play-läget avstängt: {err:#}"))
            .ok();

        self.state.open_project(handle);
        self.state.selected = None;
        self.mode = Mode::Editing;
    }
}

impl ApplicationHandler for Editor {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attributes = Window::default_attributes()
            .with_title("engine editor")
            .with_inner_size(LogicalSize::new(1600, 900));

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

        // Inbyggda meshar registreras med namn; scenfiler refererar till
        // "builtin/cube" och "builtin/plane", aldrig till index.
        self.assets = Assets::new(&mut renderer);
        self.gizmo_mesh = self.assets.mesh(ymer_core::BUILTIN_CUBE);

        self.egui_ctx.set_visuals(egui::Visuals::dark());
        self.egui_winit = Some(egui_winit::State::new(
            self.egui_ctx.clone(),
            egui::ViewportId::ROOT,
            &window,
            None,
            None,
            None,
        ));
        self.overlay = Some(EguiOverlay::new(
            renderer.device(),
            renderer.output_format(),
        ));
        self.renderer = Some(renderer);
        self.window = Some(window);

        // Projekt angivet på kommandoraden hoppar över launchern.
        if let Some(path) = std::env::args().nth(1) {
            match project::open(&path) {
                Ok(handle) => self.open_project(handle),
                Err(err) => log::error!("kunde inte öppna {path}: {err:#}"),
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(window) = self.window.clone() else {
            return;
        };
        let response = match self.egui_winit.as_mut() {
            Some(state) => state.on_window_event(&window, &event),
            None => return,
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            // Filer som släpps på fönstret kopieras in i projektet. En
            // glTF/glb triggar dessutom import: geometri och material
            // laddas upp och en prefab skrivs, så filen är genast
            // användbar från utforskaren.
            WindowEvent::DroppedFile(path) => {
                let Some(browser) = self.state.browser.as_mut() else {
                    return;
                };
                let target = match browser.import(&path) {
                    Ok(target) => target,
                    Err(err) => {
                        browser.status = format!("{err}");
                        return;
                    }
                };

                // Visa mappen filen faktiskt hamnade i – annars ser det ut
                // som att inget hände.
                if let Some(folder) = target.parent().map(Path::to_path_buf) {
                    browser.enter(&folder);
                }

                if is_gltf(&target)
                    && let (Some(renderer), Some(handle)) =
                        (self.renderer.as_mut(), self.state.project.clone())
                {
                    match ymer_editor::gltf_import::import_as_prefab(
                        renderer,
                        &mut self.assets,
                        &handle.root,
                        &target,
                    ) {
                        Ok(prefab) => {
                            self.state.browser.as_mut().unwrap().status =
                                format!("importerade {} -> {}", target.display(), prefab.display());
                        }
                        Err(err) => {
                            self.state.browser.as_mut().unwrap().status = format!("{err}");
                        }
                    }
                    self.refresh_mesh_names();
                } else {
                    self.state.browser.as_mut().unwrap().status =
                        format!("importerade {}", target.display());
                }
            }

            WindowEvent::KeyboardInput { event, .. } => {
                if let winit::keyboard::PhysicalKey::Code(code) = event.physical_key {
                    let name = format!("{code:?}");
                    let mut input = self.world.resource_mut::<Input>();
                    match event.state {
                        ElementState::Pressed => input.press(name),
                        ElementState::Released => input.release(name),
                    }
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = (position.x as f32, position.y as f32);
                if let (Some(drag), Some(renderer)) = (self.drag, self.renderer.as_ref()) {
                    let aspect = renderer.aspect_ratio();
                    let size = renderer.size();
                    if let Some(view_proj) = gizmo::camera_view_proj(&mut self.world, aspect) {
                        let ray = gizmo::screen_ray(view_proj, self.cursor, size);
                        gizmo::update_drag(&mut self.world, &drag, &ray);
                    }
                }
            }

            WindowEvent::MouseInput { state, button, .. } => {
                let over_ui = response.consumed;
                match (state, button) {
                    (ElementState::Pressed, winit::event::MouseButton::Left) if !over_ui => {
                        // Lånen tas per fält – metoder på &mut self skulle krocka
                        // med fönsterlånet ovanför.
                        let Some(renderer) = self.renderer.as_ref() else {
                            return;
                        };
                        let (aspect, size) = (renderer.aspect_ratio(), renderer.size());

                        let Some(view_proj) = gizmo::camera_view_proj(&mut self.world, aspect)
                        else {
                            return;
                        };
                        let ray = gizmo::screen_ray(view_proj, self.cursor, size);

                        let camera = {
                            let mut query =
                                self.world.query::<(&ymer_core::Camera, &GlobalTransform)>();
                            query
                                .iter(&self.world)
                                .next()
                                .map(|(_, g)| g.translation())
                                .unwrap_or(Vec3::ZERO)
                        };

                        // Handtagen har företräde framför objekten bakom dem.
                        let grabbed = self.state.selected.and_then(|entity| {
                            gizmo::begin_drag(&mut self.world, entity, &ray, camera)
                        });

                        if grabbed.is_some() {
                            self.drag = grabbed;
                        } else {
                            let meshes = &self.renderer.as_ref().unwrap().meshes;
                            self.state.selected =
                                gizmo::pick(&mut self.world, meshes, &self.assets, &ray);
                        }
                    }
                    (ElementState::Released, winit::event::MouseButton::Left) => {
                        self.drag = None;
                    }
                    _ => {}
                }
            }

            WindowEvent::Resized(size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(size.width, size.height);
                }
            }

            WindowEvent::RedrawRequested => {
                let (Some(renderer), Some(overlay)) = (&mut self.renderer, &mut self.overlay)
                else {
                    return;
                };
                let Some(egui_winit) = self.egui_winit.as_mut() else {
                    return;
                };
                let raw_input = egui_winit.take_egui_input(&window);

                self.world.resource_mut::<Time>().tick();
                let dt = self.world.resource::<Time>().delta_seconds();

                let mut opened: Option<ProjectHandle> = None;

                let mut output = match self.mode {
                    Mode::Launcher => {
                        let (output, chosen) =
                            launcher::run_ui(&self.egui_ctx, raw_input, &mut self.launcher);
                        opened = chosen;
                        output
                    }
                    Mode::Editing => {
                        if self.state.playing != self.was_playing {
                            self.was_playing = self.state.playing;
                            if let Some(play) = &mut self.play {
                                let result = if self.state.playing {
                                    play.start(&mut self.world, &self.registry)
                                        .map(|count| format!("spelar – {count} skript laddade"))
                                } else {
                                    play.stop(&mut self.world, &self.registry)
                                        .map(|()| "stoppad, scenen återställd".to_string())
                                };
                                match result {
                                    Ok(message) => self.state.status = message,
                                    Err(err) => {
                                        self.state.status = format!("{err}");
                                        self.state.playing = false;
                                        self.was_playing = false;
                                    }
                                }
                            }
                        }

                        if self.state.playing {
                            self.schedule.run(&mut self.world);

                            if let Some(play) = &mut self.play {
                                for result in play.poll_reloads(false) {
                                    self.state.status = match result {
                                        Ok(name) => format!("{name} omladdad"),
                                        Err(err) => format!("{err}"),
                                    };
                                }
                            }

                            if let Some(play) = &mut self.play
                                && let Err(err) = play.tick(&mut self.world, &self.registry, dt)
                            {
                                self.state.status = format!("{err}");
                                self.state.playing = false;
                            }
                        }
                        ymer_core::propagate_transforms(&mut self.world);
                        self.world.resource_mut::<Input>().end_frame();

                        run_ui(
                            &self.egui_ctx,
                            raw_input,
                            &mut self.state,
                            &mut self.world,
                            &self.registry,
                        )
                    }
                };

                if let Some(egui_winit) = self.egui_winit.as_mut() {
                    egui_winit.handle_platform_output(
                        &window,
                        std::mem::take(&mut output.platform_output),
                    );
                }

                let size = renderer.size();
                overlay.accept(&self.egui_ctx, output, size);

                let mut list = match self.mode {
                    Mode::Launcher => ymer_render::RenderList::default(),
                    Mode::Editing => {
                        build_render_list(&mut self.world, renderer.aspect_ratio(), &self.assets)
                    }
                };

                if let (Mode::Editing, Some(selected)) = (&self.mode, self.state.selected) {
                    let camera = {
                        let mut query =
                            self.world.query::<(&ymer_core::Camera, &GlobalTransform)>();
                        query
                            .iter(&self.world)
                            .next()
                            .map(|(_, g)| g.translation())
                            .unwrap_or(Vec3::ZERO)
                    };
                    list.overlay_items = gizmo::gizmo_items(
                        &mut self.world,
                        selected,
                        camera,
                        self.gizmo_mesh,
                        self.drag.map(|drag| drag.axis),
                    );
                }

                if let Err(err) = renderer.render_with_overlay(&list, Some(overlay)) {
                    log::error!("renderfel: {err}");
                    event_loop.exit();
                }

                if let Some(handle) = opened {
                    self.open_project(handle);
                }
            }

            _ => {}
        }

        if response.repaint {
            window.request_redraw();
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

/// `engine.d.ts` och `tsconfig.json` genereras ur typregistret, så att
/// VS Code känner igen `engine`, `Entity` och alla komponenter.
fn write_typescript_support(handle: &ProjectHandle, registry: &TypeRegistry) -> anyhow::Result<()> {
    std::fs::create_dir_all(handle.scripts_dir())?;
    std::fs::write(
        handle.scripts_dir().join("engine.d.ts"),
        ymer_scene::typescript::definitions(registry),
    )?;
    std::fs::write(
        handle.path("tsconfig.json"),
        ymer_scene::typescript::tsconfig(),
    )?;
    Ok(())
}

/// Alla modeller i projektet, oavsett mapp. Normalt ligger de i `models/`,
/// men en fil som hamnat någon annanstans ska ändå fungera – annars pekar
/// prefabs på meshnamn som inte registrerats.
fn find_models(root: &Path) -> Vec<std::path::PathBuf> {
    fn walk(dir: &Path, found: &mut Vec<std::path::PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, found);
            } else if is_gltf(&path) {
                found.push(path);
            }
        }
    }

    let mut found = Vec::new();
    walk(root, &mut found);
    found.sort();
    found
}

fn is_gltf(path: &std::path::Path) -> bool {
    matches!(
        path.extension()
            .and_then(|e| e.to_str())
            .map(str::to_lowercase)
            .as_deref(),
        Some("gltf" | "glb")
    )
}

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // Filutforskaren behöver något att bläddra i även första gången.
    std::fs::create_dir_all(PROJECTS_DIR)?;
    let _ = FileBrowser::new(PROJECTS_DIR);

    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.run_app(&mut Editor::new())?;
    Ok(())
}
