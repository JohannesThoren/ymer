# Ymer på Nintendo 3DS

Motorns tredje renderarbackend, efter wgpu och mjukvarurasteriseraren.
Egen workspace, som `scripthost` — skrivbordsbygget ska inte behöva känna
till en tier 3-plattform.

## Vad som krävs

```sh
# devkitPro med 3DS-verktygen
brew install --cask devkitpro-pacman          # macOS
sudo dkp-pacman -S 3ds-dev                    # libctru, citro3d, picasso, 3dslink

# Rust-sidan
cargo install cargo-3ds
```

`rust-toolchain.toml` ber om nightly med `rust-src`; `armv6k-nintendo-3ds`
är tier 3, så std byggs ur källkoden med `-Z build-std`. Det sköts av
`.cargo/config.toml` och behöver inget av dig.

Samma fil pekar ut `DEVKITPRO=/opt/devkitpro`, för `citro3d-macros` läser
den variabeln med `env!` — vid kompilering, inte vid körning — och
devkitPro sätter den bara i skalprofilen. Ligger devkitPro någon
annanstans hos dig räcker det att exportera variabeln i skalet: ett värde
som redan finns vinner över filens.

## Bygga och köra

```sh
cd 3ds
cargo 3ds build --release        # ger ymer-3ds.3dsx
cargo 3ds run --release          # via 3dslink till en konsol i nätverket
```

För Citra/Azahar: öppna `target/armv6k-nintendo-3ds/release/ymer-3ds.3dsx`.

## Vad demon visar

Samma scen som skrivbordets `cargo run -p ymer-render-soft --example
topscreen`: rutig mark, sex kuber i en trappa, en list i skärmrymd.
Kameran svänger långsamt. START avslutar.

Att det är *samma* scen är avsiktligt. Lägg bilderna bredvid varandra —
håller sömmen ser de likadana ut, och skiljer de sig säger skillnaden var
felet sitter.

## Hur den skiljer sig från skrivbordet

**Ljuset räknas per vertex.** PICA200 har ingen fragment-shader, bara en
kombinator med sex steg. Motorns ljusformel bor därför i
`assets/ymer.v.pica` och resultatet kommer ut som vertexfärg. Gouraud i
stället för Phong: på kuber identiskt, på mjukt skuggade meshar bryts
ljuset vid hörnen.

Det betyder också att formeln finns på två ställen — i `ymer_gfx::shade`
och i shadern. Ändras den ena måste den andra följa med. Det är den
dyraste dubbleringen i den här mappen.

**Texturer måste vara tvåpotenser** mellan 8 och 1024, lagrade i
8x8-brickor med Morton-ordning och ABGR-byteordning. `ymer-pica` sköter
omläggningen och är testad på skrivbordet — tolv tester, för det är den
sortens kod där ett fel ger en bild som ser nästan rätt ut.

**Index är 16 bitar.** En mesh med fler än 65 536 hörn avvisas när den
laddas, med ett meddelande, i stället för att tyst tappa geometri.

**Skärmen är roterad.** Framebufferten är 240x400, inte 400x240.

## Oprövat

Fortfarande inte kompilerat — devkitPro är blockerat från miljön koden
skrevs i. Men API:t är sedan dess läst ur `rust3ds`-källkoden och inte
bara ur dokumentationen, så listan är kortare än den var. Det som
bekräftats: `Matrix4::from_rows` (tar `[FVec4; 4]`), `draw_elements`
(kräver `Vec<I, LinearAllocator>`), `Info::add` (äger bufferten),
`TextureParameters::new_2d`, `Face`, `texenv`-kedjan och
`bind_vertex_uniform` (`From<&Matrix4>` finns).

| Vad | Var | Varför osäkert |
|---|---|---|
| Skärmrotationen | `Screen::projection` och `Screen::ortho` | Jag lägger på ett kvarts varv för hand. citro3d har `ScreenOrientation` och `AspectRatio` som kanske gör det åt en — då blir bilden vriden två gånger. Syns direkt: scenen ligger på högkant. |
| Djuptestet | hela `draw` | citro3d:s standardläge används. Motorn vill ha tre lägen (testa+skriv, bara testa, strunta i). Genomskinligt och gränssnitt kan därför hamna fel i djupled. |
| UV:ns v-riktning | `ymer_pica::tile_rgba8` | Ingen vändning görs. Kommer texturerna upp och ner är det raden som skriver `out[to..]` som ska läsa `height - 1 - y`. |
| Byteordningen ABGR | samma funktion | Dokumentationen säger `Rgba8`; hårdvaran lagrar ABGR. Är rött och blått utbytta är det de fyra raderna. |
| Alfablandning | `draw` | Ingen blandning sätts upp. Listen i skärmrymd har alfa 0.85 och blir troligen ogenomskinlig. |

Ordningen i tabellen är den jag skulle felsöka i. Skärmrotationen avgör
om något ritas alls; resten avgör om det ritas *rätt*.

En sak till som inte går att veta utan hårdvara: shadern i
`assets/ymer.v.pica` är skriven men aldrig assemblerad. `picasso` är
noggrann med swizzles och registerbredder, så räkna med ett par
rättningar där första gången.

## När det inte startar

`Console::new` på undre skärmen är kvar med flit: `println!` hamnar där.
Går GPU-starten fel skrivs felet ut och programmet väntar på START i
stället för att stänga sig, så meddelandet hinner läsas.
