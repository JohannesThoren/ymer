//! QuickJS inuti wasm. Motorn laddar den här modulen en gång och matar in
//! användarens script plus en platt f32-buffert med komponentdata per frame.
//!
//! ABI (allt är index i modulens linjära minne):
//!   alloc(len) -> ptr          reservera buffert som värden skriver i
//!   load_script(ptr, len) -> i32   evaluera JS-källkod, 0 = ok
//!   update(dt, count, ptr) -> i32  kör skriptets update över `count` entiteter
//!   error_ptr() / error_len()      senaste felmeddelandet som UTF-8
//!
//! Bufferten är `count * STRIDE` f32: translation(3), rotation xyzw(4), scale(3).

use std::cell::RefCell;

use rquickjs::{Context, Function, Runtime, TypedArray};

/// Antal f32 per entitet. Matchar `Transform` i ymer-core.
const STRIDE: usize = 10;

thread_local! {
    static VM: RefCell<Option<(Runtime, Context)>> = const { RefCell::new(None) };
    static LAST_ERROR: RefCell<String> = const { RefCell::new(String::new()) };
}

/// Körs före användarens kod. Ger skripten objekt istället för råa flyttal.
const PRELUDE: &str = r#"
globalThis.__STRIDE = 10;

// --- console -------------------------------------------------------------
// Rader samlas i JS och hämtas av värden efter varje anrop – samma modell
// som kommandobufferten. console.log skriver inte direkt till någonstans;
// det finns inget "någonstans" inuti wasm-sandlådan.
globalThis.__logs = [];

function __format(value) {
    if (typeof value === "string") return value;
    if (value instanceof Error) return value.stack || value.message;
    try {
        return JSON.stringify(value);
    } catch (e) {
        return String(value);
    }
}

function __log(level, args) {
    const message = Array.prototype.map.call(args, __format).join(" ");
    globalThis.__logs.push({ level: level, message: message });
    // Tak här också – ett skript i en oändlig loop ska inte svälja allt minne
    // innan värden ens hunnit dra i nödbromsen.
    if (globalThis.__logs.length > 1000) globalThis.__logs.shift();
}

globalThis.console = {
    log: function () { __log("log", arguments); },
    info: function () { __log("info", arguments); },
    warn: function () { __log("warn", arguments); },
    error: function () { __log("error", arguments); },
};

globalThis.__engine_take_logs = function () {
    const logs = globalThis.__logs;
    globalThis.__logs = [];
    return JSON.stringify(logs);
};

class Transform {
    constructor(buffer, index) {
        this.b = buffer;
        this.o = index * globalThis.__STRIDE;
    }

    get x() { return this.b[this.o]; }
    set x(v) { this.b[this.o] = v; }
    get y() { return this.b[this.o + 1]; }
    set y(v) { this.b[this.o + 1] = v; }
    get z() { return this.b[this.o + 2]; }
    set z(v) { this.b[this.o + 2] = v; }

    translate(dx, dy, dz) {
        this.b[this.o] += dx;
        this.b[this.o + 1] += dy;
        this.b[this.o + 2] += dz;
    }

    get scale() { return this.b[this.o + 7]; }
    set scale(v) {
        this.b[this.o + 7] = v;
        this.b[this.o + 8] = v;
        this.b[this.o + 9] = v;
    }

    /// Roterar kring en axel i lokal rymd. Kvaternionmultiplikation q = delta * q.
    rotate(ax, ay, az, radians) {
        const len = Math.hypot(ax, ay, az);
        if (len === 0) return;
        const s = Math.sin(radians * 0.5) / len;
        const dx = ax * s, dy = ay * s, dz = az * s, dw = Math.cos(radians * 0.5);

        const o = this.o + 3;
        const qx = this.b[o], qy = this.b[o + 1], qz = this.b[o + 2], qw = this.b[o + 3];

        let rx = dw * qx + dx * qw + dy * qz - dz * qy;
        let ry = dw * qy - dx * qz + dy * qw + dz * qx;
        let rz = dw * qz + dx * qy - dy * qx + dz * qw;
        let rw = dw * qw - dx * qx - dy * qy - dz * qz;

        // Normalisera, annars driver kvaternionen isär efter några tusen frames.
        const n = Math.hypot(rx, ry, rz, rw) || 1;
        this.b[o] = rx / n;
        this.b[o + 1] = ry / n;
        this.b[o + 2] = rz / n;
        this.b[o + 3] = rw / n;
    }
}

