/// <reference path="./engine.d.ts" />

// Rutnätet är en enhet stort, och allt ligger på hela koordinater.
const CELL = 1.0;

// Stängd dörr står på golvet, öppen har sjunkit under det. Dörrens läge
// bärs av dess höjd: ett skript kan inte läsa andra entiteters
// komponenter, bara namn, position och skala ur scenbilden.
const CLOSED_HEIGHT = 0.0;
const OPEN_HEIGHT = -1.2;

/// Spelaren flyttar en ruta per tangenttryck. Står en låda i vägen skjuts
/// den, om rutan bakom är fri.
export function update(dt: number, entities: Entity[]): void {
  const step = readStep();

  for (const e of entities) {
    const t = e.Transform;
    if (!t) continue;
    if (step) move(t, step);
    // Varje frame, inte bara när något trycks: scenbilden värden skickar
    // är från *före* skriptets egna skrivningar, så en låda som just
    // knuffats upp på plattan syns först nästa frame.
    updateDoors(t);
  }

  report();
}

function readStep() {
  const input = engine.input;
  if (input.justPressed("KeyW") || input.justPressed("ArrowUp")) return [0, -CELL];
  if (input.justPressed("KeyS") || input.justPressed("ArrowDown")) return [0, CELL];
  if (input.justPressed("KeyA") || input.justPressed("ArrowLeft")) return [-CELL, 0];
  if (input.justPressed("KeyD") || input.justPressed("ArrowRight")) return [CELL, 0];
  return null;
}

function move(t, step) {
  const target = [t.translation[0] + step[0], t.translation[2] + step[1]];
  if (wallAt(target)) return;

  const crate = crateAt(target);
  if (crate) {
    const beyond = [target[0] + step[0], target[1] + step[1]];
    if (wallAt(beyond) || crateAt(beyond)) return;
    // Scenfrågorna ger position och storlek, inte rotation – en skjuten
    // låda får därför identitetsrotation tillbaka.
    engine.set(crate, {
      Transform: {
        translation: [beyond[0], crate.t[1], beyond[1]],
        rotation: [0, 0, 0, 1],
        scale: crate.s,
      },
    });
  }

  t.translation[0] = target[0];
  t.translation[2] = target[1];
}

function wallAt(cell) {
  if (atCell(engine.findAll("Vägg"), cell) !== null) return true;
  // En öppen dörr har sjunkit ner i golvet och spärrar inte längre.
  const door = atCell(engine.findAll("Dörr"), cell);
  return door !== null && door.t[1] > OPEN_HEIGHT + 0.1;
}

function crateAt(cell) {
  return atCell(engine.findAll("Låda"), cell);
}

function atCell(candidates, cell) {
  for (const entry of candidates) {
    if (Math.abs(entry.t[0] - cell[0]) < 0.25 && Math.abs(entry.t[2] - cell[1]) < 0.25) {
      return entry;
    }
  }
  return null;
}

/// Dörrar följer plattorna: står en låda eller spelaren på en platta är
/// alla dörrar öppna.
function updateDoors(playerTransform): void {
  const plates = engine.findAll("Platta");
  const crates = engine.findAll("Låda");
  let pressed = false;

  for (const plate of plates) {
    const cell = [plate.t[0], plate.t[2]];
    if (atCell(crates, cell) !== null) {
      pressed = true;
      break;
    }
    if (
      Math.abs(playerTransform.translation[0] - cell[0]) < 0.25 &&
      Math.abs(playerTransform.translation[2] - cell[1]) < 0.25
    ) {
      pressed = true;
      break;
    }
  }

  const height = pressed ? OPEN_HEIGHT : CLOSED_HEIGHT;
  for (const door of engine.findAll("Dörr")) {
    if (Math.abs(door.t[1] - height) < 0.01) continue;
    // Grinden står vriden ett kvarts varv; rotationen finns inte i
    // scenbilden, så den måste skrivas tillbaka för hand.
    engine.set(door, {
      Transform: {
        translation: [door.t[0], height, door.t[2]],
        rotation: [0, 0.7071068, 0, 0.7071068],
        scale: door.s,
      },
    });
  }
}

/// Loggar när alla lådor står på mål. En gång, inte varje frame.
let solved = false;

function report() {
  const crates = engine.findAll("Låda");
  const goals = engine.findAll("Mål");
  let onGoal = 0;
  for (const crate of crates) {
    for (const goal of goals) {
      if (Math.abs(crate.t[0] - goal.t[0]) < 0.25 && Math.abs(crate.t[2] - goal.t[2]) < 0.25) {
        onGoal++;
        break;
      }
    }
  }

  if (onGoal === goals.length && goals.length > 0) {
    if (!solved) {
      solved = true;
      console.log("Klart! Alla lådor står på mål.");
    }
  } else {
    solved = false;
  }
}
