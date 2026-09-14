# Ymer

En 2D/3D-spelmotor i Rust. Gameplay skrivs i TypeScript som motorn
kompilerar och kör själv – ingen Node, inget byggsteg, ingen extern
verktygskedja.

```ts
export function update(dt: number, entities: Entity[]): void {
  for (const e of entities) {
    const t = e.Transform;
    if (!t) continue;

    const x = engine.input.axis("KeyA", "KeyD");
    t.translation[0] += x * 5 * dt;
  }
}
```

Spara filen medan spelet kör så laddas den om inom en kvarts sekund, utan
att scenen rörs.

## Vad den gör

- **Rendering** med wgpu – Vulkan, Metal, DX12 eller GL
- **ECS** med hierarki via bevy_ecs
- **Editor** med hierarki, typad inspector, gizmos och filutforskare
- **TypeScript-skript** i QuickJS inuti WebAssembly, sandboxat
- **Hot reload** av skript under körning
- **2D och 3D** – sprites med spritesheets och ortografisk kamera, eller
  glTF-modeller med material
- **Scener och prefabs** som läsbar RON
- **Export** till ett körbart spel: tre filer, ingen Rust hos spelaren
- **Typdeklarationer** för din kodeditor, genererade ur motorns typregister

## Snabbstart

```bash
cargo run -p ymer-editor --bin ymer
```

Skapa ett projekt i launchern och tryck **▶ Spela**. Ett nytt projekt kommer
med en scen och ett skript som fungerar direkt.

Fullständig dokumentation i [`docs/`](docs/README.md): kom igång, editorn,
skript-API:t, komponenter, scenformat, glTF-import, samt att bygga och testa.

## Status

Motorn är under aktiv utveckling och används inte i produktion. Den går att
bygga spel i, men saknar fortfarande fysik och ljud, och API:er kan ändras.

Kända begränsningar finns i [`docs/`](docs/README.md) under respektive
avsnitt – bland annat att glTF-import bara läser base color, att kollision
är AABB utan rotation, och att scenfrågor i skript är linjära i scenens
storlek.

## Köra

```bash
cargo run -p sandbox
```

Tvinga Vulkan-backend och se mer logg:

```bash
WGPU_BACKEND=vulkan RUST_LOG=info cargo run -p sandbox
```

Esc eller fönsterkryss stänger. FPS loggas en gång per sekund via ett ECS-system,
vilket är beviset på att schemat faktiskt körs.

## Utan skärm (CI, container, SSH)

```bash
```bash
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json \
  cargo run -p ymer-runtime --example headless
