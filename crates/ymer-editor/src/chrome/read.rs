//! Tillbaka ur trädet: vad användaren ändrade.
//!
//! Spegelbilden av `build`. Den läser *förra* framens träd – det som
//! pekaren just rörde – och gör om ändringarna till kommandon. Körs den
//! efter ombyggnaden läser den den nybyggda nodens värde i stället för
//! användarens ändring, och ingenting händer någonsin.

use bevy_ecs::prelude::*;
use serde_json::Value;
use ymer_scene::TypeRegistry;
use ymer_ui::prelude::*;
use ymer_ui::{Events, Kind};

use super::fields::{self, Assets, Shape, field_id, shape_of};
use super::{Actions, Chrome, components};
use crate::{Action, EditorState};

impl Chrome {
    /// Värden som ändrats sedan trädet byggdes.
    pub(super) fn read_back(
        &self,
        editor: &mut EditorState,
        world: &World,
        registry: &TypeRegistry,
        actions: &mut Actions,
    ) {
        if let Some(text) = self.text_of("tb/scene") {
            editor.scene_path = text;
        }
        if let Some(text) = self.text_of("files/name")
            && let Some(browser) = editor.browser.as_mut()
        {
            browser.new_name = text;
        }
        if let Some(NodeValue::Bool(raw)) = self.document.value("insp/raw") {
            editor.raw_mode = raw;
        }

        let Some(entity) = editor.selected.filter(|e| world.entities().contains(*e)) else {
            return;
        };

        let assets = Assets {
            scripts: &editor.scripts,
            textures: &editor.textures,
            meshes: &editor.available_meshes,
        };
        let (present, _) = components(world, registry, entity);

        for (name, value) in present {
            if editor.raw_mode {
                if let Some(text) = self.text_of(&format!("raw/{name}")) {
                    editor.drafts.insert((entity, name.clone()), text);
                }
                continue;
            }

            let mut updated = value.clone();
            if self.read_field(&name, &mut Vec::new(), None, &mut updated, &assets) {
                actions.push(Action::WriteJson {
                    component: name,
                    value: updated,
                });
            }
        }
    }

    /// Klick, dubbelklick och annat som hände.
    pub(super) fn read_events(
        &self,
        events: &Events,
        editor: &mut EditorState,
        world: &World,
        registry: &TypeRegistry,
        actions: &mut Actions,
    ) {
        for id in &events.clicked {
            match id.as_str() {
                "tb/play" => editor.playing = !editor.playing,
                "tb/export" => actions.push(Action::Export),
                "tb/spawn" => actions.push(Action::SpawnEntity),
                "tb/dup" => actions.push(Action::DuplicateEntity),
                "tb/del" => actions.push(Action::DeleteEntity),
                "tb/prefab" => actions.push(Action::SavePrefab),
                "tb/save" => actions.push(Action::Save),
                "tb/load" => actions.push(Action::Load),
                "files/up" => {
                    if let Some(browser) = editor.browser.as_mut() {
                        browser.go_up();
                    }
                }
                "files/new-file" => file_command(editor, FileCommand::NewFile),
                "files/new-dir" => file_command(editor, FileCommand::NewFolder),
                "files/delete" => file_command(editor, FileCommand::Delete),
                _ => self.read_prefixed(id, editor, world, registry, actions),
            }
        }

        // Dubbelklick i filutforskaren: gå in i mappen eller öppna filen.
        for id in &events.double_clicked {
            let Some(index) = id
                .strip_prefix("files/")
                .and_then(|i| i.parse::<usize>().ok())
            else {
                continue;
            };
            let Some(browser) = editor.browser.as_mut() else {
                continue;
            };
            let Some(entry) = browser.entries().into_iter().nth(index) else {
                continue;
            };
            if entry.is_dir {
                browser.enter(&entry.path);
            } else {
                actions.push(Action::OpenFile(entry.path));
            }
        }
    }

