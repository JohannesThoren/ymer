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
pub mod chrome;
pub mod export;
pub mod gizmo;
pub mod gltf_import;
pub mod launcher;
pub mod play;
pub mod project;
pub mod view;
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
    /// Skript och texturer i projektet, för fälten som pekar ut filer.
    /// Uppdateras av `refresh_assets`.
    pub scripts: Vec<String>,
    pub textures: Vec<String>,
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
            scripts: Vec::new(),
            textures: Vec::new(),
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

    /// Läser om projektets skript- och texturlistor.
    ///
    /// Görs en gång per frame i stället för vid varje ändring: en
    /// katalogläsning är billig, och en lista som är en frame gammal är
    /// ett mindre problem än en som aldrig uppdateras.
    pub fn refresh_assets(&mut self) {
        let Some(handle) = &self.project else {
            self.scripts.clear();
            self.textures.clear();
            return;
        };
        self.scripts = project::list_scripts(&handle.scripts_dir());
        self.textures = project::list_files(&handle.path("textures"), &handle.root, "png");
    }

    /// Absolut sökväg för en scenfil, relativt projektet om ett är öppet.
    pub fn scene_file(&self) -> std::path::PathBuf {
        match &self.project {
            Some(handle) => handle.path(&self.scene_path),
            None => std::path::PathBuf::from(&self.scene_path),
        }
    }
}

pub(crate) struct Row {
    entity: Entity,
    name: String,
    depth: usize,
}

pub(crate) fn collect_hierarchy(world: &World, registry: &TypeRegistry) -> Vec<Row> {
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

/// Verkställer det användaren bad om.
///
/// Skilt från ritandet, för att världen är utlånad som läsbar medan
/// gränssnittet byggs. Båda gränssnitten – det gamla egui-baserade och
/// det nya – går genom exakt den här listan, så att en knapp och en
/// genväg gör samma sak.
pub(crate) fn apply_actions(
    actions: Vec<Action>,
    state: &mut EditorState,
    world: &mut World,
    registry: &TypeRegistry,
) {
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
pub(crate) fn pretty(ron: &str) -> String {
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
