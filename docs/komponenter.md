# Komponenter

## Inbyggda

### `Transform`

Lokal transform relativt förälder, eller relativt världen om entiteten är
rot.

```ron
(translation:(0.0,0.5,0.0),rotation:(0.0,0.0,0.0,1.0),scale:(1.0,1.0,1.0))
```

Inspectorn visar rotationen som eulervinklar i grader. Kvaternionen ligger
kvar i filen – grader är bara en vy, eftersom en kvaternion inte går att
skriva för hand.

### `GlobalTransform`

Världstransform, uträknad av `propagate_transforms` varje frame. **Sparas
aldrig och ska aldrig skrivas till** – ändra `Transform` i stället.

### `MeshInstance`

Vad som ritas.

```ron
(mesh:"builtin/cube",texture:"textures/tegel.png",color:(r:1.0,g:1.0,b:1.0,a:1.0))
```

`mesh` och `texture` är assetnamn, inte index. Inbyggda meshar heter
`builtin/cube` och `builtin/plane`. Importerade glTF-meshar namnges
`"{sökväg}#{meshnamn}"`, t.ex. `"models/robot.glb#Head"`. Texturer namnges
efter sin sökväg relativt projektroten, eller `"{glTF-sökväg}#tex{index}"`
för texturer inbäddade i en glTF-fil. Tom textursträng ger en vit pixel,
alltså ren färg.

Okända namn faller tillbaka på kuben respektive vitt, så en scen med
stavfel går att öppna och rätta.

### `Camera`

```ron
(fov_y_radians:1.0471976,z_near:0.1,z_far:1000.0)
```

Vyn tas från entitetens `GlobalTransform`. Första kameran i världen
används; det finns ingen prioritetsordning än.

### `EntityName` (heter `Name` i filer och skript)

Läsbart namn. Driver hierarkivyn, `engine.find` och prefabfilnamn.

### `Script`

Sökväg till en `.ts`-fil, relativt `scripts/`. Alla entiteter med samma
skript uppdateras i ett anrop.

### Hierarki

`ChildOf` och `Children` kommer från bevy_ecs och underhålls automatiskt –
spawna med `ChildOf(förälder)` så sköter resten sig själv. De skrivs till
scenfiler som `parent:` på entiteten.

## Resurser

| Resurs | Innehåll |
|---|---|
| `Time` | `delta_seconds`, `elapsed_seconds`, `frame`. `advance_by` stegar manuellt för deterministiska tester |
| `Input` | Tangenter och mus. `end_frame()` nollar engångshändelser |

I bevy_ecs 0.19 lagras resurser som komponenter på entiteter. Därför finns
`clear_scene(world, registry)` – `World::clear_entities()` skulle radera
`Time` och `Input` med.

## Registrera egna komponenter

En typ behöver `Component`, `Serialize`, `Deserialize`, `Default` och
`Clone`:

```rust
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct Health {
    pub current: f32,
    pub max: f32,
}

impl Default for Health {
    fn default() -> Self {
        Self { current: 100.0, max: 100.0 }
    }
}

registry.register::<Health>("Health");
```

Det enda anropet ger dig:

- fält i inspectorn, härledda ur JSON-formen
- serialisering i scener och prefabs
- `Health`-fältet i `Entity` för skript
- en `Health`-interface i `engine.d.ts`
- knappen `+ Health` i Lägg till komponent

Namnet är det som hamnar i scenfiler och skript, så byt det inte i onödan.

## Hur inspectorn väljer widget

| JSON-form | Widget |
|---|---|
| flyttal | dragfält, finare steg för `*_radians` |
| heltal | dragfält i heltalssteg |
| bool | kryssruta |
| sträng | textfält |
| `[x,y,z]` | tre fält på rad |
| `{r,g,b,a}` | färgväljare |
| objekt | rader med fältnamn |

Tre namnbaserade undantag ligger ovanpå formen: `rotation` med fyra tal blir
eulervinklar, `Script` blir en lista över projektets `.ts`-filer, och
`texture` en lista över projektets PNG-filer.

Inspectorn känner inte till intervall – den vet inte att en skala bör vara
positiv. Kryssrutan **RON-läge** växlar till råtext för det fälten inte når.
