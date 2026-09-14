//! Grundtyper som resten av motorn delar: tid, matte, transformer, kamera.
//! Beror på bevy_ecs för att kunna derive:a Component/Resource, men vet
//! ingenting om GPU eller fönster.

pub mod physics;
pub use physics::{Collider, Gravity, RigidBody, step_physics};

pub use bevy_ecs;
pub use glam;
pub use glam::{Mat4, Quat, Vec2, Vec3, Vec4};

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::time::{Duration, Instant};

// ---------------------------------------------------------------- tid

/// Bildruteklocka. `tick()` anropas en gång per frame av runtime.
#[derive(Resource, Debug)]
pub struct Time {
    start: Instant,
    last: Instant,
    delta: Duration,
    elapsed: Duration,
    frame: u64,
}

impl Time {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            start: now,
            last: now,
            delta: Duration::ZERO,
            elapsed: Duration::ZERO,
            frame: 0,
        }
    }

    /// Uppdaterar mot väggklockan.
    pub fn tick(&mut self) {
        let now = Instant::now();
        self.delta = now - self.last;
        self.last = now;
        self.elapsed = now - self.start;
        self.frame += 1;
    }

    /// Stegar klockan manuellt – för headless-rendering och tester där vi vill
    /// ha exakt samma bild varje körning.
    pub fn advance_by(&mut self, delta: Duration) {
        self.delta = delta;
        self.elapsed += delta;
        self.frame += 1;
    }

    pub fn delta_seconds(&self) -> f32 {
        self.delta.as_secs_f32()
    }

    pub fn elapsed_seconds(&self) -> f32 {
        self.elapsed.as_secs_f32()
    }

    pub fn frame(&self) -> u64 {
        self.frame
    }
}

impl Default for Time {
    fn default() -> Self {
        Self::new()
    }
}

// -------------------------------------------------------------- input

/// Tangent- och musläge för den här framen.
///
/// Tangenter namnges som i webbens `KeyboardEvent.code` – "KeyW", "Space",
/// "ArrowLeft" – vilket råkar vara exakt winits `KeyCode`-namn. Det gör att
/// samma sträng fungerar i Rust, i skript och i en eventuell webbversion.
#[derive(Resource, Debug, Clone, Default, Serialize)]
pub struct Input {
    down: BTreeSet<String>,
    pressed: BTreeSet<String>,
    released: BTreeSet<String>,
    pub mouse_position: Vec2,
    pub mouse_delta: Vec2,
    mouse_down: BTreeSet<String>,
}

impl Input {
    /// Hålls tangenten nere just nu?
    pub fn is_down(&self, key: &str) -> bool {
        self.down.contains(key)
    }

    /// Trycktes den ned den här framen?
    pub fn just_pressed(&self, key: &str) -> bool {
        self.pressed.contains(key)
    }

    pub fn just_released(&self, key: &str) -> bool {
        self.released.contains(key)
    }

    pub fn mouse_is_down(&self, button: &str) -> bool {
        self.mouse_down.contains(button)
    }

    pub fn press(&mut self, key: impl Into<String>) {
        let key = key.into();
        // Autorepeat ska inte ge nya "just pressed".
        if self.down.insert(key.clone()) {
            self.pressed.insert(key);
        }
    }

    pub fn release(&mut self, key: impl Into<String>) {
        let key = key.into();
        if self.down.remove(&key) {
            self.released.insert(key);
        }
    }

    pub fn press_mouse(&mut self, button: impl Into<String>) {
        self.mouse_down.insert(button.into());
    }

    pub fn release_mouse(&mut self, button: &str) {
        self.mouse_down.remove(button);
    }

    pub fn set_mouse_position(&mut self, position: Vec2) {
        self.mouse_delta = position - self.mouse_position;
        self.mouse_position = position;
    }

    /// Körs sist varje frame: engångshändelserna gäller bara en frame.
    pub fn end_frame(&mut self) {
        self.pressed.clear();
        self.released.clear();
        self.mouse_delta = Vec2::ZERO;
    }
}

// --------------------------------------------------------- assetkällor

/// Var motorn läser innehåll ifrån.
///
/// Editorn läser lösa filer från projektmappen; ett exporterat spel läser
/// ur ett `.pak`-arkiv. Allt däremellan – scener, texturer, modeller,
/// skript – går genom det här gränssnittet, så att det bara finns *en*
/// kodväg att testa. Två vägar hade gett buggar som bara uppstår i
/// exporterade spel, vilket är den värsta sorten.
pub trait AssetSource: Send + Sync {
    /// Läser en fil. Sökvägen är alltid relativ till projektroten och
    /// använder `/` som separator, även på Windows.
    fn read(&self, path: &str) -> std::io::Result<Vec<u8>>;