```

Renderar demo-scenen till en textur och skriver rå RGBA till `/tmp/frame.raw`.
Klockan stegas manuellt (`Time::advance_by`) så bilden blir bitidentisk varje
körning. Fungerar på mjukvaru-Vulkan (lavapipe) – bra som röktest i CI.

## Layout

| crate | ansvar |
|---|---|
| `ymer-core` | tid, matte (glam), färg, `Transform`, `Camera`, hierarki-propagering. Vet inget om GPU. |
| `ymer-render` | wgpu: device, queue, rendermål (fönster eller offscreen), meshes, pipelines. Ser aldrig en `World` – matas med en `RenderList`. |
| `ymer-runtime` | fönster (winit), spelloop, ECS-värld och schema. |
| `ymer-script` | TS -> JS (oxc) och QuickJS-i-wasm via wasmtime. Packar komponenter till en platt f32-buffert. |
| `ymer-editor` | egui-editorn: hierarki, inspector genererad ur registret, spara/ladda. Ritas som overlay i motorns render-pass. |
| `ymer-scene` | typregister och scenformat (RON). Navet som editor och skript-ABI läser från. |
| `ymer-pak` | arkivformatet och assetkällorna (lösa filer eller `.pak`). |
| `games/ymer-play` | spelrunnern: kör ett exporterat projekt. |
| `games/sandbox` | testbädd som startar motorn utan editor. |
| `scripthost` | QuickJS kompilerad till `wasm32-wasip1`. Eget workspace. |

Versioner pinnas i `[workspace.dependencies]` i rot-`Cargo.toml`. wgpu 30 och
winit 0.30.13 är valda för att matcha egui 0.36.

## Vägen framåt

4. **Gizmos, forts.** – rotations- och skalhandtag, snapping, flerval.
5. **Scripting, forts.** – deklarerade komponentbehov, bytecode-cache,
   input och frågor mot scenen från skript.
6. **Assets, forts.** – mipmaps, animation/skelett, fler PBR-kanaler,
   materialkomponent skild från `MeshInstance` för multi-material-meshar.

Typregistret i steg 3 är navet: både editorns UI och WASM-ABI:t läser från det.
Allt som läggs till före det bör redan gå via registret.

## glTF-import

Släpp en `.gltf`/`.glb` på fönstret så hamnar den i `models/`, geometri och
material laddas upp, och en prefab skrivs till `prefabs/<namn>.ron`.
Filutforskaren hoppar till mappen filen hamnade i.

Släppta filer sorteras efter typ oavsett var du står i utforskaren:
`.glb`/`.gltf` till `models/`, bilder till `textures/`, `.ts` till
`scripts/`. Okända filtyper hamnar i mappen du står i.
Dubbelklicka prefaben för att placera modellen i scenen – exakt samma väg
som alla andra prefabs.

Nodhierarkin i filen blir entiteter med `ChildOf`, precis som allt annat:
en robot med separata mesh-noder för kropp och huvud blir en förälder-barn-
relation i scenen, inte en sammanslagen mesh.

Varje mesh får sitt material med sig – `baseColorTexture` blir en registrerad
textur, `baseColorFactor` blir `MeshInstance.color`. Har meshen ingen textur
bär färgfaktorn allt tonval; har den en, tonar färgen texturen precis som i
resten av motorn.

```bash
cargo run -p ymer-editor --example gltf_demo -- sokvag/till/modell.glb
```

**Begränsningar:** ett material per mesh (multi-material-meshar tappar alla
utom det första), ingen animation, inga skelett, inga normal- eller
metallic/roughness-texturer – bara base color. `builtin/cube` och
`builtin/plane` finns kvar som ritverktyg och platshållare.

## Assets och texturer

Komponenter refererar till **namn**, aldrig till index:

```ron
"MeshInstance": (mesh:"builtin/cube",texture:"textures/tegel.png",color:(...)),
```

`Assets` slår upp namnet till ett GPU-handtag en gång per frame och entitet.
Okända namn faller tillbaka på kuben respektive en vit 1×1-textur, så en scen
med ett stavfel går att öppna och rätta i editorn istället för att krascha.

Texturer laddas från projektets `textures/` när projektet öppnas och namnges
efter sin sökväg relativt projektroten – samma sträng som står i scenfilen.
Drar du in en PNG i utforskaren dyker den upp i inspectorns texturlista.

Otexturerade objekt binder den vita pixeln, så **en enda pipeline** ritar
båda fallen. Ritanropen sorteras på `(textur, mesh)` så att materialbytena
blir så få som möjligt.

## Prefabs

`Scene::from_subtree` serialiserar en entitet och alla dess barn. Roten får
alltid id 0, så instansieringen vet vad den ska returnera och markera.
Föräldralänkar utanför urvalet hoppas över – en prefab ska kunna placeras
var som helst.

```bash
cargo run -p ymer-editor --example prefab_demo
```

Filändelsen räcker inte för att veta vad en `.ron` är, så **mappen avgör**:
en `.ron` under `prefabs/` instansieras i den öppna scenen, allt annat laddas
som en hel scen. Enkel regel, inga metadatafiler.

Flyttar du rotens `Transform` efter instansiering följer barnen med, eftersom
hierarkin är relativ – det är hela poängen med `propagate_transforms`.

## Inspectorn

Komponenterna har inget schema – men `read_json` ger deras *form*, och det
räcker längre än man tror:

| JSON | widget |
|---|---|
| tal (flyttal) | dragfält, finare steg för `*_radians` |
| tal (heltal) | dragfält i heltalssteg |
| bool | kryssruta |
| sträng | textfält |
| `[x,y,z]` | tre fält på rad |
| `{r,g,b,a}` | färgväljare |
| objekt | rader med fältnamn |

Ovanpå formen ligger tre namnbaserade specialfall: `rotation` med fyra tal
redigeras som **eulervinklar i grader** (kvaternioner går inte att skriva
för hand), `Script` får en lista över projektets `.ts`-filer, och heltal
dras i heltalssteg.

Ändringar skrivs direkt till världen – ingen Applicera-knapp att glömma.
Kryssrutan **RON-läge** växlar tillbaka till råtext för allt fälten inte når.

Nya komponenttyper får sina fält automatiskt. Registrerar spelet en egen
komponent med serde dyker den upp med rätt widgets utan en rad UI-kod.

## Export

```bash
cargo build --release -p ymer-play     # en gång
# sedan: 📦 Exportera i editorn
```

Resultatet är tre filer och ingenting annat:

```text
Testspel/
  Testspel[.exe]     runnern, alltså motorn
  script_host.wasm   QuickJS
  game.pak           allt spelinnehåll
