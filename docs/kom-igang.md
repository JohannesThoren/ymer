# Kom igång

## Krav

- Rust 1.98 eller senare (edition 2024)
- En GPU med Vulkan, Metal, DX12 eller GL

Skriptvärden `assets/script_host.wasm` följer med förbyggd. Du behöver bara
bygga om den om du ändrar i `scripthost/` – se [Bygga och testa](bygga.md).

## Starta editorn

```bash
cargo run -p ymer-editor --bin ymer
```

Kör alltid från repo-roten. Sökvägarna till `projects/` och
`assets/script_host.wasm` är relativa till arbetskatalogen, så en exe som
startas från `target/debug/` hittar varken projekt eller skriptvärd.

Vill du hoppa över launchern:

```bash
cargo run -p ymer-editor --bin ymer -- projects/demo-spel-1
```

## Ditt första projekt

1. Skriv ett namn i launchern och tryck **Skapa projekt**. Projektet öppnas
   direkt.
2. Tryck **▶ Spela**. Kuben snurrar och guppar – det är `scripts/spin.ts`
   som kör.
3. Tryck **⏸ Paus**. Scenen läggs tillbaka precis som den var.

Ett nytt projekt ser ut så här:

```text
projects/mitt-spel/
  project.ron        namn och startscen
  tsconfig.json      genererad, för din kodeditor
  scenes/main.ron    kamera, mark och en kub
  scripts/
    spin.ts          startskript
    engine.d.ts      genererad, typerna för TypeScript
  textures/
  prefabs/
```

## Ditt första skript

Skriv ett namn i filutforskarens fält och tryck **Ny fil**. Du får en
skriptmall. Öppna projektmappen i VS Code – `tsconfig.json` och
`engine.d.ts` finns redan, så autocomplete fungerar direkt.

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

Markera en entitet i hierarkin, lägg till komponenten **Script** i
inspectorn och välj din fil i listan. Tryck Spela.

Sparar du filen medan spelet kör laddas den om inom en kvarts sekund utan
att scenen rörs. Se [Skript](skript.md) för hela API:t.

## Skapa saker i scenen

| Knapp | Gör |
|---|---|
| **＋ Entitet** | Lägger en kub i origo och markerar den |
| **⭐ Prefab** | Sparar markerad entitet med alla barn i `prefabs/` |
| **💾 Spara** | Skriver scenen till filen i sökvägsfältet |
| **📂 Ladda** | Ersätter scenen med filen i sökvägsfältet |

Dubbelklick i filutforskaren: en `.ron` under `prefabs/` instansieras i
scenen, alla andra `.ron` laddas som hel scen.

Dra in en PNG från skrivbordet och släpp den på fönstret – den kopieras in
i projektet och dyker upp i inspectorns texturlista.

## Vanliga fallgropar

**"0 texturer laddade"** – du står i fel katalog, eller så har projektet
inga PNG-filer i `textures/`.

**Skriptet gör ingenting** – kolla att funktionen heter `update` och är
`export`ad, och att `Script`-komponenten pekar på rätt filnamn relativt
`scripts/`.

**`Duplicate function implementation` i VS Code** – ett skript saknar
`export`. Utan det blir filen ett globalt script och krockar med de andra.
