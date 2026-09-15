//! Editorn. Allt UI som rör komponenter genereras ur `TypeRegistry` –
//! ingen rad här nämner Transform, Camera eller MeshInstance vid namn.
//! Registrerar spelet en egen komponent dyker den upp i inspectorn direkt.

use std::collections::BTreeMap;

use bevy_ecs::prelude::*;
use browser::FileBrowser;
use project::ProjectHandle;
use ymer_core::{EntityName, Transform};
use ymer_scene::{Scene, TypeRegistry, clear_scene};

pub mod browser;
pub mod export;
pub mod gizmo;
pub mod gltf_import;
pub mod inspector;
pub mod launcher;
pub mod overlay;
pub mod play;
pub mod project;
pub mod view;
pub use overlay::EguiOverlay;
pub use play::PlayMode;

/// Vad användaren bad om under en UI-frame. Samlas ihop medan världen är
/// utlånad som läsbar, och verkställs efteråt.
/// Genvägar begärs av fönsterhanteringen och plockas upp av UI:t.
#[derive(Debug, Clone, Copy)]
enum Shortcut {
    Delete,
    Duplicate,
}

enum Action {
    Write {
        component: String,
        ron: String,
    },
    Remove {
        component: String,
    },
    Add {
        component: String,
    },
    WriteJson {
        component: String,
        value: serde_json::Value,
    },
    Export,
    SpawnEntity,
    DeleteEntity,
    DuplicateEntity,
    SavePrefab,
    Instantiate(std::path::PathBuf),
    Save,
    Load,
    OpenFile(std::path::PathBuf),
}

pub struct EditorState {
    pub selected: Option<Entity>,
    pub playing: bool,
    pub scene_path: String,
    pub status: String,
    /// Redigeringsbuffert per (entitet, komponent) så att texten inte skrivs
    /// över av världen medan man skriver i den.
    drafts: BTreeMap<(Entity, String), String>,
    /// Råläge: redigera komponenter som RON-text istället för fält.
    pub raw_mode: bool,
    /// Öppet projekt och dess filutforskare.
    pub project: Option<ProjectHandle>,
    pub browser: Option<FileBrowser>,
    pending_shortcut: Option<Shortcut>,
    /// Alla meshnamn assetregistret känner till – inbyggda plus importerade
    /// glTF-meshar. Underhålls av `main.rs`, som är den som äger `Assets`.
    pub available_meshes: Vec<String>,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            selected: None,
            playing: false,
            scene_path: "scene.ron".to_string(),
            status: "redo".to_string(),
            drafts: BTreeMap::new(),
            raw_mode: false,
            project: None,
            browser: None,
            pending_shortcut: None,
            available_meshes: vec![
                ymer_core::BUILTIN_CUBE.to_string(),
                ymer_core::BUILTIN_PLANE.to_string(),
            ],
        }
    }
}

impl EditorState {
    /// Kopplar state till ett öppnat projekt.
    pub fn open_project(&mut self, handle: ProjectHandle) {
        self.scene_path = handle.project.start_scene.clone();
        self.browser = Some(FileBrowser::new(handle.root.clone()));
        self.status = format!("öppnade {}", handle.project.name);
        self.project = Some(handle);
    }

    /// Begär borttagning av markerad entitet. Utförs nästa gång UI:t
    /// körs, så att genvägar och knappar tar exakt samma väg.
    pub fn request_delete(&mut self) {
        self.pending_shortcut = Some(Shortcut::Delete);
    }

    pub fn request_duplicate(&mut self) {
        self.pending_shortcut = Some(Shortcut::Duplicate);
    }

    /// Absolut sökväg för en scenfil, relativt projektet om ett är öppet.
    pub fn scene_file(&self) -> std::path::PathBuf {
        match &self.project {
            Some(handle) => handle.path(&self.scene_path),
            None => std::path::PathBuf::from(&self.scene_path),
        }
    }
}

struct Row {
    entity: Entity,
    name: String,
    depth: usize,
}

