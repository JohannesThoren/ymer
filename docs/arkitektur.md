# Arkitektur

## Crates

| Crate | Ansvar | Beror på |
|---|---|---|
| `ymer-core` | Tid, matte, `Transform`, `Camera`, `Input`, hierarkipropagering | bevy_ecs, glam |
| `ymer-render` | wgpu: device, rendermål, meshar, texturer, pipelines, assetregister | ymer-core, wgpu, winit |
| `ymer-scene` | Typregister, RON-scener, TypeScript-generering | ymer-core, ron, serde_json |
| `ymer-script` | TS → JS (oxc), QuickJS i wasm (wasmtime), ECS-koppling | ymer-core, ymer-scene |
| `ymer-runtime` | Fönster, spelloop, ECS-värld, renderlista | allt ovan |
| `ymer-editor` | egui-editorn, gizmos, play-läge, projekt, filutforskare | allt ovan |
| `scripthost` | QuickJS kompilerad till `wasm32-wasip1` | eget workspace |

`games/sandbox` är en minimal testbädd som startar motorn utan editor.

## Dataflödet i en frame

```text
Input (winit)  ──>  Input-resurs
                         │
                    Schedule (Rust-system)
                         │
                    PlayMode::tick  ──>  QuickJS i wasm  ──>  komponenter
                         │
                    propagate_transforms  ──>  GlobalTransform
                         │
                    build_render_list  ──>  RenderList
                         │
                    Renderer::render_with_overlay  ──>  GPU
```

Varje pil är en gräns där nästa lager inte känner till det föregående.

## De tre gränser som bär allt

### Typregistret

`TypeRegistry` mappar ett komponentnamn till fyra funktionspekare: läs,
skriv, sätt default, ta bort – plus JSON-varianter av läs och skriv. Ingen
reflection, inga proc-makron, ingen `Any`-nedcastning; `register::<T>()`
monomorfiserar closures till `fn`-pekare.

Allt som behöver arbeta med komponenter generiskt går genom registret:

- Scenfiler och prefabs serialiserar det registret känner igen
- Inspectorn genererar sina fält ur `read_json`
- Skript får sina komponenter via samma JSON
- `engine.d.ts` genereras ur en default-instans per typ
- `clear_scene` vet vilka entiteter som är "scen" och vilka som inte är det

Det sista är inte kosmetik: i bevy_ecs 0.19 lagras resurser som komponenter
på entiteter, så `World::clear_entities()` raderar `Time` och `Input` med.
Registret är det som skiljer scen från infrastruktur.

### Renderlistan

`ymer-render` ser aldrig en `World`. Den matas med en `RenderList`:
view-projektion, ljusriktning och en `Vec<DrawItem>`. `ymer-runtime`
bygger listan ur ECS och slår upp assetnamn till GPU-handtag.

Därför kan editorn rita något helt annat än spelet – urval, gizmos,
förhandsvisningar – utan att renderaren behöver veta om det.

### Wasm-gränsen

Skript ser aldrig `World`. De får en platt buffert eller ett JSON-paket,
och lämnar tillbaka ändrad data plus en kommandobuffert. Strukturella
ändringar (spawn, despawn, set) verkställs av värden efter att `update`
returnerat.

Alternativet – värdfunktioner som skriptet anropar mitt i sin update –
kräver att `&mut World` ligger tillgänglig under wasm-anropet, alltså en rå
pekare i store-data och alla aliasing-problem som följer. Kommandobufferten
ger samma semantik som bevys `Commands` utan en rad `unsafe`.

## Beroenden och versioner

Alla versioner pinnas i `[workspace.dependencies]` i rot-`Cargo.toml`.

| | | |
|---|---|---|
| wgpu | 30.0 | måste matcha egui-wgpu |
| winit | 0.30.13 | måste matcha egui-winit |
| egui | 0.36 | kräver exakt wgpu 30 och winit 0.30.13 |
| bevy_ecs | 0.19 | hierarki via `ChildOf`/`Children` |
| glam | 0.33 | matte, serde för scenfiler |
| wasmtime | 48.0 | kör skriptvärden |
| oxc | 0.149 | strippar TypeScript-typer in-process |
| rquickjs | 0.13 | QuickJS inuti skriptvärden |
| ron | 0.12 | scenformat, `RawValue` för inline-komponenter |

egui-kedjan är den som styr: den kräver exakta versioner av wgpu och winit,
så börja där när något ska uppgraderas.

## Profiler

```toml
[profile.dev]
opt-level = 1

[profile.dev.package."*"]
opt-level = 3
```

Beroenden byggs optimerat även i debug. Utan det är naga, wgpu och wasmtime
outhärdligt långsamma – wasmtimes Cranelift tar över tio sekunder på att
kompilera skriptvärden i en ooptimerad debugbuild, mot under en sekund i
release.
