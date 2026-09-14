# Editorn

```bash
cargo run -p ymer-editor --bin ymer
```

## Launchern

Första vyn listar projekt i `projects/` och skapar nya. Ett nytt projekt
scaffoldas med en spelbar startscen och ett startskript, så att Spela
fungerar direkt.

## Panelerna

**Verktygsfältet** — Spela/Paus, ＋ Entitet, ⭐ Prefab, Spara, Ladda,
scensökväg, entitetsräknare och statusmeddelanden.

**Hierarkin** — alla entiteter registret känner igen, barn indenterade
under sina föräldrar. Klick markerar.

**Inspectorn** — komponenterna på markerad entitet som fält, reglage,
färgväljare och filväljare. Ändringar skrivs direkt till världen; det finns
ingen Applicera-knapp att glömma. Kryssrutan RON-läge växlar till råtext.

**Filutforskaren** — bläddrar i projektroten, skapar filer och mappar,
tar emot filer som släpps på fönstret. Dubbelklick öppnar: en `.ron` under
`prefabs/` instansieras, andra `.ron` laddas som scen.

## Markera och flytta

Vänsterklick i vyn skjuter en stråle från kameran och plockar närmaste
entitet vars låda träffas. Träffytan kommer från meshens verkliga storlek,
så ett stort plan och en liten kub beter sig rätt trots samma `scale`.

Gizmots tre armar dras för att flytta längs X, Y eller Z. Handtagen ritas i
ett overlay-pass utan djuptest, så de syns genom geometri, och skalas med
avståndet till kameran så att de ser lika stora ut överallt.

Handtagen har företräde framför objekt bakom dem. Greppunktens offset sparas
vid nedtryckning, så objektet hoppar inte till pekaren.

Rotation och skala saknar handtag – de skrivs i inspectorn.

## Play-läget

Spela tar en ögonblicksbild av scenen, laddar alla skript som används i den
och kör dem varje frame. Paus slänger allt skripten hann göra och bygger upp
scenen igen.

Utan den återställningen skulle varje testkörning förstöra scenen du just
byggt. Skripten flyttar saker, målar om dem och despawnar.

Under körning pollas skriptfilerna var 250:e ms. Sparar du en `.ts` laddas
den om utan att scenen rörs eller körningen avbryts.

Ett trasigt skript pausar och visar felet i statusfältet.

## Prefabs

⭐ Prefab sparar markerad entitet **med alla barn** som
`prefabs/<Namn>.ron`. Roten får id 0 i filen, så instansieringen vet vad den
ska markera. Föräldralänkar utanför urvalet hoppas över, så prefaben kan
placeras var som helst.

Dubbelklicka på filen för att instansiera. Flyttar du rotens `Transform`
följer barnen med.

Prefabs är kopior utan koppling tillbaka: ändrar du filen uppdateras inte
redan utplacerade instanser.

## Texturer

Släpp en PNG på fönstret så kopieras den in i projektet, till den mapp du
står i. Editorn refererar aldrig till filer utanför projektroten – annars
går projektet inte att flytta eller dela.

Texturer laddas när projektet öppnas och namnges efter sin sökväg relativt
projektroten. Samma sträng står i scenfilen och i inspectorns lista.

## Headless

Hela editorn går att rendera utan skärm, vilket används för
regressionstester:

```bash
cargo run -p ymer-editor --example headless_editor
cargo run -p ymer-editor --example launcher_demo
cargo run -p ymer-editor --example prefab_demo
cargo run -p ymer-editor --example hot_reload
```

Se [Bygga och testa](bygga.md).
