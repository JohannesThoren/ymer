//! Typregistret och scenformatet – navet i motorn.
//!
//! Varje komponenttyp registreras med ett namn och fyra funktioner:
//! läsa, skriva, sätta default och ta bort. Allt annat som behöver arbeta
//! med komponenter generiskt – scenfiler, editorns inspector, WASM-ABI:t –
//! går genom registret istället för att känna till typerna.

pub mod typescript;

use std::collections::BTreeMap;

use bevy_ecs::prelude::*;
use ron::value::RawValue;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

// ------------------------------------------------------------- registret

/// Typraderade operationer för en komponenttyp.
#[derive(Clone, Copy)]
pub struct ComponentType {
    pub name: &'static str,
    read: fn(&World, Entity) -> Option<Box<RawValue>>,
    write: fn(&mut World, Entity, &RawValue) -> anyhow::Result<()>,
    // JSON-vägen används av skriptlagret: JS kan inte läsa RON.
    read_json: fn(&World, Entity) -> Option<serde_json::Value>,
    write_json: fn(&mut World, Entity, serde_json::Value) -> anyhow::Result<()>,
    insert_default: fn(&mut World, Entity),
    remove: fn(&mut World, Entity),
}

impl ComponentType {
    /// Läser komponenten från en entitet som RON, eller None om den saknas.
    pub fn read(&self, world: &World, entity: Entity) -> Option<Box<RawValue>> {
        (self.read)(world, entity)
    }

    /// Skriver komponenten till en entitet från RON.
    pub fn write(&self, world: &mut World, entity: Entity, value: &RawValue) -> anyhow::Result<()> {
        (self.write)(world, entity, value)
    }

    /// Samma sak som `read`, men som JSON – gränssnittet mot skript.
    pub fn read_json(&self, world: &World, entity: Entity) -> Option<serde_json::Value> {
        (self.read_json)(world, entity)
    }

    pub fn write_json(
        &self,
        world: &mut World,
        entity: Entity,
        value: serde_json::Value,
    ) -> anyhow::Result<()> {
        (self.write_json)(world, entity, value)
    }

    /// "Add Component" i editorn.
    pub fn insert_default(&self, world: &mut World, entity: Entity) {
        (self.insert_default)(world, entity)
    }

    pub fn remove(&self, world: &mut World, entity: Entity) {
        (self.remove)(world, entity)
    }
}

/// Alla kända komponenttyper, sorterade på namn.
#[derive(Resource, Default, Clone)]
pub struct TypeRegistry {
    types: BTreeMap<&'static str, ComponentType>,
}

impl TypeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registrerar en komponenttyp. Namnet är det som hamnar i scenfilen
    /// och i editorns UI, så byt det inte i onödan.
    pub fn register<T>(&mut self, name: &'static str) -> &mut Self
    where
        T: Component<Mutability = bevy_ecs::component::Mutable>
            + Serialize
            + DeserializeOwned
            + Default
            + Clone,
    {
        self.types.insert(
            name,
            ComponentType {
                name,
                read: |world, entity| {
                    let value = world.get::<T>(entity)?;
                    RawValue::from_rust(value).ok()
                },
                write: |world, entity, raw| {
                    let value: T = raw.into_rust().map_err(|err| anyhow::anyhow!("{err}"))?;
                    world.entity_mut(entity).insert(value);
                    Ok(())
                },
                read_json: |world, entity| {
                    let value = world.get::<T>(entity)?;
                    serde_json::to_value(value).ok()
                },
                write_json: |world, entity, value| {
                    let parsed: T = serde_json::from_value(value)?;
                    world.entity_mut(entity).insert(parsed);
                    Ok(())
                },
                insert_default: |world, entity| {
                    world.entity_mut(entity).insert(T::default());
                },
                remove: |world, entity| {
                    world.entity_mut(entity).remove::<T>();
                },
            },
        );
        self
    }

    pub fn get(&self, name: &str) -> Option<&ComponentType> {
        self.types.get(name)
    }

    /// Alla typer, i namnordning – editorns "Add Component"-lista.
    pub fn iter(&self) -> impl Iterator<Item = &ComponentType> {
        self.types.values()
    }

    pub fn names(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.types.keys().copied()
    }

    pub fn len(&self) -> usize {
        self.types.len()
    }

    pub fn is_empty(&self) -> bool {
        self.types.is_empty()
    }
}

/// Registrerar motorns egna komponenter.
pub fn register_builtin_types(registry: &mut TypeRegistry) {
    use ymer_core::{Camera, EntityName, MeshInstance, Script, Sprite, SpriteAnimation, Transform};

    registry
        .register::<EntityName>("Name")
        .register::<Transform>("Transform")
        .register::<Camera>("Camera")
        .register::<MeshInstance>("MeshInstance")
        .register::<Sprite>("Sprite")
        .register::<SpriteAnimation>("SpriteAnimation")
        .register::<Script>("Script");
}

