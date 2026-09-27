/// <reference path="./engine.d.ts" />

// Stadens regler. Allt som går att balansera bor här, inte i Rust:
// vad en byggnad ger, hur fort folk flyttar in, hur jobben bemannas.
// Ändra en siffra, spara, och motorn laddar om skriptet.
//
// Skriptet ligger på varje byggnad *och* på Stadshuset. Skript kör som
// system, så det här är ett anrop per frame med hela listan – därför kan
// en och samma update både summera byggnaderna och bokföra i kassan.
// Ett skript kan inte läsa andra entiteters komponenter, men de här är
// alla "egna" eftersom de bär samma skript.

interface Regel {
  platser: number;
  jobb: number;
  inkomst: number;
}

const REGLER: { [kind: string]: Regel } = {
  Hus: { platser: 4, jobb: 0, inkomst: 0 },
  Butik: { platser: 0, jobb: 2, inkomst: 2 },
  Marknad: { platser: 0, jobb: 5, inkomst: 5 },
  Kontor: { platser: 0, jobb: 12, inkomst: 14 },
};

// Invånare per sekund som flyttar in när det finns ledig plats.
const INFLYTTNING = 2.0;

export function update(dt: number, entities: Entity[]): void {
  let kassa = null;
  const byggnader = [];

  for (const e of entities) {
    if (e.Stadskassa) kassa = e;
    if (e.Byggnad) byggnader.push(e);
  }
  if (!kassa) return;

  // --- vad staden har -----------------------------------------------
  let platser = 0;
  let jobb = 0;

  for (const e of byggnader) {
    const b = e.Byggnad;
    const regel = REGLER[b.kind];
    if (!regel) continue;

    // Rust sätter bara sorten; siffrorna är skriptets och skrivs
    // tillbaka så att de syns i editorns inspector.
    b.platser = regel.platser;
    b.jobb = regel.jobb;
    b.inkomst = regel.inkomst;

    platser += regel.platser;
    jobb += regel.jobb;
  }

  const k = kassa.Stadskassa;

  // --- inflyttning ---------------------------------------------------
  if (k.invanare < platser) {
    k.invanare = Math.min(platser, k.invanare + dt * INFLYTTNING);
  } else if (k.invanare > platser) {
    // Rivet boende: folk flyttar ut direkt.
    k.invanare = platser;
  }

  // --- bemanning -----------------------------------------------------
  // Arbetsplatserna bemannas i tur och ordning. Den sista kan bli
  // halvbemannad och ger då bara sin andel – det är spänningen i
  // spelet: fler jobb än folk betyder mindre per byggnad.
  let ledig = Math.floor(k.invanare);
  let inkomst = 0;

  for (const e of byggnader) {
    const b = e.Byggnad;
    if (b.jobb <= 0) {
      b.bemanning = 0;
      continue;
    }
    const fick = Math.min(ledig, b.jobb);
    ledig -= fick;
    b.bemanning = fick;
    inkomst += b.inkomst * (fick / b.jobb);
  }

  // --- bokföring ------------------------------------------------------
  k.platser = platser;
  k.jobb = jobb;
  k.sysselsatta = Math.floor(k.invanare) - ledig;
  k.inkomst = inkomst;
  k.guld += inkomst * dt;
}
