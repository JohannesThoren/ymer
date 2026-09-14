//! Genererar `engine.d.ts` ur typregistret.
//!
//! Poängen är att deklarationerna aldrig kan glida isär från motorn:
//! samma register som ritar inspectorn och läser scenfiler beskriver
//! komponenterna för TypeScript. Registrerar spelet en egen komponent
//! dyker den upp i editorns autocomplete nästa gång projektet öppnas.
//!
//! Formen härleds ur en default-instans serialiserad till JSON – exakt
//! samma trick som den typade inspectorn använder.

use bevy_ecs::prelude::*;
use serde_json::Value;

use crate::TypeRegistry;

/// Hela innehållet i `engine.d.ts`.
pub fn definitions(registry: &TypeRegistry) -> String {
    let mut world = World::new();
    let scratch = world.spawn_empty().id();

    let mut interfaces = String::new();
    let mut entity_fields = String::new();

    for component in registry.iter() {
        component.insert_default(&mut world, scratch);
        let Some(value) = component.read_json(&world, scratch) else {
            component.remove(&mut world, scratch);
            continue;
        };
        component.remove(&mut world, scratch);

        match &value {
            // Newtypes (Name, Script) är bara sitt innehåll – ingen
            // interface behövs, fältet får primitivtypen direkt.
            Value::Object(_) => {
                interfaces.push_str(&format!(
                    "/** Komponenten `{name}`. */\ndeclare interface {name} {body}\n\n",
                    name = component.name,
                    body = object_body(&value, 0),
                ));
                entity_fields.push_str(&format!("  {}?: {};\n", component.name, component.name));
            }
            other => {
                entity_fields.push_str(&format!("  {}?: {};\n", component.name, type_of(other)));
            }
        }
    }

    format!(
        "{HEADER}{interfaces}declare interface Entity {{\n  /** Index i den här framens lista. */\n  i: number;\n{entity_fields}}}\n{FOOTER}"
    )
}

/// `tsconfig.json` som gör att VS Code hittar deklarationerna och
/// behandlar skripten som globala script, inte som ES-moduler.
pub fn tsconfig() -> &'static str {
    r#"{
  "compilerOptions": {
    "target": "ES2020",
    "lib": ["ES2020"],
    "types": [],
    "strict": true,
    "noEmit": true,
    "noImplicitAny": false,
    "skipLibCheck": true,
    "module": "esnext",
    "moduleResolution": "bundler",
    "moduleDetection": "force"
  },
  "include": ["scripts/**/*.ts"]
}
"#
}

fn type_of(value: &Value) -> String {
    match value {
        Value::Bool(_) => "boolean".to_string(),
        Value::Number(_) => "number".to_string(),
        Value::String(_) => "string".to_string(),
        Value::Null => "unknown".to_string(),
        Value::Array(items) => {
            if items.is_empty() {
                return "unknown[]".to_string();
            }
            // Fast längd ger tupel: [number, number, number] fångar att en
            // Vec3 har exakt tre komponenter.
            if items.len() <= 4 && items.iter().all(Value::is_number) {
                let parts = vec!["number"; items.len()].join(", ");
                return format!("[{parts}]");
            }
            format!("{}[]", type_of(&items[0]))
        }
        Value::Object(_) => object_body(value, 1),
    }
}

fn object_body(value: &Value, depth: usize) -> String {
    let Value::Object(map) = value else {
        return "unknown".to_string();
    };

    let indent = "  ".repeat(depth + 1);
    let closing = "  ".repeat(depth);

    let mut body = String::from("{\n");
    for (key, field) in map {
        body.push_str(&format!("{indent}{key}: {};\n", type_of(field)));
    }
    body.push_str(&format!("{closing}}}"));
    body
}

const HEADER: &str = r#"// AUTOGENERERAD av motorn – skriv inte i den här filen.
// Innehållet kommer från typregistret och skrivs om varje gång projektet
// öppnas. Lägg egna typer i en separat .d.ts.

"#;

const FOOTER: &str = r#"
/** En entitet i scenen, sedd av skriptens frågor. */
declare interface WorldEntry {
  /** Index i scenbilden – används av despawn och set. */
  w: number;
  name?: string;
  /** Position i världsrymd. */
  t: [number, number, number];
  /** Storlek, används vid överlapp och raycast. */
  s: [number, number, number];
}

declare interface EngineInput {
  /** Hålls tangenten nere? Namn som i KeyboardEvent.code: "KeyW", "Space". */
  isDown(key: string): boolean;
  /** Trycktes den ned den här framen? */
  justPressed(key: string): boolean;
  justReleased(key: string): boolean;
  mouseIsDown(button: string): boolean;
  mouse: { x: number; y: number; dx: number; dy: number };
  /** -1, 0 eller 1 från ett tangentpar, t.ex. axis("KeyA", "KeyD"). */
  axis(negative: string, positive: string): number;
}

declare const engine: {
  input: EngineInput;

  /** Roterar en kvaternion [x,y,z,w] på plats kring en axel. */
  rotate(q: number[], ax: number, ay: number, az: number, radians: number): number[];

  /** Skapar en entitet. Verkställs när update returnerat. */
  spawn(components: Record<string, unknown>): void;
  /** Tar bort en entitet, egen eller från en scenfråga. */
  despawn(entity: Entity | WorldEntry): void;
  /** Skriver komponenter på vilken entitet som helst i scenen. */
  set(entry: WorldEntry, components: Record<string, unknown>): void;

  /** Första entiteten med exakt det namnet. */
  find(name: string): WorldEntry | null;
  /** Alla entiteter vars namn innehåller fragmentet. */
  findAll(fragment: string): WorldEntry[];
  distance(a: Entity | WorldEntry, b: Entity | WorldEntry): number;
  /** Axelinriktat överlappstest. */
  overlaps(a: Entity | WorldEntry, b: Entity | WorldEntry): boolean;
  /** Närmaste AABB-träff längs strålen. */
  raycast(
    origin: number[],
    direction: number[],
    maxDistance?: number,
    skipName?: string,
  ): { entry: WorldEntry; distance: number } | null;
};

/**
 * Signaturen varje skript ska exportera:
 *
 * ```ts
 * export function update(dt: number, entities: Entity[]): void { ... }
 * ```
 *
 * Varje skriptfil är en egen modul, så två skript kan använda samma
 * variabelnamn utan att krocka. Motorn plockar bort `export` innan koden
 * evalueras – varje skript kör ändå i en egen QuickJS-instans.
 */
declare type UpdateFn = (dt: number, entities: Entity[]) => void;
"#;