/// Tar bort alla entiteter som registret känner igen – alltså scenen –
/// och lämnar allt annat i fred.
///
/// `World::clear_entities` går **inte** att använda: i bevy_ecs 0.19 lagras
/// resurser som komponenter på entiteter, så den raderar `Time`, `Input`
/// och allt annat med. Nästa resursläsning panikar då.
pub fn clear_scene(world: &mut World, registry: &TypeRegistry) -> usize {
    let candidates: Vec<Entity> = world.iter_entities().map(|entity| entity.id()).collect();

    let doomed: Vec<Entity> = candidates
        .into_iter()
        .filter(|entity| {
            registry
                .iter()
                .any(|component| component.read(world, *entity).is_some())
        })
        .collect();

    let mut removed = 0;
    for entity in doomed {
        // Despawn av en förälder tar barnen med sig, så listan hinner
        // bli inaktuell under tiden – try_despawn istället för despawn.
        if world.try_despawn(entity).is_ok() {
            removed += 1;
        }
    }
    removed
}

// --------------------------------------------------------- scenformatet

/// En entitet i en scenfil. `id` är lokalt för filen, inte ett `Entity`.
#[derive(Debug, Serialize, Deserialize)]
pub struct SceneEntity {
    pub id: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<u32>,
    pub components: BTreeMap<String, Box<RawValue>>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Scene {
    pub entities: Vec<SceneEntity>,
}

impl Scene {
    /// Plockar ut allt som registret känner igen ur världen.
    pub fn from_world(world: &mut World, registry: &TypeRegistry) -> Self {
        let entities: Vec<Entity> = world.iter_entities().map(|entity| entity.id()).collect();
        Self::from_entities(world, registry, entities)
    }

    /// En prefab: en entitet med alla sina barn, utan resten av scenen.
    /// Roten får alltid id 0, så den går att plocka ut vid instansiering.
    pub fn from_subtree(world: &mut World, registry: &TypeRegistry, root: Entity) -> Self {
        let mut entities = vec![root];
        let mut index = 0;
        while index < entities.len() {
            if let Some(children) = world.get::<Children>(entities[index]) {
                let kids: Vec<Entity> = children.iter().collect();
                entities.extend(kids);
            }
            index += 1;
        }
        Self::from_entities(world, registry, entities)
    }

    fn from_entities(world: &mut World, registry: &TypeRegistry, entities: Vec<Entity>) -> Self {
        // Filens id:n är index i listan; föräldrar refererar till dem.
        let index: BTreeMap<Entity, u32> = entities
            .iter()
            .enumerate()
            .map(|(index, entity)| (*entity, index as u32))
            .collect();

        let mut scene = Scene::default();
        for entity in &entities {
            let mut components = BTreeMap::new();
            for component_type in registry.iter() {
                if let Some(value) = component_type.read(world, *entity) {
                    components.insert(component_type.name.to_string(), value);
                }
            }

            // Entiteter utan kända komponenter är inte värda att spara.
            if components.is_empty() {
                continue;
            }

            // Föräldrar utanför urvalet hoppas över – en prefab ska kunna
            // instansieras var som helst.
            let parent = world
                .get::<ChildOf>(*entity)
                .and_then(|c| index.get(&c.0).copied());

            scene.entities.push(SceneEntity {
                id: index[entity],
                parent,
                components,
            });
        }
        scene
    }

    /// Skapar entiteterna i världen. Returnerar fil-id -> `Entity`.
    pub fn spawn_into(
        &self,
        world: &mut World,
        registry: &TypeRegistry,
    ) -> anyhow::Result<BTreeMap<u32, Entity>> {
        // Alla entiteter måste finnas innan föräldralänkarna kan sättas.
        let mut mapping = BTreeMap::new();
        for entity in &self.entities {
            mapping.insert(entity.id, world.spawn_empty().id());
        }

        for scene_entity in &self.entities {
            let entity = mapping[&scene_entity.id];

            for (name, value) in &scene_entity.components {
                let Some(component_type) = registry.get(name) else {
                    log::warn!("okänd komponent '{name}' i scenen – hoppar över");
                    continue;
                };
                component_type.write(world, entity, value)?;
            }

            // GlobalTransform sparas aldrig; den räknas ut av propagate_transforms.
            if world.get::<ymer_core::Transform>(entity).is_some() {
                world
                    .entity_mut(entity)
                    .insert(ymer_core::GlobalTransform::default());
            }

            if let Some(parent_id) = scene_entity.parent {
                let parent = mapping[&parent_id];
                world.entity_mut(entity).insert(ChildOf(parent));
            }
        }

        Ok(mapping)
    }

    pub fn to_ron(&self) -> anyhow::Result<String> {
        let config = ron::ser::PrettyConfig::new()
            .struct_names(true)
            .indentor("  ");
        Ok(ron::ser::to_string_pretty(self, config)?)
    }

    pub fn from_ron(text: &str) -> anyhow::Result<Self> {
        ron::from_str(text).map_err(|err| anyhow::anyhow!("{err}"))
    }

    pub fn save(&self, path: impl AsRef<std::path::Path>) -> anyhow::Result<()> {
        std::fs::write(path, self.to_ron()?)?;
        Ok(())
    }

    pub fn load(path: impl AsRef<std::path::Path>) -> anyhow::Result<Self> {
        Self::from_ron(&std::fs::read_to_string(path)?)
    }

    /// Laddar via en assetkälla – disk i editorn, arkiv i ett exporterat
    /// spel. Samma kodväg för båda.
    pub fn load_from(source: &dyn ymer_core::AssetSource, path: &str) -> anyhow::Result<Self> {
        Self::from_ron(&source.read_to_string(path)?)
    }
}