globalThis.Transform = Transform;

// --- generell väg: godtyckliga komponenter som JSON ---------------------
// Skriptet ser komponenterna precis som de ser ut i Rust, och samlar
// strukturella ändringar i en kommandobuffert istället för att röra
// världen direkt – samma modell som bevy Commands.
globalThis.__commands = [];

globalThis.engine = {
    /// Roterar en kvaternion [x,y,z,w] på plats kring en axel.
    rotate(q, ax, ay, az, radians) {
        const len = Math.hypot(ax, ay, az);
        if (len === 0) return q;
        const s = Math.sin(radians * 0.5) / len;
        const dx = ax * s, dy = ay * s, dz = az * s, dw = Math.cos(radians * 0.5);
        const rx = dw * q[0] + dx * q[3] + dy * q[2] - dz * q[1];
        const ry = dw * q[1] - dx * q[2] + dy * q[3] + dz * q[0];
        const rz = dw * q[2] + dx * q[1] - dy * q[0] + dz * q[3];
        const rw = dw * q[3] - dx * q[0] - dy * q[1] - dz * q[2];
        const n = Math.hypot(rx, ry, rz, rw) || 1;
        q[0] = rx / n; q[1] = ry / n; q[2] = rz / n; q[3] = rw / n;
        return q;
    },

    spawn(components) {
        globalThis.__commands.push({ op: "spawn", components: components });
    },

    /// Tar bort en entitet. Fungerar både på skriptets egna (har `i`) och
    /// på träffar från scenfrågorna (har `w`).
    despawn(entity) {
        if (entity.w !== undefined) {
            globalThis.__commands.push({ op: "despawn", w: entity.w });
        } else {
            globalThis.__commands.push({ op: "despawn", i: entity.i });
        }
    },

    /// Skriver komponenter på vilken entitet som helst i scenen.
    set(entry, components) {
        globalThis.__commands.push({ op: "set", w: entry.w, components: components });
    },

    // --- scenfrågor ----------------------------------------------------
    // Arbetar mot en ögonblicksbild av scenen som värden skickar varje
    // frame. Billigt nog för hobbyscener; en riktig motor flyttar det här
    // till värdfunktioner med ett spatialt index.

    find(name) {
        const world = globalThis.__world;
        for (let i = 0; i < world.length; i++) {
            if (world[i].name === name) return world[i];
        }
        return null;
    },

    findAll(fragment) {
        return globalThis.__world.filter((e) => e.name && e.name.indexOf(fragment) !== -1);
    },

    distance(a, b) {
        const pa = a.t || (a.Transform && a.Transform.translation);
        const pb = b.t || (b.Transform && b.Transform.translation);
        return Math.hypot(pa[0] - pb[0], pa[1] - pb[1], pa[2] - pb[2]);
    },

    /// Axelinriktad överlappstest. Skalan tolkas som kubens storlek.
    overlaps(a, b) {
        const pa = a.t || (a.Transform && a.Transform.translation);
        const sa = a.s || (a.Transform && a.Transform.scale);
        const pb = b.t || (b.Transform && b.Transform.translation);
        const sb = b.s || (b.Transform && b.Transform.scale);
        for (let axis = 0; axis < 3; axis++) {
            const overlap = Math.abs(pa[axis] - pb[axis]) * 2 - (sa[axis] + sb[axis]);
            if (overlap > 0) return false;
        }
        return true;
    },

    /// Raycast mot alla AABB:er i scenen. Slab-metoden, närmaste träff vinner.
    raycast(origin, direction, maxDistance, skipName) {
        const len = Math.hypot(direction[0], direction[1], direction[2]) || 1;
        const d = [direction[0] / len, direction[1] / len, direction[2] / len];
        const limit = maxDistance === undefined ? Infinity : maxDistance;

        let best = null;
        const world = globalThis.__world;

        for (let i = 0; i < world.length; i++) {
            const entry = world[i];
            if (skipName !== undefined && entry.name === skipName) continue;

            let near = 0;
            let far = limit;
            let miss = false;

            for (let axis = 0; axis < 3; axis++) {
                const half = entry.s[axis] * 0.5;
                const min = entry.t[axis] - half;
                const max = entry.t[axis] + half;

                if (Math.abs(d[axis]) < 1e-8) {
                    if (origin[axis] < min || origin[axis] > max) { miss = true; break; }
                    continue;
                }
                let t0 = (min - origin[axis]) / d[axis];
                let t1 = (max - origin[axis]) / d[axis];
                if (t0 > t1) { const tmp = t0; t0 = t1; t1 = tmp; }
                if (t0 > near) near = t0;
                if (t1 < far) far = t1;
                if (near > far) { miss = true; break; }
            }

            if (!miss && near <= limit && (best === null || near < best.distance)) {
                best = { entry: entry, distance: near };
            }
        }
        return best;
    },
};

