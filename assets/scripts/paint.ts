/// <reference path="./engine.d.ts" />

// Generella vägen: skriptet ser *alla* komponenter som typregistret känner
// till, inte bara Transform. Här läses höjden och skrivs färgen, och några
// nya entiteter skapas via kommandobufferten.

let elapsed: number = 0;
let hasSpawned: boolean = false;

export function update(dt: number, entities: Entity[]): void {
  elapsed += dt;

  for (const e of entities) {
    const t = e.Transform;
    if (!t) continue;

    // Våg genom rutnätet.
    const phase: number = elapsed * 2.0 - e.i * 0.4;
    t.translation[1] = 1.4 + Math.sin(phase) * 0.9;
    engine.rotate(t.rotation, 0.2, 1.0, 0.1, dt * (0.4 + e.i * 0.05));

    // Färgen räknas ut ur höjden – en komponent läses, en annan skrivs.
    const mesh = e.MeshInstance;
    if (mesh) {
      const heat: number = (t.translation[1] - 0.5) / 1.8;
      mesh.color.r = 0.15 + heat * 0.85;
      mesh.color.g = 0.25 + Math.max(0, 1 - Math.abs(heat - 0.5) * 2) * 0.6;
      mesh.color.b = 0.95 - heat * 0.8;
    }
  }

  // Strukturella ändringar går via kommandobufferten och verkställs av
  // värden efter att update har returnerat.
  if (!hasSpawned) {
    hasSpawned = true;
    for (let i = 0; i < 5; i++) {
      const angle: number = (i / 5) * Math.PI * 2;
      engine.spawn({
        Name: "Spawnad " + i,
        Transform: {
          translation: [Math.cos(angle) * 7.5, 0.6, Math.sin(angle) * 7.5],
          rotation: [0, 0, 0, 1],
          scale: [1.6, 1.6, 1.6],
        },
        MeshInstance: { mesh: "builtin/cube", color: { r: 0.98, g: 0.85, b: 0.25, a: 1 } },
      });
    }
  }
}