    fn read_prefixed(
        &self,
        id: &str,
        editor: &mut EditorState,
        world: &World,
        registry: &TypeRegistry,
        actions: &mut Actions,
    ) {
        if let Some(bits) = id.strip_prefix("hier/") {
            if let Ok(bits) = bits.parse::<u64>() {
                let entity = Entity::from_bits(bits);
                if world.entities().contains(entity) {
                    editor.selected = Some(entity);
                }
            }
            return;
        }
        if let Some(index) = id
            .strip_prefix("files/")
            .and_then(|i| i.parse::<usize>().ok())
        {
            if let Some(browser) = editor.browser.as_mut()
                && let Some(entry) = browser.entries().into_iter().nth(index)
            {
                browser.selected = Some(entry.path);
            }
            return;
        }
        if let Some(name) = id.strip_prefix("add/") {
            actions.push(Action::Add {
                component: name.to_string(),
            });
            return;
        }
        if let Some(rest) = id.strip_prefix("insp/")
            && let Some(name) = rest.strip_suffix("/remove")
        {
            actions.push(Action::Remove {
                component: name.to_string(),
            });
            return;
        }
        if let Some(rest) = id.strip_prefix("raw/")
            && let Some(name) = rest.strip_suffix("/apply")
        {
            let Some(entity) = editor.selected else {
                return;
            };
            let ron = editor
                .drafts
                .get(&(entity, name.to_string()))
                .cloned()
                .unwrap_or_default();
            actions.push(Action::Write {
                component: name.to_string(),
                ron,
            });
            let _ = registry;
        }
    }

    /// Talet i ett sifferfält, utan omvägen via `NodeValue`.
    ///
    /// `NodeValue::Number` är f32; inspektorn räknar i f64 och skulle
    /// annars rapportera en ändring varje frame för tal som inte ryms.
    fn number_of(&self, id: &str) -> Option<f64> {
        match self.document.find(id).map(|node| &node.kind) {
            Some(Kind::NumberField { value, .. }) => Some(*value),
            _ => None,
        }
    }

    fn text_of(&self, id: &str) -> Option<String> {
        match self.document.value(id) {
            Some(NodeValue::Text(text)) => Some(text),
            _ => None,
        }
    }