    /// Alla sökvägar som börjar med `prefix` och slutar med `extension`.
    /// `extension` anges utan punkt; tom sträng matchar allt.
    fn list(&self, prefix: &str, extension: &str) -> Vec<String>;

    fn exists(&self, path: &str) -> bool {
        self.read(path).is_ok()
    }

    /// Bekvämlighet: läs som UTF-8.
    fn read_to_string(&self, path: &str) -> std::io::Result<String> {
        let bytes = self.read(path)?;
        String::from_utf8(bytes)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))
    }
}

// -------------------------------------------------------------- konsol

/// Allvarlighetsgrad för en konsolrad. Ordningen är stigande – `as u8`
/// duger för filtrering i UI:t.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Log,
    Info,
    Warn,
    Error,
}

/// En rad i debug-konsolen. `source` är skriptfilen som loggade, eller
/// `"console"` för rader skrivna direkt i konsolen.
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub level: LogLevel,
    pub message: String,
    pub source: String,
}

/// Loggbuffert delad av alla körande skript. Fylls av `console.log` och
/// släktingar i wasm-lagret, läses av debug-konsolen – i editorn och i
/// spelet självt.
#[derive(Resource, Default)]
pub struct ConsoleLog {
    entries: std::collections::VecDeque<LogEntry>,
}

/// Tak på hur många rader som sparas – en konsol som loggar varje frame
/// ska inte äta minne obegränsat.
const CONSOLE_LOG_CAPACITY: usize = 500;

impl ConsoleLog {
    pub fn push(&mut self, level: LogLevel, message: impl Into<String>, source: impl Into<String>) {
        if self.entries.len() >= CONSOLE_LOG_CAPACITY {
            self.entries.pop_front();
        }
        self.entries.push_back(LogEntry {
            level,
            message: message.into(),
            source: source.into(),
        });
    }