```

Spelaren behöver inte Rust, inte motorn, ingenting. `engine.d.ts`,
`tsconfig.json` och tidigare `.pak`-exporter filtreras bort – de hör till
utvecklingen, inte till spelet.

**Ingen dynamisk länkning.** Runnern *är* motorn, och spelet är data. Att
uppdatera motorn i ett utgivet spel betyder att byta ut runnern och
skriptvärden; arkivet rörs inte. Rust saknar stabil ABI, så en `.dll`-lösning
hade bytt en filkopiering mot risken att `World` i biblioteket och `World` i
exe:n tyst är olika typer. `format_version` i arkivheadern är kontraktet i
stället: en nyare runner läser äldre arkiv, och ett arkiv som är nyare än
runnern avvisas med ett begripligt fel.

Debug-konsolen är `#[cfg(debug_assertions)]` – den finns i utvecklingsbyggen
och kompileras inte in i släppta spel.

## Arkivformatet

`.pak` är ett eget format: header, data, index sist.

| | |
|---|---|
| Komprimering | zstd per fil, bara när den lönar sig |
| Integritet | crc32 per fil |
| Ordning | sorterad, så samma indata ger samma arkiv |
| Version | `format_version` i headern |

Beslutet tas per fil, inte globalt. Uppmätt på ett testprojekt: `.glb` krympte
75 %, RON 47 %, TypeScript 29 %, medan PNG lagrades rått – zstd hade gjort
dem större.

```bash
cargo run -p ymer-pak --example pack -- <katalog> <arkiv.pak>
```

## Assetkällor

`AssetSource` i `ymer-core` har två implementationer: `LooseFiles`
(editorn) och `PakArchive` (exporterade spel). `Scene::load_from`,
`Assets::load_textures_from`, `ScriptHost::from_source` och
`import_gltf_bytes` går alla genom den.

Ett test i `ymer-pak` ställer samma frågor till båda och kräver identiska
svar. Utan det får man buggar som bara uppstår i exporterade spel, vilket är
den värsta sorten att felsöka.

`.glb` fungerar ur arkiv eftersom allt är inbäddat. En `.gltf` som refererar
externa filer gör det inte – det finns ingen katalog att leta i.

## Projekt

```text
projects/demo-spel-1/
  project.ron        namn och startscen
  scenes/main.ron
  scripts/*.ts
  textures/*.png
  prefabs/*.ron
```

Editorn startar i launchern: välj ett projekt eller skapa ett nytt. Ett nytt
projekt scaffoldas med en spelbar startscen (kamera, mark, en kub) och ett
startskript, så att Spela fungerar direkt.

```bash
cargo run -p ymer-editor --bin ymer
cargo run -p ymer-editor --bin ymer -- projects/demo-spel-1
```

