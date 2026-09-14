# Bygga och testa

## Bygga motorn

```bash
cargo build --workspace
```

Beroenden byggs optimerat även i debug (`[profile.dev.package."*"]`), annars
är naga, wgpu och wasmtime outhärdligt långsamma. Ren `cargo build` av hela
workspacet tar ett par minuter första gången, sekunder därefter.

## Bygga skriptvärden

`assets/script_host.wasm` följer med förbyggd (782 KB, hela QuickJS). Bygg
om den bara om du ändrar `scripthost/src/lib.rs` – till exempel för att
lägga till en ny host-funktion i `engine`-objektet.

```bash
cd scripthost
rustup target add wasm32-wasip1

# Ubuntu/Debian: wasi-libc ligger utspritt över systemet, clang vill ha en
# sammanhållen sysroot.
apt-get install clang lld wasi-libc
mkdir -p /tmp/wasi-sysroot/include /tmp/wasi-sysroot/lib/wasm32-wasi
cp -r /usr/include/wasm32-wasi/* /tmp/wasi-sysroot/include/
cp -r /usr/lib/wasm32-wasi/*     /tmp/wasi-sysroot/lib/wasm32-wasi/

CC_wasm32_wasip1=clang \
CFLAGS_wasm32_wasip1="--target=wasm32-wasi --sysroot=/tmp/wasi-sysroot" \
  cargo build --release --target wasm32-wasip1

cp target/wasm32-wasip1/release/script_host.wasm ../assets/
```

`scripthost` är ett eget Cargo-workspace, skilt från motorns, eftersom det
kompilerar till en annan target.

## Rendera utan skärm

Ingen del av motorn kräver ett fönster. Varje crate med grafik har ett
headless-exempel som renderar till en offscreen-textur:

```bash
cargo run -p ymer-runtime --example headless
cargo run -p ymer-runtime --example typescript
cargo run -p ymer-runtime --example typescript_components
cargo run -p ymer-runtime --example typescript_input
cargo run -p ymer-runtime --example typescript_queries
cargo run -p ymer-runtime --example scene_roundtrip
cargo run -p ymer-editor --example headless_editor
cargo run -p ymer-editor --example launcher_demo
cargo run -p ymer-editor --example prefab_demo
cargo run -p ymer-editor --example hot_reload
```

På en Linux-server utan GPU, kör mot Mesas mjukvaru-Vulkan:

```bash
apt-get install mesa-vulkan-drivers vulkan-tools
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json \
  cargo run -p ymer-runtime --example headless
```

`scene_roundtrip` är samtidigt ett regressionstest: bygger en scen, sparar
den, laddar den i en tom värld, animerar båda lika långt och jämför
pixlarna byte för byte. `hot_reload` renderar samma värld två gånger med
två versioner av samma skriptfil emellan.

## Enhetstester

```bash
cargo test --workspace
```

`ymer-script` har ett test som kontrollerar att `export` faktiskt
försvinner efter TS-till-JS-kompileringen – regression mot bugen där
skript-som-moduler krockade med QuickJS som kör dem som script.

## Prestandamätning

```bash
cargo run --release -p ymer-runtime --example bench
```

Bygger en scen med 5000 entiteter, alla med samma skript, och mäter varje
steg i frame-loopen separat. Kör alltid med `--release` – debug-siffror för
wasmtime och naga är meningslösa.

Uppmätta värden, release-läge, mjukvaru-Vulkan (llvmpipe – ingen riktig
GPU i den här containern; räkna med väsentligt bättre `render`-tal på
faktisk hårdvara, resten är CPU-bundet och flyttas inte av det):

```
5004 entiteter

propagate_transforms                   0.271 ms/frame
build_render_list                      0.075 ms/frame
render (sortering + draw calls)       19.664 ms/frame
registerskanning (hierarki)            3.889 ms/frame
Scene::from_world                     18.276 ms/frame
run_script_system (platt buffert)     18.052 ms/frame
run_script_general (JSON)            253.534 ms/frame

clear_scene tog bort 5006 entiteter
```

Tre saker värda att läsa ut ur detta:

**`run_script_general` är omkring 14× dyrare än `run_script_system`** vid
den här storleken (253,5 / 18,0). Skalar linjärt med antal entiteter, så vid
5000 objekt märks det: 253 ms är långt över en frame-budget. Skript som bara
rör `Transform` ska gå via den platta vägen; den generella är för
gameplay-täta scener med hundratals, inte tusentals, aktiva skriptobjekt.

**`registerskanning` och `Scene::from_world` är O(entiteter × registerstorlek).**
Båda går igenom varje entitet och frågar varje registrerad komponenttyp om
den finns. Med fem komponenttyper och 5000 entiteter blir det 25 000
uppslag för en enda hierarkiritning. Det här är samma kod som
`clear_scene` använder för att skilja scen från resurser – prisvärt vid
demostorlek, en tydlig optimeringskandidat om scener växer mot tusentals
objekt. En bitmask eller ett cachat medlemskap per entitet skulle göra det
till en O(1)-slagning.

**`render` dominerar ändå.** 19,7 ms för 5000 texturerade kuber på en
mjukvaru-rasterizer är förväntat – ingen frustum culling finns än, så varje
objekt skickas till GPU:n oavsett om det syns. På riktig GPU-hårdvara blir
den siffran mycket lägre; det gör inte de CPU-bundna stegen ovanför det.

## Diskutrymme

wasmtime, oxc och wgpu tillsammans bygger stort. `cargo clean` innan du
byter mellan release och debug om utrymmet är knappt – `target/` för hela
workspacet kan nå 8–10 GB med debuginfo.