globalThis.__engine_update_json = function (payload) {
    const frame = JSON.parse(payload);

    // Input exponeras som metoder istället för råa mängder.
    const raw = frame.input || { down: [], pressed: [], released: [], mouse_down: [] };
    globalThis.engine.input = {
        isDown: (key) => raw.down.indexOf(key) !== -1,
        justPressed: (key) => raw.pressed.indexOf(key) !== -1,
        justReleased: (key) => raw.released.indexOf(key) !== -1,
        mouseIsDown: (button) => raw.mouse_down.indexOf(button) !== -1,
        mouse: {
            x: raw.mouse_position ? raw.mouse_position[0] : 0,
            y: raw.mouse_position ? raw.mouse_position[1] : 0,
            dx: raw.mouse_delta ? raw.mouse_delta[0] : 0,
            dy: raw.mouse_delta ? raw.mouse_delta[1] : 0,
        },
        /// -1, 0 eller 1 längs en axel från två tangenter.
        axis: (negative, positive) => {
            const n = raw.down.indexOf(negative) !== -1 ? 1 : 0;
            const p = raw.down.indexOf(positive) !== -1 ? 1 : 0;
            return p - n;
        },
    };

    globalThis.__world = frame.world || [];

    const input = frame;
    const fn = globalThis.update;
    if (typeof fn !== "function") {
        throw new Error("skriptet saknar en global update(dt, entities)");
    }

    globalThis.__commands = [];
    fn(input.dt, input.entities);

    return JSON.stringify({
        entities: input.entities,
        commands: globalThis.__commands,
    });
};

// Anropas av värden. Bygger vyer över bufferten och lämnar dem till
// användarens update(dt, entities).
globalThis.__engine_update = function (dt, count, buffer) {
    const fn = globalThis.update;
    if (typeof fn !== "function") {
        throw new Error("skriptet saknar en global update(dt, entities)");
    }
    const entities = new Array(count);
    for (let i = 0; i < count; i++) {
        entities[i] = new Transform(buffer, i);
    }
    fn(dt, entities);
};
"#;

thread_local! {
    static RESULT: RefCell<String> = const { RefCell::new(String::new()) };
}

fn set_error(message: impl Into<String>) -> i32 {
    LAST_ERROR.with(|slot| *slot.borrow_mut() = message.into());
    -1
}

/// Reserverar en buffert som värden skriver komponentdata i.
///
/// # Safety
/// Minnet läcks med flit – värden äger det tills modulen kastas.
#[unsafe(no_mangle)]
pub extern "C" fn alloc(len: u32) -> u32 {
    let mut buffer = vec![0u8; len as usize];
    let ptr = buffer.as_mut_ptr() as u32;
    std::mem::forget(buffer);
    ptr
}