Filutforskaren längst ner bläddrar i projektroten, skapar skriptfiler och
mappar, och tar emot filer som **släpps på fönstret** – de kopieras in i
projektet. Editorn refererar aldrig till filer utanför projektroten, annars
går projektet inte att flytta eller dela.

Dubbelklick på en `.ron` laddar den som scen; Ladda ersätter numera scenen
istället för att lägga den ovanpå.

## Editorn

```bash
cargo run -p ymer-editor --bin ymer
```

Hierarki till vänster, inspector till höger, scenen bakom. Inspectorn visar
varje komponent som redigerbar RON-text; `Applicera` skriver tillbaka via
registret, och trasig syntax visas i statusfältet istället för att krascha.
`Lägg till komponent` listar allt registret känner till som entiteten saknar.

Utan skärm:

```bash
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json \
  cargo run -p ymer-editor --example headless_editor
```

## Typstöd i din editor

När ett projekt öppnas skriver motorn `scripts/engine.d.ts` och
`tsconfig.json`. Deklarationerna **genereras ur typregistret**, inte för
hand, så de kan aldrig glida isär från motorn: registrerar spelet en egen
komponent dyker den upp i autocomplete nästa gång projektet öppnas.

Formen härleds ur en default-instans serialiserad till JSON – samma trick
som den typade inspectorn använder. `Vec3` blir `[number, number, number]`,
newtypes som `Name` och `Script` blir `string`:

```ts
declare interface Transform {
  rotation: [number, number, number, number];
  scale: [number, number, number];
  translation: [number, number, number];
}
```

`tsc --noEmit` i projektroten typkollar allt, och VS Code läser samma
tsconfig utan extra installation.

### Skripten är moduler

Varje skript är en ES-modul och exporterar sin `update`:

```ts
export function update(dt: number, entities: Entity[]): void { ... }
```

Det är nödvändigt för editorn: som globala script hamnar alla filer i
samma scope, och två skript som båda deklarerar `update` eller `let elapsed`
ger `Duplicate function implementation`. Som moduler får varje fil egen
scope. Motorn plockar bort `export` innan koden evalueras, eftersom QuickJS
kör den som script – varje skript har ändå en egen instans.

## Skript

```bash
cargo run -p ymer-runtime --example typescript
```

Lägg en `.ts`-fil i `assets/scripts/` och sätt en `Script`-komponent på
entiteten. Skriptet definierar en global `update(dt, entities)` – ingen
`export`, eftersom koden evalueras som script och inte som ESM.

```ts
function update(dt: number, entities: Transform[]): void {
  for (const t of entities) t.rotate(0, 1, 0, dt * 1.5);
}
```

Typerna i `assets/scripts/engine.d.ts` finns bara för editorn och tsc.
Körningen strippar all typinformation och gör ingen typkontroll.

Värden packar alla matchande entiteters `Transform` till en platt f32-buffert
(stride 10: translation, rotation xyzw, skala), skriver den i modulens minne
och gör **ett** anrop per frame. Preludet i skriptmodulen bygger
`Transform`-vyer över bufferten.

All state ska ligga i komponenter, inte i skriptets globaler – då är hot
reload bara ett nytt `load_typescript`-anrop.

### Två vägar över wasm-gränsen

| | `run_script_system` | `run_script_general` |
|---|---|---|
| data | platt f32-buffert, bara `Transform` | JSON, alla registrerade komponenter |
| kostnad (29 entiteter) | 0,22 ms/frame | 2,16 ms/frame |
| kan spawna/despawna | nej | ja |

Den generella vägen är tio gånger dyrare och värd varenda mikrosekund när
skriptet behöver läsa `MeshInstance` och skriva färger. Hot paths som bara
rör transformer bör ligga kvar på den platta bufferten.

```bash
cargo run -p ymer-runtime --example typescript_components
```

Strukturella ändringar går via kommandobufferten, aldrig direkt mot världen:

