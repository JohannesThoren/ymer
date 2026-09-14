/// <reference path="./engine.d.ts" />

// Gameplay som faktiskt använder scenen: spelaren plockar mynt genom
// överlappstest, och en raycast framåt markerar nästa mynt i sikte.

const SPEED: number = 6.0;
const JUMP: number = 7.5;
const GRAVITY: number = -18.0;

let velocityY: number = 0;
let grounded: boolean = true;
let collected: number = 0;

export function update(dt: number, entities: Entity[]): void {
  for (const e of entities) {
    const t = e.Transform;
    if (!t) continue;

    const x: number = engine.input.axis("KeyA", "KeyD");
    const z: number = engine.input.axis("KeyW", "KeyS");

    t.translation[0] += x * SPEED * dt;
    t.translation[2] += z * SPEED * dt;

    if (engine.input.justPressed("Space") && grounded) {
      velocityY = JUMP;
      grounded = false;
    }
    velocityY += GRAVITY * dt;
    t.translation[1] += velocityY * dt;
    if (t.translation[1] <= 0) {
      t.translation[1] = 0;
      velocityY = 0;
      grounded = true;
    }

    // Plocka allt som heter Mynt och överlappar spelaren.
    const coins = engine.findAll("Mynt");
    for (const coin of coins) {
      if (engine.overlaps(e, coin)) {
        engine.despawn(coin);
        collected += 1;
      }
    }

    // Raycast i rörelseriktningen: nästa mynt i sikte blir rött.
    if (x !== 0 || z !== 0) {
      // Utgå från mynthöjd – annars missar strålen medan spelaren är i luften.
      const eye: number[] = [t.translation[0], 0.35, t.translation[2]];
      const hit = engine.raycast(eye, [x, 0, z], 9, "Player");
      if (hit && hit.entry.name && hit.entry.name.indexOf("Mynt") !== -1) {
        engine.set(hit.entry, {
          MeshInstance: { mesh: "builtin/cube", color: { r: 0.95, g: 0.2, b: 0.2, a: 1 } },
        });
      }
    }
  }
}
