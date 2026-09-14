//! Skriptlagret: TypeScript in, körd gameplay ut.
//!
//! Kedjan är helt i Rust – ingen Node, ingen extern kompilator:
//!   .ts  --oxc-->  .js  --QuickJS i wasm (wasmtime)-->  körning
//!
//! Skript körs som system: ett `update(dt, entities)`-anrop per frame över
//! alla matchande entiteter, inte ett anrop per objekt.

use std::path::Path;

use anyhow::{Result, anyhow};
use oxc::allocator::Allocator;
use oxc::codegen::Codegen;
use oxc::parser::Parser;
use oxc::semantic::SemanticBuilder;
use oxc::span::SourceType;
use oxc::transformer::{TransformOptions, Transformer};
use wasmtime::{Engine, Instance, Linker, Memory, Module, Store, TypedFunc};
use wasmtime_wasi::WasiCtxBuilder;
use wasmtime_wasi::p1::{WasiP1Ctx, add_to_linker_sync};

/// Antal f32 per entitet: translation(3), rotation xyzw(4), scale(3).
/// Måste matcha STRIDE i skriptmodulen.
pub const STRIDE: usize = 10;

/// Strippar TypeScript-typer och ger körbar JavaScript.
///
/// Det här är hela "kompilatorn": typerna raderas, semantiken är oförändrad.
/// Ingen typkontroll sker – den hör hemma i editorn, inte i körningen.
pub fn typescript_to_javascript(source: &str, name: &str) -> Result<String> {
    let allocator = Allocator::default();
    let source_type = SourceType::ts();

    let parsed = Parser::new(&allocator, source, source_type).parse();
    if let Some(first) = parsed.diagnostics.first() {
        return Err(anyhow!("{name}: syntaxfel: {first}"));
    }

    let mut program = parsed.program;
    let scoping = SemanticBuilder::new()
        .build(&program)
        .semantic
        .into_scoping();

    let options = TransformOptions::default();
    Transformer::new(&allocator, Path::new(name), &options)
        .build_with_scoping(scoping, &mut program);

    Ok(strip_exports(&Codegen::new().build(&program).code))
}