    /// Läser ett värde ur trädet. Sant om något ändrades.
    ///
    /// Parallell med `build::field`, och delar `shape_of` med den: valet
    /// av fältsort får bara fattas på ett ställe.
    fn read_field(
        &self,
        component: &str,
        path: &mut Vec<String>,
        label: Option<&str>,
        value: &mut Value,
        assets: &Assets,
    ) -> bool {
        let id = field_id(component, path);
        match shape_of(component, label, value, assets) {
            Shape::Bool => match self.document.value(&id) {
                Some(NodeValue::Bool(flag)) if Some(flag) != value.as_bool() => {
                    *value = Value::Bool(flag);
                    true
                }
                _ => false,
            },

            Shape::Int => match self.number_of(&id) {
                Some(number) if changed(number, value.as_f64().unwrap_or(0.0)) => {
                    *value = Value::Number((number.round() as i64).into());
                    true
                }
                _ => false,
            },

            Shape::Float { .. } => match self.number_of(&id) {
                Some(number) if changed(number, value.as_f64().unwrap_or(0.0)) => {
                    *value = fields::number(number);
                    true
                }
                _ => false,
            },

            Shape::Text => match self.text_of(&id) {
                Some(text) if Some(text.as_str()) != value.as_str() => {
                    *value = Value::String(text);
                    true
                }
                _ => false,
            },

            Shape::Asset { options, .. } => {
                let Some(NodeValue::Index(selected)) = self.document.value(&id) else {
                    return false;
                };
                let chosen = selected
                    .and_then(|index| options.get(index))
                    .cloned()
                    .unwrap_or_default();
                if Some(chosen.as_str()) == value.as_str() {
                    return false;
                }
                *value = Value::String(chosen);
                true
            }

            Shape::Vector { axes } => {
                let mut changed_any = false;
                let Some(items) = value.as_array_mut() else {
                    return false;
                };
                for index in 0..axes.len() {
                    path.push(index.to_string());
                    let field = field_id(component, path);
                    path.pop();
                    let Some(number) = self.number_of(&field) else {
                        continue;
                    };
                    let current = items.get(index).and_then(Value::as_f64).unwrap_or(0.0);
                    if changed(number, current) {
                        items[index] = fields::number(number);
                        changed_any = true;
                    }
                }
                changed_any
            }

            Shape::Euler => {
                let Some(items) = value.as_array_mut() else {
                    return false;
                };
                let current = fields::euler_degrees(items);
                let mut degrees = current;
                for index in 0..3 {
                    path.push(format!("deg{index}"));
                    let field = field_id(component, path);
                    path.pop();
                    if let Some(number) = self.number_of(&field) {
                        degrees[index] = number as f32;
                    }
                }
                // Jämför graderna, inte kvaternionen: vägen tillbaka är
                // inte exakt, och en jämförelse på kvaternionen hade sett
                // en ändring varje frame och långsamt vridit objektet.
                if (0..3).all(|i| (degrees[i] - current[i]).abs() < 1e-4) {
                    return false;
                }
                let quat = fields::euler_quat(degrees);
                for (index, part) in quat.iter().enumerate() {
                    if index < items.len() {
                        items[index] = fields::number(*part);
                    }
                }
                true
            }

            Shape::Color => {
                let Some(map) = value.as_object_mut() else {
                    return false;
                };
                let mut changed_any = false;
                for key in ["r", "g", "b", "a"] {
                    path.push(key.to_string());
                    let field = field_id(component, path);
                    path.pop();
                    let Some(number) = self.number_of(&field) else {
                        continue;
                    };
                    let current = map.get(key).and_then(Value::as_f64).unwrap_or(1.0);
                    if changed(number, current) {
                        map.insert(key.to_string(), fields::number(number));
                        changed_any = true;
                    }
                }
                changed_any
            }

            Shape::Object => {
                let Some(map) = value.as_object_mut() else {
                    return false;
                };
                let keys: Vec<String> = map.keys().cloned().collect();
                let mut changed_any = false;
                for key in keys {
                    let Some(mut inner) = map.get(&key).cloned() else {
                        continue;
                    };
                    path.push(key.clone());
                    if self.read_field(component, path, Some(&key), &mut inner, assets) {
                        map.insert(key.clone(), inner);
                        changed_any = true;
                    }
                    path.pop();
                }
                changed_any
            }

            Shape::Array => {
                let Some(items) = value.as_array().cloned() else {
                    return false;
                };
                let mut changed_any = false;
                let mut updated = items.clone();
                for (index, item) in items.iter().enumerate() {
                    let mut inner = item.clone();
                    path.push(index.to_string());
                    if self.read_field(component, path, None, &mut inner, assets) {
                        updated[index] = inner;
                        changed_any = true;
                    }
                    path.pop();
                }
                if changed_any {
                    *value = Value::Array(updated);
                }
                changed_any
            }

            Shape::Null => false,
        }
    }
}

/// Ändrades talet på riktigt?
///
/// Fältet byggs med exakt det värde komponenten har, så orört är det
/// bitvis lika. Marginalen finns för klampningen i fält med spann.
fn changed(new: f64, old: f64) -> bool {
    (new - old).abs() > 1e-9
}

enum FileCommand {
    NewFile,
    NewFolder,
    Delete,
}

fn file_command(editor: &mut EditorState, command: FileCommand) {
    let Some(browser) = editor.browser.as_mut() else {
        return;
    };
    let name = browser.new_name.clone();
    browser.status = match command {
        FileCommand::NewFile => match browser.create_file(&name) {
            Ok(path) => {
                browser.new_name.clear();
                format!("skapade {}", path.display())
            }
            Err(err) => format!("{err}"),
        },
        FileCommand::NewFolder => match browser.create_folder(&name) {
            Ok(_) => {
                browser.new_name.clear();
                "mapp skapad".to_string()
            }
            Err(err) => format!("{err}"),
        },
        FileCommand::Delete => {
            let Some(selected) = browser.selected.clone() else {
                return;
            };
            match browser.delete(&selected) {
                Ok(()) => "borttagen".to_string(),
                Err(err) => format!("{err}"),
            }
        }
    };
}
