//! Bevisar att console.log/warn/error i skript landar i ConsoleLog.
//!
//!     cargo run -p ymer-runtime --example console_log

use ymer_core::{ConsoleLog, EntityName, Input, LogLevel, Script, Time};
use ymer_runtime::prelude::*;
use ymer_scene::{TypeRegistry, register_builtin_types};
use ymer_script::{ScriptRuntime, run_script_general};

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    let mut registry = TypeRegistry::new();
    register_builtin_types(&mut registry);

    let mut world = World::new();
    world.insert_resource(Time::new());
    world.insert_resource(Input::default());
    world.insert_resource(ConsoleLog::default());

    world.spawn((
        EntityName::new("Testobjekt"),
        Transform::IDENTITY,
        Script::new("console_log_demo.ts"),
    ));

    let wasm = std::fs::read("assets/script_host.wasm")?;
    // Skriptet bor i exemplet – det är ett test, inte en motor-asset.
    let source = r#"
export function update(dt: number, entities: Entity[]): void {
  console.log("uppdaterar", entities.length, "entiteter, dt =", dt);
  console.warn("en varning");
  console.error("ett fel:", { kod: 42 });
}
"#;
    let mut runtime = ScriptRuntime::new(&wasm)?;
    runtime.load_typescript(source, "console_log_demo.ts")?;

    run_script_general(
        &mut world,
        &mut runtime,
        &registry,
        "console_log_demo.ts",
        1.0 / 60.0,
    )?;

    let log = world.resource::<ConsoleLog>();
    println!("{} rader i konsolen:", log.len());
    for entry in log.entries() {
        let level = match entry.level {
            LogLevel::Log => "LOG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        };
        println!("  [{level:5}] {:12} {}", entry.source, entry.message);
    }
    Ok(())
}
