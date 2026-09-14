# Scenformat

Scener och prefabs är samma format: RON, med en post per entitet.

```ron
Scene(
  entities: [
    SceneEntity(
      id: 2,
      parent: 1,
      components: {
        "MeshInstance": (mesh:"builtin/cube",texture:"",color:(r:0.9,g:0.9,b:0.95,a:1.0)),
        "Name": ("Kub"),
        "Script": ("spin.ts"),
        "Transform": (translation:(0.0,0.5,0.0),rotation:(0.0,0.0,0.0,1.0),scale:(1.0,1.0,1.0)),
      },
    ),
  ],
)
```

`id` är lokalt för filen, inte ett `Entity`. `parent` pekar på ett annat
`id` i samma fil och utelämnas för rötter.

## Varför komponenterna står inline

Komponentvärdena lagras som `ron::value::RawValue`, som behåller RON-texten
ordagrant och serialiseras inline. Alternativen var sämre: `ron::Value`
förlorar struct-syntaxen och blir generiska maps, och strängar hade gett
citerad, escapead text i filen.

Det betyder också att scenformatet inte behöver känna till någon
komponenttyp. Filen är en lista av namn och ogenomskinliga värden;
typregistret tolkar dem vid laddning.

## Assetnamn

```ron
(mesh:"builtin/cube",texture:"textures/tegel.png",color:(...))
```

Namn, aldrig index. `builtin/cube` och `builtin/plane` finns alltid;
texturer namnges efter sin sökväg relativt projektroten. Okända namn faller
tillbaka på kuben respektive en vit pixel.

Tidigare versioner skrev råa index (`mesh:(0)`). De filerna går inte att
läsa längre – indexen berodde på laddordningen, vilket är precis problemet
namnen löser.

## Vad som inte sparas

- `GlobalTransform` – härledd data, räknas om vid laddning
- Allt registret inte känner till, inklusive resurser

## Prefabs

Samma format, sparade i `prefabs/`. Skillnaden är bara var filen ligger:
mappen avgör om en `.ron` instansieras i scenen eller laddas som hel scen.

Roten har alltid `id: 0`. Föräldralänkar utanför urvalet hoppas över.

## Projektfilen

```ron
(
    name: "Demo spel 1",
    start_scene: "scenes/main.ron",
)
```

## Ladda och spara i kod

```rust
let scene = Scene::from_world(&mut world, &registry);
scene.save("scenes/main.ron")?;

clear_scene(&mut world, &registry);
Scene::load("scenes/main.ron")?.spawn_into(&mut world, &registry)?;

let prefab = Scene::from_subtree(&mut world, &registry, entity);
```

Använd `clear_scene`, inte `World::clear_entities()` – den senare raderar
resurserna med sig, eftersom bevy_ecs 0.19 lagrar dem som komponenter.
