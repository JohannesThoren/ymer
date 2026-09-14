# Ymer – dokumentation

En 2D/3D-spelmotor i Rust med wgpu, bevy_ecs och TypeScript-skript som körs
i QuickJS inuti WebAssembly.

## Börja här

| | |
|---|---|
| [Kom igång](kom-igang.md) | Bygg motorn, skapa ett projekt, skriv ditt första skript |
| [Editorn](editorn.md) | Panelerna, gizmos, play-läget, filutforskaren |
| [Skript](skript.md) | TypeScript-API:t: komponenter, input, scenfrågor, kommandon |

## Referens

| | |
|---|---|
| [Arkitektur](arkitektur.md) | Crates, dataflöde och varför gränserna går där de går |
| [Komponenter](komponenter.md) | Alla inbyggda komponenter och hur du registrerar egna |
| [Scenformat](scenformat.md) | RON-scener, prefabs och assetnamn |
| [Bygga och testa](bygga.md) | Skriptvärden, headless-rendering, prestandamätning |
| [glTF-import](assets.md) | Riktiga 3D-modeller: geometri, material, nodhierarki -> prefab |

## Vad motorn gör

- Renderar 3D med wgpu på Vulkan, Metal, DX12 eller GL
- ECS med hierarki via bevy_ecs
- Editor med hierarki, typad inspector, gizmos och filutforskare
- Scener och prefabs som läsbar RON
- Gameplay i TypeScript – motorn kompilerar och kör den själv, ingen Node
- Hot reload av skript under körning
- Typdeklarationer för din kodeditor, genererade ur motorns eget typregister

## Vad motorn inte gör än

- glTF-import finns (geometri, material, hierarki -> prefab), men bara
  base color, inget animation eller skelett
- Ingen fysik, inga mipmaps, inget ljud
- Ingen frustum culling
- 2D finns bara i den meningen att man kan platta till en kub