fn collect_hierarchy(world: &World, registry: &TypeRegistry) -> Vec<Row> {
    let mut rows = Vec::new();

    // Resurser lagras som entiteter i bevy_ecs 0.19. Hierarkin ska bara
    // visa scenen, alltså det registret känner igen.
    let roots: Vec<Entity> = world
        .iter_entities()
        .filter(|entity| entity.get::<ChildOf>().is_none())
        .map(|entity| entity.id())
        .filter(|entity| {
            registry
                .iter()
                .any(|component| component.read(world, *entity).is_some())
        })
        .collect();

    fn push(world: &World, entity: Entity, depth: usize, rows: &mut Vec<Row>) {
        let name = world
            .get::<EntityName>(entity)
            .map(|n| n.0.clone())
            .unwrap_or_else(|| format!("Entity {}", entity.index()));

        rows.push(Row {
            entity,
            name,
            depth,
        });

        if let Some(children) = world.get::<Children>(entity) {
            let kids: Vec<Entity> = children.iter().collect();
            for child in kids {
                push(world, child, depth + 1, rows);
            }
        }
    }

    for root in roots {
        push(world, root, 0, &mut rows);
    }
    rows
}

/// Ritar hela editorn och verkställer det användaren gjorde.
///
/// `raw_input` måste ha ett `screen_rect` – utan det antar egui en oändlig
/// skärm och paneler som ankras till höger hamnar utanför bilden.
pub fn run_ui(
    ctx: &egui::Context,
    raw_input: egui::RawInput,
    state: &mut EditorState,
    world: &mut World,
    registry: &TypeRegistry,
) -> egui::FullOutput {
    // --- fas 1: läs ut allt UI:t behöver -------------------------------
    let rows = collect_hierarchy(world, registry);
    let entity_count = rows.len();

    let selected = state
        .selected
        .filter(|entity| world.entities().contains(*entity));
    state.selected = selected;

    // Komponenter på den valda entiteten. RON för rålägets textrutor,
    // JSON för de typade fälten – samma data, två vyer.
    let mut present: Vec<(String, String)> = Vec::new();
    let mut fields: Vec<(String, serde_json::Value)> = Vec::new();
    let mut missing: Vec<String> = Vec::new();
    if let Some(entity) = selected {
        for component_type in registry.iter() {
            match component_type.read(world, entity) {
                Some(value) => {
                    present.push((component_type.name.to_string(), value.get_ron().to_string()));
                    if let Some(json) = component_type.read_json(world, entity) {
                        fields.push((component_type.name.to_string(), json));
                    }
                }
                None => missing.push(component_type.name.to_string()),
            }
        }
    }

    // Skriptväljaren behöver veta vad som finns i projektet.
    let scripts = state
        .project
        .as_ref()
        .map(|handle| inspector::list_scripts(&handle.scripts_dir()))
        .unwrap_or_default();

    // Texturer namnges relativt projektroten, precis som i assetregistret.
    let textures = state
        .project
        .as_ref()
        .map(|handle| inspector::list_files(&handle.path("textures"), &handle.root, "png"))
        .unwrap_or_default();

    // Fyll på redigeringsbuffertar för komponenter vi inte redan har text för.
    for (name, ron) in &present {
        let key = (selected.unwrap(), name.clone());
        state.drafts.entry(key).or_insert_with(|| pretty(ron));
    }

    // --- fas 2: rita ---------------------------------------------------
    let mut actions: Vec<Action> = Vec::new();
    match state.pending_shortcut.take() {
        Some(Shortcut::Delete) => actions.push(Action::DeleteEntity),
        Some(Shortcut::Duplicate) => actions.push(Action::DuplicateEntity),
        None => {}
    }

    // egui 0.36: panelerna är en enda `Panel`-typ och tar ett `Ui`, inte ett `Context`.
    let output = ctx.run_ui(raw_input, |ui| {
        egui::Panel::top("toolbar").show(ui, |ui| {
            ui.horizontal(|ui| {
                let label = if state.playing {
                    "⏸ Paus"
                } else {
                    "▶ Spela"
                };
                if ui.button(label).clicked() {
                    state.playing = !state.playing;
                }
                ui.separator();
                if state.project.is_some()
                    && ui
                        .button("📦 Exportera")
                        .on_hover_text("bygg körbart spel")
                        .clicked()
                {
                    actions.push(Action::Export);
                }
                if ui
                    .button("＋ Entitet")
                    .on_hover_text("ny kub i origo")
                    .clicked()
                {
                    actions.push(Action::SpawnEntity);
                }
                if state.selected.is_some() {
                    if ui
                        .button("⧉ Duplicera")
                        .on_hover_text("kopiera markerad entitet (Ctrl+D)")
                        .clicked()
                    {
                        actions.push(Action::DuplicateEntity);
                    }
                    if ui
                        .button("🗑 Ta bort")
                        .on_hover_text("ta bort markerad entitet (Delete)")
                        .clicked()
                    {
                        actions.push(Action::DeleteEntity);
                    }
                    if ui
                        .button("⭐ Prefab")
                        .on_hover_text("spara markerad som prefab")
                        .clicked()
                    {
                        actions.push(Action::SavePrefab);
                    }
                }
                ui.separator();
                if ui.button("💾 Spara").clicked() {
                    actions.push(Action::Save);
                }
                if ui.button("📂 Ladda").clicked() {
                    actions.push(Action::Load);
                }
                ui.add(
                    egui::TextEdit::singleline(&mut state.scene_path)
                        .desired_width(160.0)
                        .hint_text("scenfil"),
                );
                ui.separator();
                ui.label(format!("{entity_count} entiteter"));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(&state.status);
                });
            });
        });

        egui::Panel::left("hierarchy")
            .resizable(true)
            .default_size(240.0)
            .show(ui, |ui| {
                ui.heading("Hierarki");
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for row in &rows {
                        ui.horizontal(|ui| {
                            ui.add_space(row.depth as f32 * 14.0);
                            let selected = state.selected == Some(row.entity);
                            let prefix = if row.depth > 0 { "└ " } else { "" };
                            if ui
                                .selectable_label(selected, format!("{prefix}{}", row.name))
                                .clicked()
                            {
                                state.selected = Some(row.entity);
                            }
                        });
                    }
                });
            });

        if state.browser.is_some() {
            egui::Panel::bottom("files")
                .resizable(true)
                .default_size(190.0)
                .show(ui, |ui| {
                    let mut enter: Option<std::path::PathBuf> = None;
                    let mut go_up = false;
                    let mut create_file = false;
                    let mut create_folder = false;
                    let mut delete: Option<std::path::PathBuf> = None;

                    let browser = state.browser.as_mut().expect("kontrollerad ovan");

                    ui.horizontal(|ui| {
                        if ui.button("⬆").on_hover_text("upp en nivå").clicked() {
                            go_up = true;
                        }
                        ui.label(
                            egui::RichText::new(browser.breadcrumb())
                                .monospace()
                                .strong(),
                        );
                        ui.separator();
                        ui.add(
                            egui::TextEdit::singleline(&mut browser.new_name)
                                .hint_text("namn.ts")
                                .desired_width(150.0),
                        );
                        if ui.button("Ny fil").clicked() {
                            create_file = true;
                        }
                        if ui.button("Ny mapp").clicked() {
                            create_folder = true;
                        }
                        if let Some(selected) = browser.selected.clone() {
                            ui.separator();
                            if ui.button("Ta bort").clicked() {
                                delete = Some(selected);
                            }
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(egui::RichText::new(&browser.status).weak());
                        });
                    });
                    ui.separator();

                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for entry in browser.entries() {
                            let icon = if entry.is_dir { "📁" } else { "📄" };
                            let selected =
                                browser.selected.as_deref() == Some(entry.path.as_path());
                            let response =
                                ui.selectable_label(selected, format!("{icon}  {}", entry.name));

                            if response.clicked() {
                                browser.selected = Some(entry.path.clone());
                            }
                            // Dubbelklick öppnar mappar och laddar scener.
                            if response.double_clicked() {
                                if entry.is_dir {
                                    enter = Some(entry.path.clone());
                                } else {
                                    actions.push(Action::OpenFile(entry.path.clone()));
                                }
                            }
                        }
                    });

                    let name = browser.new_name.clone();
                    if go_up {
                        browser.go_up();
                    }
                    if let Some(path) = enter {
                        browser.enter(&path);
                    }
                    if create_file {
                        browser.status = match browser.create_file(&name) {
                            Ok(path) => {
                                browser.new_name.clear();
                                format!("skapade {}", path.display())
                            }
                            Err(err) => format!("{err}"),
                        };
                    }
                    if create_folder {
                        browser.status = match browser.create_folder(&name) {
                            Ok(_) => {
                                browser.new_name.clear();
                                "mapp skapad".to_string()
                            }
                            Err(err) => format!("{err}"),
                        };
                    }
                    if let Some(path) = delete {
                        browser.status = match browser.delete(&path) {
                            Ok(()) => "borttagen".to_string(),
                            Err(err) => format!("{err}"),
                        };
                    }
                });
        }

        egui::Panel::right("inspector")
            .resizable(true)
            .default_size(360.0)
            .show(ui, |ui| {
                ui.heading("Inspector");
                ui.separator();

                let Some(entity) = state.selected else {
                    ui.label("Ingen entitet vald.");
                    return;
                };

                ui.horizontal(|ui| {
                    // Råläget är kvar som reservutgång för allt fälten inte når.
                    ui.checkbox(&mut state.raw_mode, "RON-läge");
                });
                ui.separator();

                let inspector_ctx = inspector::Context {
                    scripts: &scripts,
                    textures: &textures,
                    meshes: &state.available_meshes,
                };

                egui::ScrollArea::vertical().show(ui, |ui| {
                    if state.raw_mode {
                        for (name, _) in &present {
                            egui::CollapsingHeader::new(name)
                                .default_open(true)
                                .show(ui, |ui| {
                                    let key = (entity, name.clone());
                                    if let Some(draft) = state.drafts.get_mut(&key) {
                                        ui.add(
                                            egui::TextEdit::multiline(draft)
                                                .code_editor()
                                                .desired_rows(3)
                                                .desired_width(f32::INFINITY),
                                        );
                                    }
                                    ui.horizontal(|ui| {
                                        if ui.button("Applicera").clicked()
                                            && let Some(draft) = state.drafts.get(&key)
                                        {
                                            actions.push(Action::Write {
                                                component: name.clone(),
                                                ron: draft.clone(),
                                            });
                                        }
                                        if ui.button("Ta bort").clicked() {
                                            actions.push(Action::Remove {
                                                component: name.clone(),
                                            });
                                        }
                                    });
                                });
                        }
                    } else {
                        for (name, value) in fields.iter_mut() {
                            egui::CollapsingHeader::new(name.as_str())
                                .default_open(true)
                                .show(ui, |ui| {
                                    // Ändringar skrivs direkt till världen –
                                    // ingen Applicera-knapp att glömma.
                                    if inspector::edit_component(ui, name, value, &inspector_ctx) {
                                        actions.push(Action::WriteJson {
                                            component: name.clone(),
                                            value: value.clone(),
                                        });
                                    }
                                    if ui.button("Ta bort").clicked() {
                                        actions.push(Action::Remove {
                                            component: name.clone(),
                                        });
                                    }
                                });
                        }
                    }

                    if !missing.is_empty() {
                        ui.separator();
                        ui.label("Lägg till komponent:");
                        ui.horizontal_wrapped(|ui| {
                            for name in &missing {
                                if ui.button(format!("+ {name}")).clicked() {
                                    actions.push(Action::Add {
                                        component: name.clone(),
                                    });
                                }
                            }
                        });
                    }
                });
            });
    });

    // --- fas 3: verkställ ----------------------------------------------
    // OpenFile kan lägga till fler kommandon (ladda scenen den pekar på).
    let mut actions_late: Vec<Action> = Vec::new();
    for action in actions.into_iter().chain(std::mem::take(&mut actions_late)) {
        let entity = state.selected;
        match action {
            Action::Write { component, ron } => {
                let Some(entity) = entity else { continue };
                match apply_ron(world, registry, entity, &component, &ron) {
                    Ok(()) => state.status = format!("{component} uppdaterad"),
                    // Trasig RON ska inte krascha editorn – visa felet och behåll texten.
                    Err(err) => state.status = format!("fel i {component}: {err}"),
                }
            }
            Action::WriteJson { component, value } => {
                let Some(entity) = entity else { continue };
                let Some(component_type) = registry.get(&component) else {
                    continue;
                };
                if let Err(err) = component_type.write_json(world, entity, value) {
                    state.status = format!("fel i {component}: {err}");
                }
                // Texten i rålägets buffert är nu inaktuell.
                state.drafts.remove(&(entity, component));
            }

            Action::Remove { component } => {
                let Some(entity) = entity else { continue };
                if let Some(component_type) = registry.get(&component) {
                    component_type.remove(world, entity);
                    state.drafts.remove(&(entity, component.clone()));
                    state.status = format!("{component} borttagen");
                }
            }
            Action::Add { component } => {
                let Some(entity) = entity else { continue };
                if let Some(component_type) = registry.get(&component) {
                    component_type.insert_default(world, entity);
                    state.drafts.remove(&(entity, component.clone()));
                    state.status = format!("{component} tillagd");
                }
            }
            Action::OpenFile(path) => {
                // Scener laddas, skript får en notis – de kopplas via
                // Script-komponenten, inte genom att "öppnas".
                let extension = path
                    .extension()
                    .map(|e| e.to_string_lossy().to_lowercase())
                    .unwrap_or_default();
                match extension.as_str() {
                    // Filer i prefabs/ instansieras i scenen, allt annat
                    // laddas som en hel scen. Mappen är typen.
                    "ron" if path.components().any(|part| part.as_os_str() == "prefabs") => {
                        actions_late.push(Action::Instantiate(path.clone()));
                    }
                    "ron" => {
                        if let Some(handle) = &state.project {
                            state.scene_path = handle.relative(&path);
                        } else {
                            state.scene_path = path.to_string_lossy().into_owned();
                        }
                        actions_late.push(Action::Load);
                    }
                    "ts" => {
                        let name = path
                            .file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_default();
                        state.status = format!("{name}: sätt Script-komponenten till detta namn");
                    }
                    other => state.status = format!("vet inte vad jag ska göra med .{other}"),
                }
            }

            Action::Export => {
                let Some(handle) = &state.project else {
                    continue;
                };
                // Bredvid projektet, inte i det – annars packas förra
                // exporten in i nästa.
                let output = handle.root.parent().unwrap_or(&handle.root).join(format!(
                    "{}-export",
                    handle
                        .root
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                ));

                state.status = match export::export(handle, &output) {
                    Ok(report) => report.summary(),
                    Err(err) => format!("export misslyckades: {err}"),
                };
            }

            Action::SpawnEntity => {
                let spawned = world
                    .spawn((
                        ymer_core::EntityName::new("Ny entitet"),
                        ymer_core::Transform::IDENTITY,
                        ymer_core::GlobalTransform::default(),
                        ymer_core::MeshInstance::new(
                            ymer_core::BUILTIN_CUBE,
                            ymer_core::Color::rgb(0.8, 0.8, 0.85),
                        ),
                    ))
                    .id();
                state.selected = Some(spawned);
                state.status = "entitet skapad".to_string();
            }

            Action::DeleteEntity => {
                let Some(target) = entity else { continue };
                let name = world
                    .get::<EntityName>(target)
                    .map(|name| name.0.clone())
                    .unwrap_or_else(|| "entiteten".to_string());

                // despawn tar barnen med sig via ChildOf-relationen.
                state.status = match world.try_despawn(target) {
                    Ok(()) => {
                        state.selected = None;
                        state.drafts.retain(|(entity, _), _| *entity != target);
                        format!("tog bort {name}")
                    }
                    Err(err) => format!("kunde inte ta bort {name}: {err}"),
                };
            }

            Action::DuplicateEntity => {
                let Some(target) = entity else { continue };

                // Samma väg som prefabs: serialisera delträdet och spawna
                // in det igen. Då följer barn och alla komponenter med
                // utan att duplicera logiken.
                let copy = Scene::from_subtree(world, registry, target);
                state.status = match copy.spawn_into(world, registry) {
                    Ok(mapping) => {
                        if let Some(&root) = mapping.get(&0) {
                            // Flytta kopian en aning så att den inte göms
                            // exakt bakom originalet.
                            if let Some(mut transform) = world.get_mut::<Transform>(root) {
                                transform.translation.x += 1.0;
                            }
                            state.selected = Some(root);
                        }
                        format!("duplicerade {} entiteter", mapping.len())
                    }
                    Err(err) => format!("kunde inte duplicera: {err}"),
                };
            }

            Action::SavePrefab => {
                let Some(entity) = entity else { continue };
                let name = world
                    .get::<EntityName>(entity)
                    .map(|name| name.0.clone())
                    .unwrap_or_else(|| "prefab".to_string());
                let file = format!("{}.ron", name.replace(['/', '\\'], "-"));

                let Some(handle) = &state.project else {
                    state.status = "inget projekt öppet".to_string();
                    continue;
                };
                let path = handle.path("prefabs").join(&file);

                let prefab = Scene::from_subtree(world, registry, entity);
                state.status = match prefab.save(&path) {
                    Ok(()) => format!(
                        "sparade prefabs/{file} ({} entiteter)",
                        prefab.entities.len()
                    ),
                    Err(err) => format!("{err}"),
                };
            }

            Action::Instantiate(path) => {
                state.status =
                    match Scene::load(&path).and_then(|scene| scene.spawn_into(world, registry)) {
                        Ok(mapping) => {
                            // Roten har alltid id 0 i en prefab.
                            state.selected = mapping.get(&0).copied();
                            format!("instansierade {} entiteter", mapping.len())
                        }
                        Err(err) => format!("{err}"),
                    };
            }

            Action::Save => {
                let scene = Scene::from_world(world, registry);
                state.status = match scene.save(state.scene_file()) {
                    Ok(()) => format!("sparade {} entiteter", scene.entities.len()),
                    Err(err) => format!("kunde inte spara: {err}"),
                };
            }
            Action::Load => {
                // Ladda ersätter scenen istället för att lägga ovanpå den.
                clear_scene(world, registry);
                state.status = match Scene::load(state.scene_file())
                    .and_then(|scene| scene.spawn_into(world, registry).map(|m| m.len()))
                {
                    Ok(count) => {
                        state.drafts.clear();
                        state.selected = None;
                        format!("laddade {count} entiteter")
                    }
                    Err(err) => format!("kunde inte ladda: {err}"),
                };
            }
        }
    }

    output
}

fn apply_ron(
    world: &mut World,
    registry: &TypeRegistry,
    entity: Entity,
    component: &str,
    ron_text: &str,
) -> anyhow::Result<()> {
    let component_type = registry
        .get(component)
        .ok_or_else(|| anyhow::anyhow!("okänd komponent"))?;
    let raw = ron::value::RawValue::from_boxed_ron(ron_text.into())
        .map_err(|err| anyhow::anyhow!("{err}"))?;
    component_type.write(world, entity, &raw)
}

/// RON på en rad är svårläst i ett textfält – bryt efter kommatecken på toppnivå.
fn pretty(ron: &str) -> String {
    let mut out = String::with_capacity(ron.len() + 16);
    let mut depth = 0usize;
    for ch in ron.chars() {
        match ch {
            '(' | '[' => {
                depth += 1;
                out.push(ch);
            }
            ')' | ']' => {
                depth = depth.saturating_sub(1);
                out.push(ch);
            }
            ',' if depth == 1 => {
                out.push_str(",\n  ");
            }
            _ => out.push(ch),
        }
    }
    out
}
