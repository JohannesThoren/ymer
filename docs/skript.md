# Skript

Gameplay skrivs i TypeScript. Motorn strippar typerna med oxc in-process
och kör koden i QuickJS inuti WebAssembly, via wasmtime. Ingen Node, ingen
extern kompilator, inget byggsteg.

## Livscykeln

1. Du sätter en `Script`-komponent på en entitet och pekar på en fil i
   `scripts/`.
2. Play-läget kompilerar filen och skapar en QuickJS-instans per fil.
3. Varje frame anropas `update` **en gång** med alla entiteter som har just
   det skriptet.
4. Skriptet ändrar komponentdata och lägger strukturella ändringar i en
   kommandobuffert.
5. Värden skriver tillbaka ändringarna och verkställer kommandona.

Skript körs alltså som system, inte som objektbeteenden. Har hundra
entiteter samma skript blir det ett anrop per frame, inte hundra.

## Formen på ett skript

```ts
export function update(dt: number, entities: Entity[]): void {
  for (const e of entities) {
    const t = e.Transform;
    if (!t) continue;
    engine.rotate(t.rotation, 0, 1, 0, dt);
  }
}
```

`export` krävs för att din kodeditor ska se varje fil som en egen modul –
annars krockar `update` och alla toppnivåvariabler mellan skript. Motorn
plockar bort nyckelordet innan koden evalueras.

**State hör hemma i komponenter, inte i skriptets globaler.** En global
`let elapsed` fungerar, men nollställs vid hot reload. Allt som ska överleva
en omladdning ska ligga i en komponent.

## Entiteter

Varje element i `entities` speglar de komponenter registret känner till:

```ts
interface Entity {
  i: number;              // index i den här framens lista
  Transform?: Transform;
  MeshInstance?: MeshInstance;
  Name?: string;
  Script?: string;
  // ... plus spelets egna registrerade komponenter
}
```

Fälten är samma data som i Rust, serialiserad till JSON. `Vec3` blir
`[x, y, z]`, kvaternioner `[x, y, z, w]`, färger `{r, g, b, a}`.

Skrivningar går tillbaka till ECS när `update` returnerat. Kvaternioner
normaliseras av värden, så du kan inte förstöra en rotation genom slarv.

## `engine`

### Matte

```ts
engine.rotate(q, ax, ay, az, radians)   // roterar kvaternion på plats
```

### Input

```ts
engine.input.isDown("KeyW")
engine.input.justPressed("Space")
engine.input.justReleased("KeyE")
engine.input.mouseIsDown("Left")
engine.input.mouse            // { x, y, dx, dy }
engine.input.axis("KeyA", "KeyD")   // -1, 0 eller 1
```

Tangenter namnges som i webbens `KeyboardEvent.code` – `"KeyW"`, `"Space"`,
`"ArrowLeft"` – vilket råkar vara exakt winits `KeyCode`-namn. Samma sträng
fungerar i Rust, i skript och i en framtida webbversion.

`justPressed` gäller bara den frame tangenten trycktes ned, även om den
hålls kvar.

### Scenfrågor

```ts
engine.find("Spelare")              // WorldEntry | null
engine.findAll("Mynt")              // WorldEntry[]
engine.distance(a, b)
engine.overlaps(a, b)               // axelinriktat, struntar i rotation
engine.raycast(origin, dir, maxDistance?, skipName?)
```

Frågorna arbetar mot en ögonblicksbild av scenen som följer med varje
frame: namn, position och storlek för allt med en `Transform`. Kostnaden är
linjär i scenens storlek och `raycast` går igenom varenda låda. Det duger
för hobbyscener men inte för tusentals objekt.

```ts
const hit = engine.raycast([x, 0.35, z], [dx, 0, dz], 9, "Spelare");
if (hit) console.log(hit.entry.name, hit.distance);
```

Skjut inte strålen från spelarens position om du testar mot låga objekt –
är spelaren i luften går strålen rakt över dem.

### Kommandon

```ts
engine.spawn({
  Name: "Mynt",
  Transform: { translation: [0, 1, 0], rotation: [0, 0, 0, 1], scale: [1, 1, 1] },
  MeshInstance: { mesh: "builtin/cube", texture: "", color: { r: 1, g: 0.8, b: 0.2, a: 1 } },
});

engine.despawn(entityOrWorldEntry);
engine.set(worldEntry, { MeshInstance: { ... } });
```

Kommandon verkställs efter att `update` returnerat, så index i `entities`
håller hela framen. `engine.set` är vägen att ändra entiteter som inte har
ditt skript.

## Två vägar över wasm-gränsen

| | `run_script_system` | `run_script_general` |
|---|---|---|
| Data | platt f32-buffert, bara `Transform` | JSON, alla registrerade komponenter |
| Kostnad (29 entiteter) | 0,22 ms/frame | 2,16 ms/frame |
| Input och scenfrågor | nej | ja |
| Spawn och despawn | nej | ja |

Editorns play-läge använder den generella vägen. Den platta finns kvar för
system som bara flyttar transformer och behöver vara billiga.

## Typer i din kodeditor

`scripts/engine.d.ts` och `tsconfig.json` genereras ur typregistret varje
gång projektet öppnas. Registrerar spelet en egen komponent dyker den upp i
autocomplete nästa gång – ingen handskriven deklaration att hålla i synk.

```bash
cd projects/mitt-spel && tsc --noEmit
```

Ingen typkontroll sker vid körning. Motorn strippar bara typerna; `tsc` vid
sidan om är det som fångar felen.

## Hot reload

Ändrade filer upptäcks inom 250 ms och kompileras om utan att scenen rörs.
Det är mtime-pollning, inte inotify: de flesta editorer sparar atomärt genom
att skriva en temporärfil och byta namn, vilket filbevakare rapporterar
olika på olika plattformar.

Ett trasigt skript pausar körningen och visar felet i statusfältet. Det
kraschar aldrig editorn.

## Begränsningar

- Ingen `import` mellan skript – varje fil evalueras som fristående script
- Inga host-anrop mitt i `update`; allt går via data in och kommandon ut
- Ingen bytecode-cache: varje laddning parsar om källkoden
- Skript kan inte spela ljud eller ladda assets