    pub fn entries(&self) -> impl Iterator<Item = &LogEntry> {
        self.entries.iter()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

// -------------------------------------------------------------- färger

/// Linjär RGBA.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const WHITE: Self = Self::rgb(1.0, 1.0, 1.0);

    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub fn to_array(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

// ---------------------------------------------------------- transformer

/// Lokal transform relativt förälder (eller världen om entiteten är rot).
#[derive(Component, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Transform {
    pub const IDENTITY: Self = Self {
        translation: Vec3::ZERO,
        rotation: Quat::IDENTITY,
        scale: Vec3::ONE,
    };

    pub fn from_xyz(x: f32, y: f32, z: f32) -> Self {
        Self {
            translation: Vec3::new(x, y, z),
            ..Self::IDENTITY
        }
    }

    pub fn with_scale(mut self, scale: Vec3) -> Self {
        self.scale = scale;
        self
    }

    pub fn with_rotation(mut self, rotation: Quat) -> Self {
        self.rotation = rotation;
        self
    }

    /// Kamerahjälp: rikta transformen mot en punkt.
    pub fn looking_at(mut self, target: Vec3, up: Vec3) -> Self {
        let matrix = glam::camera::rh::view::look_at_mat4(self.translation, target, up).inverse();
        let (_, rotation, _) = matrix.to_scale_rotation_translation();
        self.rotation = rotation;
        self
    }

    pub fn matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

/// Transform i världsrymden, uträknad av `propagate_transforms` varje frame.
/// Skriv aldrig till den direkt – ändra `Transform` istället.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct GlobalTransform(pub Mat4);

impl GlobalTransform {
    pub fn translation(&self) -> Vec3 {
        self.0.w_axis.truncate()
    }
}

// ------------------------------------------------------------- rendering

/// Runtime-handtag till en uppladdad mesh. Aldrig i scenfiler – där står
/// namnet, som assetregistret slår upp vid laddning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct MeshId(pub u32);

/// Runtime-handtag till en uppladdad textur.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct TextureId(pub u32);

/// Ritbar entitet. `mesh` och `texture` är assetnamn: inbyggda heter
/// `builtin/cube` och `builtin/plane`, projektets egna namnges efter sin
/// sökväg, t.ex. `textures/tegel.png`.
#[derive(Component, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeshInstance {
    pub mesh: String,
    // Inget skip_serializing_if: fältet måste synas i default-instansen,
    // annars saknas det i de genererade TypeScript-typerna.
    #[serde(default)]
    pub texture: String,
    pub color: Color,
}

pub const BUILTIN_CUBE: &str = "builtin/cube";
pub const BUILTIN_PLANE: &str = "builtin/plane";

impl MeshInstance {
    pub fn new(mesh: impl Into<String>, color: Color) -> Self {
        Self {
            mesh: mesh.into(),
            texture: String::new(),
            color,
        }
    }

    pub fn with_texture(mut self, texture: impl Into<String>) -> Self {
        self.texture = texture.into();
        self
    }
}

impl Default for MeshInstance {
    fn default() -> Self {
        Self {
            mesh: BUILTIN_CUBE.to_string(),
            texture: String::new(),
            color: Color::WHITE,
        }
    }
}

/// Kopplar ett skript till en entitet. Strängen är sökvägen till .ts-filen
/// relativt assets/scripts. Skript körs som system: alla entiteter med samma
/// skript uppdateras i ett enda anrop.
#[derive(Component, Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Script(pub String);

impl Script {
    pub fn new(path: impl Into<String>) -> Self {
        Self(path.into())
    }
}

/// Läsbart namn – används av editorns hierarkivy och av scenfiler.
#[derive(Component, Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntityName(pub String);

impl EntityName {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Perspektivkamera. Vyn tas från entitetens `GlobalTransform`.
#[derive(Component, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Camera {
    pub fov_y_radians: f32,
    pub z_near: f32,
    pub z_far: f32,
    /// Ortografisk projektion istället för perspektiv – 2D-läget.
    ///
    /// Medvetet en bool plus ett tal, inte en enum: den typade inspectorn
    /// härleder sina widgets ur JSON-formen, och en enum blir ett nästlat
    /// objekt med variantnyckel som inte går att byta i UI:t. Så här får
    /// man en kryssruta och ett dragfält.
    #[serde(default)]
    pub orthographic: bool,
    /// Synfältets höjd i världsenheter när `orthographic` är satt.
    /// Bredden följer av fönstrets bildkvot.
    #[serde(default = "default_ortho_height")]
    pub ortho_height: f32,
}

fn default_ortho_height() -> f32 {
    10.0
}

impl Camera {
    /// Skapar en 2D-kamera. `height` är hur många världsenheter som ryms
    /// vertikalt i bild.
    pub fn orthographic_2d(height: f32) -> Self {
        Self {
            orthographic: true,
            ortho_height: height,
            ..Self::default()
        }
    }

    pub fn projection(&self, aspect: f32) -> Mat4 {
        let aspect = aspect.max(0.0001);

        // wgpu/WebGPU använder DirectX-konventionen: Y uppåt och djup 0..1.
        // (glams "vulkan"-variant vänder Y – det sköter wgpu redan åt oss.)
        if self.orthographic {
            let half_height = self.ortho_height.max(0.0001) * 0.5;
            let half_width = half_height * aspect;
            return glam::camera::rh::proj::directx::orthographic(
                -half_width,
                half_width,
                -half_height,
                half_height,
                self.z_near,
                self.z_far,
            );
        }

        glam::camera::rh::proj::directx::perspective(
            self.fov_y_radians,
            aspect,
            self.z_near,
            self.z_far,
        )
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            fov_y_radians: 60f32.to_radians(),
            z_near: 0.1,
            z_far: 1000.0,
            orthographic: false,
            ortho_height: default_ortho_height(),
        }
    }
}

// --------------------------------------------------------------- sprites

pub const BUILTIN_QUAD: &str = "builtin/quad";

/// En 2D-sprite. Ritas som en kvadrat i XY-planet, genomskinlighet
/// respekterad, och alltid efter all ogenomskinlig geometri.
///
/// Spritesheets hanteras med `columns`/`rows`/`index` istället för
/// pixelrektanglar: rutnätsindelning täcker nästan alla spritesheets och
/// kräver inte att motorn känner till texturens pixelstorlek.
#[derive(Component, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sprite {
    /// Texturnamn. Tom sträng ger en vit ruta i `color`.
    #[serde(default)]
    pub texture: String,
    /// Storlek i världsenheter.
    pub size: Vec2,
    pub color: Color,
    /// Antal rutor i spritesheetet. 1x1 = hela texturen.
    #[serde(default = "one")]
    pub columns: u32,
    #[serde(default = "one")]
    pub rows: u32,
    /// Vilken ruta som visas, radvis uppifrån och ner.
    #[serde(default)]
    pub index: u32,
    #[serde(default)]
    pub flip_x: bool,
    #[serde(default)]
    pub flip_y: bool,
}

