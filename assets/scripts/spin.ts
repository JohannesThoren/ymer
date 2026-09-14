/// <reference path="./engine.d.ts" />

// Skript körs som system: ett anrop per frame över alla matchande entiteter.

let elapsed: number = 0;

const AXIS = { x: 0.3, y: 1.0, z: 0.15 };

export function update(dt: number, entities: Transform[]): void {
  elapsed += dt;

  for (let i = 0; i < entities.length; i++) {
    const t: Transform = entities[i];

    // Varje kub snurrar i sin egen takt.
    const speed: number = 0.45 + i * 0.06;
    t.rotate(AXIS.x, AXIS.y, AXIS.z, dt * speed);

    // Våg genom rutnätet – syns direkt i bilden att det inte är Rust-koden.
    const phase: number = elapsed * 2.2 - i * 0.45;
    t.y = 1.3 + Math.sin(phase) * 0.8;

    const pulse: number = 1 + Math.cos(phase) * 0.18;
    t.scale = pulse;
  }
}