/// Skripten är ES-moduler för editorns skull – varje fil får då egen scope,
/// så två skript kan använda samma variabelnamn. QuickJS evaluerar dem som
/// script, så nyckelordet `export` tas bort här. Ingen funktionalitet går
/// förlorad: varje skript kör i en egen instans och `update` blir global.
fn strip_exports(code: &str) -> String {
    code.lines()
        .filter_map(|line| {
            let trimmed = line.trim_start();
            if trimmed.starts_with("export{") || trimmed.starts_with("export {") {
                // Ren re-export-sats utan innehåll.
                return None;
            }
            match trimmed.strip_prefix("export ") {
                Some(rest) => {
                    let indent = &line[..line.len() - trimmed.len()];
                    Some(format!("{indent}{rest}"))
                }
                None => Some(line.to_string()),
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// En instans av QuickJS-i-wasm med ett laddat script.
pub struct ScriptRuntime {
    store: Store<WasiP1Ctx>,
    memory: Memory,
    alloc: TypedFunc<u32, u32>,
    load_script: TypedFunc<(u32, u32), i32>,
    update: TypedFunc<(f32, u32, u32), i32>,
    update_json: TypedFunc<(u32, u32), i32>,
    take_logs: TypedFunc<(), i32>,
    error_ptr: TypedFunc<(), u32>,
    error_len: TypedFunc<(), u32>,
    result_ptr: TypedFunc<(), u32>,
    result_len: TypedFunc<(), u32>,
    buffer: Option<(u32, usize)>,
    source_name: String,
    /// Fylls av `run`/`run_json`, töms av `take_pending_logs`. Ett internt
    /// mellanlager, eftersom `run`/`run_json` inte har tillgång till
    /// `ConsoleLog`-resursen – den bor i ett `World` som ECS-limmet äger.
    pending_logs: Vec<ymer_core::LogEntry>,
}

impl ScriptRuntime {
    /// `wasm` är den förkompilerade skriptvärden (script_host.wasm).
    pub fn new(wasm: &[u8]) -> Result<Self> {
        let engine = Engine::default();
        // wasmtime::Error är inte std::error::Error, så den mappas för hand.
        let module = Module::new(&engine, wasm)
            .map_err(|err| anyhow!("kunde inte ladda skriptmodulen: {err}"))?;

        let mut linker: Linker<WasiP1Ctx> = Linker::new(&engine);
        add_to_linker_sync(&mut linker, |ctx| ctx)?;

        // Skriptet får ingen filsystemsåtkomst och inga argument – bara stdout
        // så att console.log kan hamna någonstans.
        let wasi = WasiCtxBuilder::new()
            .inherit_stdout()
            .inherit_stderr()
            .build_p1();
        let mut store = Store::new(&engine, wasi);
        let instance: Instance = linker.instantiate(&mut store, &module)?;

        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or_else(|| anyhow!("modulen exporterar inget minne"))?;

        Ok(Self {
            alloc: instance.get_typed_func(&mut store, "alloc")?,
            load_script: instance.get_typed_func(&mut store, "load_script")?,
            update: instance.get_typed_func(&mut store, "update")?,
            update_json: instance.get_typed_func(&mut store, "update_json")?,
            take_logs: instance.get_typed_func(&mut store, "take_logs")?,
            error_ptr: instance.get_typed_func(&mut store, "error_ptr")?,
            error_len: instance.get_typed_func(&mut store, "error_len")?,
            result_ptr: instance.get_typed_func(&mut store, "result_ptr")?,
            result_len: instance.get_typed_func(&mut store, "result_len")?,
            store,
            memory,
            buffer: None,
            source_name: String::new(),
            pending_logs: Vec::new(),
        })
    }

    /// Kompilerar och laddar ett TypeScript-script. Kan köras om när som
    /// helst – hot reload är bara ett nytt anrop, eftersom allt state bor
    /// i komponenter och inget i skriptets globaler.
    pub fn load_typescript(&mut self, source: &str, name: &str) -> Result<()> {
        let javascript = typescript_to_javascript(source, name)?;
        self.source_name = name.to_string();

        let bytes = javascript.as_bytes();
        let ptr = self.alloc.call(&mut self.store, bytes.len() as u32)?;
        self.memory.write(&mut self.store, ptr as usize, bytes)?;

        let status = self
            .load_script
            .call(&mut self.store, (ptr, bytes.len() as u32))?;
        if status != 0 {
            return Err(anyhow!("{name}: {}", self.take_error()?));
        }
        Ok(())
    }

    /// Läser ut allt som loggats sedan förra anropet. Körs automatiskt av
    /// `run` och `run_json` – anropa bara direkt om du kör skriptet på
    /// något annat sätt.
    pub fn drain_logs(&mut self) -> Result<Vec<ymer_core::LogEntry>> {
        let status = self.take_logs.call(&mut self.store, ())?;
        if status != 0 {
            // Ett fel här är inte skriptets fel – bara att logga det räcker.
            let message = self.take_error()?;
            log::warn!("{}: kunde inte hämta loggar: {message}", self.source_name);
            return Ok(Vec::new());
        }

        let out_ptr = self.result_ptr.call(&mut self.store, ())?;
        let out_len = self.result_len.call(&mut self.store, ())?;
        let mut out = vec![0u8; out_len as usize];
        self.memory.read(&self.store, out_ptr as usize, &mut out)?;

        #[derive(serde::Deserialize)]
        struct RawEntry {
            level: String,
            message: String,
        }

        let raw: Vec<RawEntry> = match serde_json::from_slice(&out) {
            Ok(entries) => entries,
            Err(err) => {
                log::warn!("{}: trasig loggpost: {err}", self.source_name);
                return Ok(Vec::new());
            }
        };

        Ok(raw
            .into_iter()
            .map(|entry| ymer_core::LogEntry {
                level: match entry.level.as_str() {
                    "error" => ymer_core::LogLevel::Error,
                    "warn" => ymer_core::LogLevel::Warn,
                    "info" => ymer_core::LogLevel::Info,
                    _ => ymer_core::LogLevel::Log,
                },
                message: entry.message,
                source: self.source_name.clone(),
            })
            .collect())
    }

    /// Kör skriptets `update` över komponentdatan. `data` skrivs tillbaka
    /// med vad skriptet ändrade.
    pub fn run(&mut self, dt: f32, data: &mut [f32]) -> Result<()> {
        if data.is_empty() {
            return Ok(());
        }
        anyhow::ensure!(
            data.len().is_multiple_of(STRIDE),
            "datalängden måste vara multipel av {STRIDE}"
        );
        let count = (data.len() / STRIDE) as u32;

        let ptr = self.ensure_buffer(data.len())?;
        let bytes: &[u8] = bytemuck_cast(data);
        self.memory.write(&mut self.store, ptr as usize, bytes)?;

        let status = self.update.call(&mut self.store, (dt, count, ptr))?;
        if status != 0 {
            let message = self.take_error()?;
            return Err(anyhow!("{}: {message}", self.source_name));
        }

        let mut out = vec![0u8; data.len() * 4];
        self.memory.read(&self.store, ptr as usize, &mut out)?;
        for (index, chunk) in out.as_chunks::<4>().0.iter().enumerate() {
            data[index] = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        }
        Ok(())
    }

    /// Generella vägen: skickar komponentdata som JSON och får tillbaka
    /// ändrad data plus skriptets kommandobuffert.
    pub fn run_json(&mut self, payload: &str) -> Result<String> {
        let bytes = payload.as_bytes();
        let ptr = self.alloc.call(&mut self.store, bytes.len() as u32)?;
        self.memory.write(&mut self.store, ptr as usize, bytes)?;

        let status = self
            .update_json
            .call(&mut self.store, (ptr, bytes.len() as u32))?;
        if status != 0 {
            let message = self.take_error()?;
            return Err(anyhow!("{}: {message}", self.source_name));
        }

        let out_ptr = self.result_ptr.call(&mut self.store, ())?;
        let out_len = self.result_len.call(&mut self.store, ())?;
        let mut out = vec![0u8; out_len as usize];
        self.memory.read(&self.store, out_ptr as usize, &mut out)?;
        let result = String::from_utf8_lossy(&out).into_owned();

        // Måste ske efter att resultatet kopierats ut – take_logs skriver
        // till samma buffert i wasm-minnet.
        let logs = self.drain_logs()?;
        self.pending_logs.extend(logs);

        Ok(result)
    }

    /// Tar emot loggraderna `run`/`run_json` samlat på sig sedan senast.
    pub fn take_pending_logs(&mut self) -> Vec<ymer_core::LogEntry> {
        std::mem::take(&mut self.pending_logs)
    }

    /// Återanvänder bufferten mellan frames; växer bara när scenen gör det.
    fn ensure_buffer(&mut self, floats: usize) -> Result<u32> {
        if let Some((ptr, capacity)) = self.buffer
            && capacity >= floats
        {
            return Ok(ptr);
        }
        let bytes = (floats * 4) as u32;
        let ptr = self.alloc.call(&mut self.store, bytes)?;
        self.buffer = Some((ptr, floats));
        Ok(ptr)
    }

    fn take_error(&mut self) -> Result<String> {
        let ptr = self.error_ptr.call(&mut self.store, ())?;
        let len = self.error_len.call(&mut self.store, ())?;
        let mut bytes = vec![0u8; len as usize];
        self.memory.read(&self.store, ptr as usize, &mut bytes)?;
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }
}

/// Flyttar allt skriptet loggat sedan förra anropet in i `ConsoleLog`.
/// Tyst no-op om resursen saknas – att kräva den av alla anropare hade
/// tvingat headless-exempel som inte bryr sig om konsolen att sätta upp den.
fn push_logs(world: &mut World, runtime: &mut ScriptRuntime, fallback_source: &str) {
    let logs = runtime.take_pending_logs();
    if logs.is_empty() {
        return;
    }
    let Some(mut console) = world.get_resource_mut::<ymer_core::ConsoleLog>() else {
        return;
    };
    for mut entry in logs {
        if entry.source.is_empty() {
            entry.source = fallback_source.to_string();
        }
        console.push(entry.level, entry.message, entry.source);
    }
}

fn bytemuck_cast(data: &[f32]) -> &[u8] {
    // f32 har inga invalid bit patterns, så reinterpretationen är säker.
    unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * 4) }
}

// ------------------------------------------------------- ECS-koppling

use bevy_ecs::prelude::*;
use ymer_core::{Quat, Script, Transform, Vec3};

/// Packar ihop alla entiteter med `script`, kör deras `update` en gång,
/// och skriver tillbaka resultatet. Returnerar antalet påverkade entiteter.
pub fn run_script_system(
    world: &mut World,
    runtime: &mut ScriptRuntime,
    script: &str,
    dt: f32,
) -> Result<usize> {
    let mut targets: Vec<Entity> = Vec::new();
    let mut query = world.query::<(Entity, &Script, &Transform)>();
    for (entity, attached, _) in query.iter(world) {
        if attached.0 == script {
            targets.push(entity);
        }
    }
    if targets.is_empty() {
        return Ok(0);
    }

    let mut data = Vec::with_capacity(targets.len() * STRIDE);
    for entity in &targets {
        let transform = world.get::<Transform>(*entity).expect("filtrerad ovan");
        data.extend_from_slice(&[
            transform.translation.x,
            transform.translation.y,
            transform.translation.z,
            transform.rotation.x,
            transform.rotation.y,
            transform.rotation.z,
            transform.rotation.w,
            transform.scale.x,
            transform.scale.y,
            transform.scale.z,
        ]);
    }

    let run_result = runtime.run(dt, &mut data);
    push_logs(world, runtime, "run_script_system");
    run_result?;

    for (index, entity) in targets.iter().enumerate() {
        let offset = index * STRIDE;
        let Some(mut transform) = world.get_mut::<Transform>(*entity) else {
            continue;
        };
        transform.translation = Vec3::new(data[offset], data[offset + 1], data[offset + 2]);
        // Skript kan skriva skräp i kvaternionen; normalisera hellre än att
        // låta en trasig rotation förstöra renderingen.
        let rotation = Quat::from_xyzw(
            data[offset + 3],
            data[offset + 4],
            data[offset + 5],
            data[offset + 6],
        );
        transform.rotation = if rotation.length_squared() > 1e-6 {
            rotation.normalize()
        } else {
            Quat::IDENTITY
        };
        transform.scale = Vec3::new(data[offset + 7], data[offset + 8], data[offset + 9]);
    }

    Ok(targets.len())
}

// ------------------------------------------- generella vägen mot registret

use serde_json::{Map, Value, json};
use ymer_scene::TypeRegistry;

/// Kör ett skript med full komponentåtkomst. Skriptet får alla komponenter
/// som registret känner igen på varje matchande entitet, och kan lägga
/// spawn/despawn i en kommandobuffert som verkställs här efteråt.
///
/// Dyrare än `run_script_system` – JSON per frame istället för en platt
/// buffert – men fungerar för alla komponenttyper, inte bara `Transform`.
pub fn run_script_general(
    world: &mut World,
    runtime: &mut ScriptRuntime,
    registry: &TypeRegistry,
    script: &str,
    dt: f32,
) -> Result<usize> {
    let mut targets: Vec<Entity> = Vec::new();
    let mut query = world.query::<(Entity, &Script)>();
    for (entity, attached) in query.iter(world) {
        if attached.0 == script {
            targets.push(entity);
        }
    }
    // Ren optimering: hoppa över JSON-serialisering när inget matchar.
    // `execute_general` gör inget motsvarande tidigt-ut, eftersom en
    // konsolkommando giltigt kan ha noll targets och ändå behöva köras.
    if targets.is_empty() {
        return Ok(0);
    }

    execute_general(world, runtime, registry, script, &targets, dt)
}

/// Kör en engångskodsnutt i en fristående QuickJS-instans mot en tom
/// entitetslista. Kommandot ser fortfarande hela scenen via
/// `engine.find`/`raycast`/etc – bara `entities`-parametern är tom.
///
/// Till skillnad från skript som körs varje frame kompileras koden här
/// om varje anrop; det är rätt avvägning för en debug-konsol där
/// kommandon skrivs för hand, inte för hot paths.
pub fn run_console_command(
    world: &mut World,
    wasm: &[u8],
    registry: &TypeRegistry,
    source: &str,
) -> Result<()> {
    let mut runtime = ScriptRuntime::new(wasm)?;
    let wrapped =
        format!("export function update(dt: number, entities: Entity[]): void {{\n{source}\n}}");
    runtime.load_typescript(&wrapped, "console")?;
    execute_general(world, &mut runtime, registry, "console", &[], 0.0)?;
    Ok(())
}

fn execute_general(
    world: &mut World,
    runtime: &mut ScriptRuntime,
    registry: &TypeRegistry,
    script: &str,
    targets: &[Entity],
    dt: f32,
) -> Result<usize> {
    let mut entities = Vec::with_capacity(targets.len());
    for (index, entity) in targets.iter().enumerate() {
        let mut object = Map::new();
        object.insert("i".to_string(), json!(index));
        for component_type in registry.iter() {
            if let Some(value) = component_type.read_json(world, *entity) {
                object.insert(component_type.name.to_string(), value);
            }
        }
        entities.push(Value::Object(object));
    }

    // Ögonblicksbild av scenen som skriptets frågor arbetar mot. Bara namn,
    // position och storlek – tillräckligt för find, överlapp och raycast.
    let mut snapshot_entities: Vec<Entity> = Vec::new();
    let mut world_view: Vec<Value> = Vec::new();
    {
        let mut query = world.query::<(Entity, &Transform)>();
        for (entity, transform) in query.iter(world) {
            snapshot_entities.push(entity);
            world_view.push(json!({
                "w": world_view.len(),
                "t": [transform.translation.x, transform.translation.y, transform.translation.z],
                "s": [transform.scale.x, transform.scale.y, transform.scale.z],
            }));
        }
    }
    // Namnen hämtas separat för att slippa en andra query i samma lån.
    for (index, entity) in snapshot_entities.iter().enumerate() {
        if let Some(name) = world.get::<ymer_core::EntityName>(*entity)
            && let Some(object) = world_view[index].as_object_mut()
        {
            object.insert("name".to_string(), json!(name.0));
        }
    }

    // Input följer med i varje frame; saknas resursen kör skriptet utan den.
    let input = world
        .get_resource::<ymer_core::Input>()
        .cloned()
        .unwrap_or_default();
    let payload = serde_json::to_string(&json!({
        "dt": dt,
        "input": input,
        "entities": entities,
        "world": world_view,
    }))?;
    let raw_response = runtime.run_json(&payload);
    push_logs(world, runtime, script);
    let response: Value = serde_json::from_str(&raw_response?)?;

    // Skriv tillbaka ändrade komponenter.
    if let Some(list) = response.get("entities").and_then(Value::as_array) {
        for item in list {
            let Some(index) = item.get("i").and_then(Value::as_u64) else {
                continue;
            };
            let Some(entity) = targets.get(index as usize).copied() else {
                continue;
            };
            let Some(object) = item.as_object() else {
                continue;
            };

            for (name, value) in object {
                if name == "i" {
                    continue;
                }
                let Some(component_type) = registry.get(name) else {
                    continue;
                };
                if let Err(err) = component_type.write_json(world, entity, value.clone()) {
                    log::warn!("{script}: kunde inte skriva {name}: {err}");
                }
            }
        }
    }

    // Verkställ kommandobufferten sist, så att index i `entities` håller.
    if let Some(commands) = response.get("commands").and_then(Value::as_array) {
        for command in commands {
            match command.get("op").and_then(Value::as_str) {
                Some("spawn") => {
                    let entity = world.spawn_empty().id();
                    if let Some(components) = command.get("components").and_then(Value::as_object) {
                        for (name, value) in components {
                            let Some(component_type) = registry.get(name) else {
                                log::warn!("{script}: spawn med okänd komponent {name}");
                                continue;
                            };
                            if let Err(err) =
                                component_type.write_json(world, entity, value.clone())
                            {
                                log::warn!("{script}: spawn {name}: {err}");
                            }
                        }
                    }
                    // Allt ritbart behöver en världstransform att fyllas i.
                    if world.get::<Transform>(entity).is_some() {
                        world
                            .entity_mut(entity)
                            .insert(ymer_core::GlobalTransform::default());
                    }
                }
                Some("despawn") => {
                    // "i" är index i skriptets egna entiteter, "w" i scenbilden.
                    let entity = match (command.get("i"), command.get("w")) {
                        (Some(value), _) => value
                            .as_u64()
                            .and_then(|index| targets.get(index as usize).copied()),
                        (_, Some(value)) => value
                            .as_u64()
                            .and_then(|index| snapshot_entities.get(index as usize).copied()),
                        _ => None,
                    };
                    // try_despawn: entiteten kan redan ha tagits av en
                    // förälders despawn tidigare i samma kommandobuffert.
                    if let Some(entity) = entity
                        && let Err(err) = world.try_despawn(entity)
                    {
                        log::warn!("{script}: despawn: {err}");
                    }
                }

                Some("set") => {
                    let entity = command
                        .get("w")
                        .and_then(Value::as_u64)
                        .and_then(|index| snapshot_entities.get(index as usize).copied());
                    let Some(entity) = entity else { continue };
                    if !world.entities().contains(entity) {
                        continue;
                    }
                    if let Some(components) = command.get("components").and_then(Value::as_object) {
                        for (name, value) in components {
                            let Some(component_type) = registry.get(name) else {
                                continue;
                            };
                            if let Err(err) =
                                component_type.write_json(world, entity, value.clone())
                            {
                                log::warn!("{script}: set {name}: {err}");
                            }
                        }
                    }
                }
                other => log::warn!("{script}: okänt kommando {other:?}"),
            }
        }
    }

    Ok(targets.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exports_forsvinner() {
        let js = typescript_to_javascript(
            "export function update(dt: number): void { const x: number = dt; }",
            "test.ts",
        )
        .unwrap();
        assert!(!js.contains("export"), "export skulle ha tagits bort: {js}");
        assert!(js.contains("function update"));
    }
}