fn one() -> u32 {
    1
}

impl Sprite {
    pub fn new(texture: impl Into<String>, width: f32, height: f32) -> Self {
        Self {
            texture: texture.into(),
            size: Vec2::new(width, height),
            color: Color::WHITE,
            columns: 1,
            rows: 1,
            index: 0,
            flip_x: false,
            flip_y: false,
        }
    }

    /// Delar upp texturen i ett rutnät.
    pub fn with_grid(mut self, columns: u32, rows: u32) -> Self {
        self.columns = columns.max(1);
        self.rows = rows.max(1);
        self
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// UV-transform som `[offset_x, offset_y, scale_x, scale_y]`.
    /// Vändning görs genom att spegla skalan och flytta offseten till
    /// andra kanten – billigare än en separat shader-variant.
    pub fn uv_transform(&self) -> [f32; 4] {
        let columns = self.columns.max(1);
        let rows = self.rows.max(1);
        let total = columns * rows;
        let index = if total == 0 { 0 } else { self.index % total };

        let scale_x = 1.0 / columns as f32;
        let scale_y = 1.0 / rows as f32;
        let mut offset_x = (index % columns) as f32 * scale_x;
        let mut offset_y = (index / columns) as f32 * scale_y;

        let mut scale_x = scale_x;
        let mut scale_y = scale_y;
        if self.flip_x {
            offset_x += scale_x;
            scale_x = -scale_x;
        }
        if self.flip_y {
            offset_y += scale_y;
            scale_y = -scale_y;
        }

        [offset_x, offset_y, scale_x, scale_y]
    }
}

impl Default for Sprite {
    fn default() -> Self {
        Self::new(String::new(), 1.0, 1.0)
    }
}

/// Spelar upp rutorna i ett spritesheet. `frames` = 0 betyder alla rutor.
#[derive(Component, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpriteAnimation {
    pub fps: f32,
    #[serde(default)]
    pub frames: u32,
    #[serde(default = "yes")]
    pub playing: bool,
    /// Ackumulerad tid sedan senaste rutbyte. Sparas inte i scenfiler –
    /// det är körtidstillstånd, inte innehåll.
    #[serde(skip)]
    pub timer: f32,
}

fn yes() -> bool {
    true
}

impl Default for SpriteAnimation {
    fn default() -> Self {
        Self {
            fps: 8.0,
            frames: 0,
            playing: true,
            timer: 0.0,
        }
    }
}

/// Stegar `Sprite::index` framåt enligt `SpriteAnimation`.
pub fn animate_sprites(time: Res<Time>, mut query: Query<(&mut Sprite, &mut SpriteAnimation)>) {
    let dt = time.delta_seconds();

    for (mut sprite, mut animation) in &mut query {
        if !animation.playing || animation.fps <= 0.0 {
            continue;
        }

        animation.timer += dt;
        let step = 1.0 / animation.fps;
        if animation.timer < step {
            continue;
        }

        // while-loop, inte if: vid låg fps eller en lång frame ska
        // animationen hoppa rätt antal rutor, inte halka efter.
        let total = if animation.frames == 0 {
            (sprite.columns.max(1) * sprite.rows.max(1)).max(1)
        } else {
            animation.frames
        };

        while animation.timer >= step {
            animation.timer -= step;
            sprite.index = (sprite.index + 1) % total;
        }
    }
}

// ------------------------------------------------------------ hierarki

/// Räknar ut `GlobalTransform` för alla entiteter genom att gå ned i
/// hierarkin från rötterna. Exklusivt system: enklare och helt korrekt,
/// och snabbt nog tills scenerna blir stora.
pub fn propagate_transforms(world: &mut World) {
    let roots: Vec<Entity> = world
        .query_filtered::<Entity, (With<Transform>, Without<ChildOf>)>()
        .iter(world)
        .collect();

    let mut stack: Vec<(Entity, Mat4)> = roots.into_iter().map(|e| (e, Mat4::IDENTITY)).collect();

    while let Some((entity, parent_matrix)) = stack.pop() {
        let Some(local) = world.get::<Transform>(entity).copied() else {
            continue;
        };
        let global = parent_matrix * local.matrix();

        if let Some(mut slot) = world.get_mut::<GlobalTransform>(entity) {
            slot.0 = global;
        } else {
            world.entity_mut(entity).insert(GlobalTransform(global));
        }

        if let Some(children) = world.get::<Children>(entity) {
            let kids: Vec<Entity> = children.iter().collect();
            stack.extend(kids.into_iter().map(|child| (child, global)));
        }
    }
}
