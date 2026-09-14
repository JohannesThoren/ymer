# glTF-import

Riktiga 3D-modeller importeras från `.gltf`/`.glb` och blir prefabs – samma
format och samma instansieringsväg som allt annat i editorn.

## Hur det fungerar

1. Släpp en fil på editorfönstret – den hamnar i `models/` oavsett var i
   utforskaren du står – eller lägg den där själv.
2. Motorn läser geometri, material och nodhierarki med `gltf`-craten.
3. Varje mesh laddas upp och registreras i assetregistret som
   `"{sökväg}#{meshnamn}"`, t.ex. `"models/robot.glb#Body"`.
4. Har meshens material en `baseColorTexture` laddas den upp och registreras
   som `"{sökväg}#tex{bildindex}"`.
5. Nodträdet byggs om till en `Scene` – varje nod blir en entitet med
   `Transform`, `Name` och (om noden har en mesh) `MeshInstance`. Barn får
   `ChildOf` mot sin förälder.
6. Scenen sparas som `prefabs/<filnamn>.ron`.

Öppnar du projektet på nytt laddas alla `.glb`/`.gltf` i projektet om
automatiskt – hela trädet skannas, inte bara `models/`, så en fil som
hamnat fel ändå fungerar – så assetnamnen finns även om ingen ny prefab behöver skrivas –
mesh-handtag är runtime-data och överlever inte en omstart, men namnen i
scenfilerna gör det.

## Material

Bara `pbrMetallicRoughness.baseColorTexture` och `.baseColorFactor` läses.

- Har materialet en textur: den registreras och `MeshInstance.texture`
  pekar på den. `baseColorFactor` blir ändå `MeshInstance.color` och tonar
  texturen, precis som ett handmålat objekt.
- Har materialet ingen textur: `baseColorFactor` blir hela färgen.

Ett mesh med flera primitiver och olika material per primitiv får bara det
första primitivets material – `MeshInstance` har exakt en textur och en
färg. Dela upp modellen i flera meshar i din 3D-mjukvara om du behöver mer
än ett material.

## Vad som inte importeras

- Metallic/roughness-, normal-, emissive- och occlusion-texturer
- Animationer och skelett
- Kameror och ljus definierade i filen
- Flera scener i samma fil (bara `defaultScene`, eller första scenen)

## Kodvägen

```rust
// Ren data, ingen kunskap om Scene – lever i ymer-render.
let asset: GltfAsset = ymer_render::import_gltf(&mut renderer, &mut assets, path, "models/robot.glb")?;

// Bygger prefaben, lever i ymer-editor.
let prefab_path = ymer_editor::gltf_import::import_as_prefab(&mut renderer, &mut assets, project_root, path)?;
```

Uppdelningen finns för att hålla `ymer-render` oberoende av scenformatet –
samma princip som att renderaren aldrig ser en `World`.