/// Hämtar allt som loggats sedan senaste anropet, som JSON.
/// Anropas av värden efter varje `update`/`update_json`.
#[unsafe(no_mangle)]
pub extern "C" fn take_logs() -> i32 {
    VM.with(|slot| {
        let borrow = slot.borrow();
        let Some((_, context)) = borrow.as_ref() else {
            return set_error("inget script laddat");
        };

        let result = context.with(|ctx| -> rquickjs::Result<String> {
            let globals = ctx.globals();
            let entry: Function = globals.get("__engine_take_logs")?;
            entry.call::<_, String>(())
        });

        match result {
            Ok(json) => {
                RESULT.with(|slot| *slot.borrow_mut() = json);
                0
            }
            Err(err) => set_error(format!("{err}")),
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn error_ptr() -> u32 {
    LAST_ERROR.with(|slot| slot.borrow().as_ptr() as u32)
}

#[unsafe(no_mangle)]
pub extern "C" fn error_len() -> u32 {
    LAST_ERROR.with(|slot| slot.borrow().len() as u32)
}

/// Evaluerar prelude + användarens script i en färsk QuickJS-runtime.
/// Att alltid skapa om runtimen är hela hot reload-strategin: allt state
/// ligger i komponenter, inget i skriptets globaler.
#[unsafe(no_mangle)]
pub extern "C" fn load_script(ptr: u32, len: u32) -> i32 {
    let source = unsafe {
        let slice = std::slice::from_raw_parts(ptr as *const u8, len as usize);
        match std::str::from_utf8(slice) {
            Ok(text) => text.to_string(),
            Err(err) => return set_error(format!("ogiltig UTF-8: {err}")),
        }
    };

    let runtime = match Runtime::new() {
        Ok(runtime) => runtime,
        Err(err) => return set_error(format!("kunde inte skapa runtime: {err}")),
    };
    let context = match Context::full(&runtime) {
        Ok(context) => context,
        Err(err) => return set_error(format!("kunde inte skapa context: {err}")),
    };

    let result = context.with(|ctx| -> rquickjs::Result<()> {
        ctx.eval::<(), _>(PRELUDE)?;
        ctx.eval::<(), _>(source)?;
        Ok(())
    });

    if let Err(err) = result {
        return set_error(format!("{err}"));
    }

    VM.with(|slot| *slot.borrow_mut() = Some((runtime, context)));
    0
}

/// Kör ett systemanrop: en `update` över alla matchande entiteter.
#[unsafe(no_mangle)]
pub extern "C" fn update(dt: f32, count: u32, ptr: u32) -> i32 {
    let floats = unsafe {
        std::slice::from_raw_parts_mut(ptr as *mut f32, count as usize * STRIDE)
    };

    VM.with(|slot| {
        let borrow = slot.borrow();
        let Some((_, context)) = borrow.as_ref() else {
            return set_error("inget script laddat");
        };

        let result = context.with(|ctx| -> rquickjs::Result<()> {
            // QuickJS har egen heap, så data kopieras in och ut. Några tiotal
            // kB per frame – billigare än ett värd-anrop per komponent.
            let array = TypedArray::<f32>::new(ctx.clone(), &*floats)?;
            let globals = ctx.globals();
            let entry: Function = globals.get("__engine_update")?;
            entry.call::<_, ()>((dt, count, array.clone()))?;

            // as_raw pekar rakt in i QuickJS-heapen – kopiera tillbaka därifrån.
            if let Some(raw) = array.as_raw() {
                let bytes = unsafe { raw.as_ref() };
                let values = unsafe {
                    std::slice::from_raw_parts(bytes.as_ptr() as *const f32, bytes.len() / 4)
                };
                if let Some(slice) = values.get(..floats.len()) {
                    floats.copy_from_slice(slice);
                }
            }
            Ok(())
        });

        match result {
            Ok(()) => 0,
            Err(err) => set_error(format!("{err}")),
        }
    })
}

/// Generella vägen: hela frame:ns komponentdata som JSON in, ändrad data
/// plus kommandobuffert ut. Långsammare än `update`, men klarar alla
/// komponenter som typregistret känner till.
#[unsafe(no_mangle)]
pub extern "C" fn update_json(ptr: u32, len: u32) -> i32 {
    let payload = unsafe {
        let slice = std::slice::from_raw_parts(ptr as *const u8, len as usize);
        match std::str::from_utf8(slice) {
            Ok(text) => text.to_string(),
            Err(err) => return set_error(format!("ogiltig UTF-8: {err}")),
        }
    };

    VM.with(|slot| {
        let borrow = slot.borrow();
        let Some((_, context)) = borrow.as_ref() else {
            return set_error("inget script laddat");
        };

        let result = context.with(|ctx| -> rquickjs::Result<String> {
            let globals = ctx.globals();
            let entry: Function = globals.get("__engine_update_json")?;
            entry.call::<_, String>((payload,))
        });

        match result {
            Ok(json) => {
                RESULT.with(|slot| *slot.borrow_mut() = json);
                0
            }
            Err(err) => set_error(format!("{err}")),
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn result_ptr() -> u32 {
    RESULT.with(|slot| slot.borrow().as_ptr() as u32)
}

#[unsafe(no_mangle)]
pub extern "C" fn result_len() -> u32 {
    RESULT.with(|slot| slot.borrow().len() as u32)
}