```ts
engine.spawn({
  Name: "Spawnad",
  Transform: { translation: [0, 1, 0], rotation: [0, 0, 0, 1], scale: [1, 1, 1] },
  MeshInstance: { mesh: 0, color: { r: 1, g: 0.8, b: 0.2, a: 1 } },
});
```

Kommandona verkställs av värden efter att `update` returnerat, så entitets-
index håller under hela frame:n – samma modell som bevy Commands.

### Input

Tangenter namnges som webbens `KeyboardEvent.code` – `"KeyW"`, `"Space"`,
`"ArrowLeft"` – vilket råkar vara exakt winits `KeyCode`-namn. Samma sträng
fungerar därför i Rust, i skript och i en framtida webbversion.

```ts
const x = engine.input.axis("KeyA", "KeyD");
if (engine.input.justPressed("Space") && grounded) velocityY = JUMP;
```

```bash
cargo run -p ymer-runtime --example typescript_input
```

Exemplet kör utan fönster och matar in ett inspelat tangentmakro, vilket
också är hur inputberoende gameplay går att regressionstesta: samma makro in,
samma bild ut.

### Play-läget

Play-knappen tar en ögonblicksbild av scenen med `Scene::from_world`, laddar
alla skript som används i den, och kör dem varje frame. Stopp slänger allt
skripten hann göra och bygger upp scenen igen från ögonblicksbilden.

Utan den återställningen skulle varje testkörning förstöra scenen man just
byggt – skript flyttar saker, ändrar färger och despawnar.

Ett trasigt skript pausar körningen och visar felet i statusfältet; det
kraschar aldrig editorn. `PlayMode::reload` läser om en enskild fil utan att
röra scenen, vilket räcker som hot reload eftersom allt state ligger i
komponenter.

Rensningen använder `clear_scene`, inte `World::clear_entities`. Den senare
raderar **resurserna med**, eftersom bevy_ecs 0.19 lagrar dem som komponenter
på entiteter – `Time` och `Input` försvinner och nästa frame panikar.
`clear_scene` tar bara bort entiteter som typregistret känner igen.

### Hot reload

`PlayMode::poll_reloads` jämför filernas mtime mot vad som laddades och
kompilerar om det som ändrats. Scenen rörs inte – allt state ligger i
komponenter, så kuberna står kvar där de var när den nya koden tar över.

Det är mtime-pollning, inte inotify. Skälet: de flesta editorer sparar
atomärt genom att skriva en temporärfil och byta namn, vilket filbevakare
rapporterar olika på olika plattformar och ofta som "borttagen" plus
"skapad" istället för "ändrad". Ett stat-anrop per skript var 250:e ms är
osynligt i jämförelse, och kan inte missa en ändring.

```bash
cargo run -p ymer-editor --example hot_reload
```

Exemplet renderar samma värld två gånger med två versioner av samma
skriptfil och skriver om filen mitt emellan.

### Gizmos

Vänsterklick i vyn skjuter en stråle från kameran och plockar närmaste
entitet vars AABB träffas. Träffytan kommer från meshens verkliga storlek –
`MeshRegistry` räknar ut halva utsträckningen när meshen laddas upp, vilket
också är vad frustum culling kommer att behöva.

Handtagen ritas i ett **overlay-pass** med `depth_compare: Always` och
avstängd djupskrivning, så de syns genom geometri. Storleken skalas med
avståndet till kameran så att gizmot ser lika stort ut oavsett var objektet
står.

Dragningen är närmaste punkt mellan två linjer: musstrålen och axeln.
Greppunktens offset sparas när man trycker ned, så objektet hoppar inte
till pekaren.

```bash
cargo run -p ymer-editor --example headless_editor
```

Exemplet simulerar ett klick och en dragning utan fönster: det projicerar en
känd kub till skärmen, skjuter strålen genom den pixeln, kontrollerar att
plockningen hittar tillbaka till samma entitet, och drar sedan Y-handtaget
90 pixlar uppåt.

### Scenfrågor

Varje frame följer en ögonblicksbild av scenen med i payloaden: namn,
position och skala för allt som har en `Transform`. Skriptens frågor räknas
i JS mot den bilden.

```ts
for (const coin of engine.findAll("Mynt")) {
  if (engine.overlaps(player, coin)) engine.despawn(coin);
}

const hit = engine.raycast(eye, direction, 9, "Player");
if (hit) engine.set(hit.entry, { MeshInstance: { mesh: 0, color: red } });
```

```bash
cargo run -p ymer-runtime --example typescript_queries
```

Kostnaden växer linjärt med scenen, och `raycast` går igenom varje AABB.
Det duger för hobbyscener men inte för tusentals objekt – då hör frågorna
hemma i värden bakom ett spatialt index.

## Scener

```bash
cargo run -p ymer-runtime --example scene_roundtrip
```

Bygger demo-scenen, sparar den som RON, laddar den i en tom värld, animerar båda
lika långt och jämför pixlarna byte för byte. Sedan redigeras komponenter enbart
via registret – läs som RON-text, ändra texten, skriv tillbaka – vilket är exakt
den väg editorn kommer att gå.

`MeshId` sparas som råindex i renderarens meshregister. Det håller bara så länge
scenen laddas i samma ordning; riktiga asset-sökvägar kommer i milstolpe 5.

## Kända luckor

- Den generella vägen skickar *alla* registrerade komponenter varje frame,
  även de skriptet inte rör. Skriptet borde deklarera vad det behöver.
- Ingen bytecode-cache: varje `load_typescript` parsar om källkoden, och
  wasmtime kompilerar om modulen vid varje start (`Module::serialize` löser det).
- Skript kan inte spela ljud.
- Scenfrågorna är linjära: ingen BVH, inget spatialt index, ingen riktig fysik.
- Kollision är AABB mot AABB och struntar i rotation – samma gäller plockningen.
- Gizmot flyttar bara; rotation och skala får man skriva i inspectorn.
- Inga intervall på reglagen: inspectorn vet inte att en skala bör vara
  positiv eller att fov rimligen ligger under 180°.
- Inga mipmaps: texturer flimrar på håll.
- glTF: ett material per mesh, ingen animation, inga skelett, bara base color.
- Prefabs har ingen koppling tillbaka: ändrar du prefabfilen uppdateras inte
  redan utplacerade instanser.
- Editorns hierarki visar bara entiteter med registrerade komponenter,
  eftersom resurser annars dyker upp där som namnlösa rader.
- Hot reload upptäcker bara filer som redan är laddade; ett helt nytt
  skriptnamn kräver att entiteten får sin `Script`-komponent först.
- `just_pressed` nollas av `Input::end_frame`, som anropas av `App` – kör du
  egna loopar måste du anropa den själv.
- Ingen typkontroll, bara strippning. `tsc --noEmit` vid sidan om är din vän.

### Renderaren

- Normalmatrisen antar uniform skalning; icke-uniform skala ger fel ljus.
- Ingen frustum culling, ingen sprite-/2D-väg, inga texturer.
- `propagate_transforms` går igenom hela hierarkin varje frame utan
  change detection.

## Anteckningar

- `Renderer::render` slukar övergående surface-tillstånd (timeout, minimerat,
  outdated) och returnerar bara fel vid förlorad surface. Att återskapa surfacen
  vid `SurfaceLost` är ännu inte implementerat.
- Beroenden byggs med `opt-level = 3` även i debug (`[profile.dev.package."*"]`),
  annars är naga och wgpu outhärdligt långsamma.

## Licens

LGJT License v1 – se [LICENSE](LICENSE).

Kort sammanfattat: du får ladda ner, läsa och köra koden privat och
icke-kommersiellt. Allt annat – kommersiell användning, hosting, att bygga
vidare på koden, att återanvända delar i andra projekt – kräver skriftligt
tillstånd i förväg. Att repot är publikt ger i sig ingen användningsrätt.

Beroendena har sina egna licenser, se
[THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md).
