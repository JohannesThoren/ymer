//! Ett litet stadsbyggarspel, byggt på motorns egna delar.
//!
//! Arbetsfördelningen är medveten:
//!
//! * **Scenen** (`scenes/main.ron`) är kartan. Den laddas som vilken scen
//!   som helst och går att öppna i editorn.
//! * **Skriptet** (`scripts/ekonomi.ts`) äger reglerna: inflyttning,
//!   bemanning, inkomst. Att ändra vad en butik drar in är en filändring,
//!   inte en omkompilering – och hot reload gäller.
//! * **Rust** gör det skript inte når: plocket från mus till ruta, som
//!   behöver kameramatrisen, och gränssnittet, som är egui.
//!
//! `Byggnad` och `Stadskassa` är riktiga komponenter i typregistret. Det
//! är det som gör resten möjligt: de serialiseras till scenfilen, syns i
//! editorns inspector, och skript ser dem.
//!
//! Grafiken är Kenneys "City Kit (Commercial)" (CC0), se
//! `models/KENNEY-LICENSE.txt`.

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};
use ymer_core::{
    Color, EntityName, GlobalTransform, Mat4, MeshInstance, Script, Transform, Vec2, Vec3,
};
use ymer_scene::TypeRegistry;

pub mod ui;

/// Rutans storlek i världsenheter. Modellerna är gjorda för exakt det.
pub const CELL: f32 = 1.0;
/// Halva bredden på kartan i rutor: -HALF..=HALF i båda riktningar.
pub const HALF: i32 = 5;
/// Skriptet som driver ekonomin. Ligger på varje byggnad *och* på
/// stadshuset, eftersom skript kör som system: ett anrop per frame får
/// hela listan, så en enda `update` ser både byggnaderna och kassan.
pub const SCRIPT: &str = "ekonomi.ts";

pub type Tile = (i32, i32);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Kind {
    #[default]
    Hus,
    Butik,
    Marknad,
    Kontor,
}

impl Kind {
    pub const ALL: [Kind; 4] = [Kind::Hus, Kind::Butik, Kind::Marknad, Kind::Kontor];

    pub fn namn(self) -> &'static str {
        match self {
            Kind::Hus => "Hus",
            Kind::Butik => "Butik",
            Kind::Marknad => "Marknad",
            Kind::Kontor => "Kontor",
        }
    }

    /// Kostnaden är Rusts, eftersom UI:t visar den innan något byggts och
    /// skriptet aldrig ser ett bygge som inte blev av. Vad byggnaden
    /// sedan *gör* bestämmer skriptet.
    pub fn kostnad(self) -> f32 {
        match self {
            Kind::Hus => 50.0,
            Kind::Butik => 80.0,
            Kind::Marknad => 140.0,
            Kind::Kontor => 300.0,
        }
    }

    pub fn beskrivning(self) -> &'static str {
        match self {
            Kind::Hus => "Ger boendeplatser. Invånare flyttar in av sig själva.",
            Kind::Butik => "Liten arbetsplats. Ger guld när den är bemannad.",
            Kind::Marknad => "Större arbetsplats, mer guld och fler jobb.",
            Kind::Kontor => "Stor arbetsplats. Kräver många invånare.",
        }
    }

    fn asset(self) -> (&'static str, &'static str) {
        // (mesh, textur). Namnen är verifierade med gltf_probe.
        match self {
            Kind::Hus => (
                "models/building-a.glb#building-a",
                "models/building-a.glb#tex0",
            ),
            Kind::Butik => (
                "models/building-c.glb#building-c",
                "models/building-c.glb#tex0",
            ),
            Kind::Marknad => (
                "models/building-b.glb#building-b",
                "models/building-b.glb#tex0",
            ),
            Kind::Kontor => (
                "models/building-skyscraper-a.glb#building-skyscraper-a",
                "models/building-skyscraper-a.glb#tex0",
            ),
        }
    }
}

/// En byggnad på kartan.
///
/// `platser`, `jobb` och `inkomst` sätts av skriptet vid första ticket –
/// Rust vet bara *vilken sorts* hus det är, inte vad det gör. Det gör att
/// balansen kan ändras i en textfil medan spelet kör.
#[derive(Component, Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Byggnad {
    pub kind: Kind,
    pub tile_x: i32,
    pub tile_z: i32,
    /// Boendeplatser byggnaden bidrar med.
    pub platser: u32,
    /// Arbetstillfällen den erbjuder.
    pub jobb: u32,
    /// Guld per sekund vid full bemanning.
    pub inkomst: f32,
    /// Hur många som faktiskt arbetar här just nu. Skriptets bokföring.
    pub bemanning: u32,
}

impl Byggnad {
    pub fn tile(&self) -> Tile {
        (self.tile_x, self.tile_z)
    }
}

/// Stadens gemensamma räkenskaper, på en egen entitet ("Stadshuset").
///
/// Skript kan inte läsa andra entiteters komponenter – men de kör som
/// system, så en enda `update` får hela listan över entiteter som bär
/// skriptet. Ligger kassan på en av dem ser skriptet både byggnaderna
/// och kassan i samma anrop, och kan bokföra mellan dem.
#[derive(Component, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Stadskassa {
    pub guld: f32,
    pub invanare: f32,
    /// Summerat av skriptet varje frame, läst av HUD:en.
    pub platser: u32,
    pub jobb: u32,
    pub sysselsatta: u32,
    pub inkomst: f32,
}

impl Default for Stadskassa {
    fn default() -> Self {
        Self {
            guld: 200.0,
            invanare: 0.0,
            platser: 0,
            jobb: 0,
            sysselsatta: 0,
            inkomst: 0.0,
        }
    }
}

/// Marken. Egen komponent så att plocket kan skilja mark från byggnad.
#[derive(Component, Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Mark {
    pub tile_x: i32,
    pub tile_z: i32,
}

/// Registrerar spelets komponenter utöver motorns inbyggda.
///
/// Utan det här blir `Byggnad` osynlig för både scenfilen och skriptet –
/// typregistret är den enda vägen in för något som inte är motorns eget.
pub fn register_types(registry: &mut TypeRegistry) {
    registry.register::<Byggnad>("Byggnad");
    registry.register::<Stadskassa>("Stadskassa");
    registry.register::<Mark>("Mark");
}

/// Vad spelaren håller på med. Rent UI-tillstånd, inget skriptet rör.
#[derive(Resource, Debug, Clone)]
pub struct Val {
    pub vald: Kind,
    pub markerad: Option<Tile>,
    pub status: String,
}

impl Default for Val {
    fn default() -> Self {
        Self {
            vald: Kind::Hus,
            markerad: None,
            status: "Bygg ett hus för att få invånare.".to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Neka {
    UtanforKartan,
    Upptaget,
    ForDyrt,
}

impl Neka {
    pub fn text(self) -> &'static str {
        match self {
            Neka::UtanforKartan => "Utanför kartan.",
            Neka::Upptaget => "Rutan är upptagen.",
            Neka::ForDyrt => "Har inte råd.",
        }
    }
}

pub fn inom_kartan(tile: Tile) -> bool {
    tile.0 >= -HALF && tile.0 <= HALF && tile.1 >= -HALF && tile.1 <= HALF
}

/// Byggnaden på en ruta, om någon.
pub fn byggnad_pa(world: &mut World, tile: Tile) -> Option<(Entity, Byggnad)> {
    let mut query = world.query::<(Entity, &Byggnad)>();
    query
        .iter(world)
        .find(|(_, b)| b.tile() == tile)
        .map(|(e, b)| (e, *b))
}

/// Kassans entitet. Stadshuset finns i scenen, så den ska alltid gå att
/// hitta – men ett spel som laddat en trasig scen ska inte krascha.
pub fn kassa_entity(world: &mut World) -> Option<Entity> {
    let mut query = world.query_filtered::<Entity, With<Stadskassa>>();
    query.iter(world).next()
}

pub fn kassa(world: &mut World) -> Stadskassa {
    kassa_entity(world)
        .and_then(|e| world.get::<Stadskassa>(e).copied())
        .unwrap_or_default()
}

/// Bygger på en ruta. Drar guldet och sätter komponenten; vad byggnaden
/// *gör* fyller skriptet i vid nästa tick.
pub fn bygg(world: &mut World, tile: Tile, kind: Kind) -> Result<Entity, Neka> {
    if !inom_kartan(tile) {
        return Err(Neka::UtanforKartan);
    }
    if byggnad_pa(world, tile).is_some() {
        return Err(Neka::Upptaget);
    }

    let Some(kassa_entity) = kassa_entity(world) else {
        return Err(Neka::ForDyrt);
    };
    let guld = world
        .get::<Stadskassa>(kassa_entity)
        .map_or(0.0, |k| k.guld);
    if guld < kind.kostnad() {
        return Err(Neka::ForDyrt);
    }

    let (mesh, texture) = kind.asset();
    let entity = world
        .spawn((
            EntityName::new(format!("{} {},{}", kind.namn(), tile.0, tile.1)),
            Transform::from_xyz(tile.0 as f32 * CELL, 0.0, tile.1 as f32 * CELL),
            GlobalTransform::default(),
            MeshInstance {
                mesh: mesh.to_string(),
                texture: texture.to_string(),
                color: Color::WHITE,
            },
            Byggnad {
                kind,
                tile_x: tile.0,
                tile_z: tile.1,
                ..Default::default()
            },
            // Skriptet kör som system: byggnaden måste bära det för att
            // komma med i listan skriptet får.
            Script::new(SCRIPT),
        ))
        .id();

    if let Some(mut k) = world.get_mut::<Stadskassa>(kassa_entity) {
        k.guld -= kind.kostnad();
    }
    Ok(entity)
}

/// River byggnaden på en ruta och betalar tillbaka halva kostnaden.
pub fn riv(world: &mut World, tile: Tile) -> bool {
    let Some((entity, byggnad)) = byggnad_pa(world, tile) else {
        return false;
    };
    world.despawn(entity);

    if let Some(kassa_entity) = kassa_entity(world)
        && let Some(mut k) = world.get_mut::<Stadskassa>(kassa_entity)
    {
        k.guld += byggnad.kind.kostnad() * 0.5;
    }
    true
}

/// Bygger kartan i en värld: mark, stadshus och inget mer.
///
/// Körs av `generate_level` för att skriva scenfilen. Spelet laddar
/// scenen i stället för att anropa den här – kartan är data.
pub fn bygg_karta(world: &mut World) -> usize {
    let mut count = 0;
    for x in -HALF..=HALF {
        for z in -HALF..=HALF {
            // Schackrutigt, så att rutnätet syns utan texturer.
            let ljus = (x + z).rem_euclid(2) == 0;
            let color = if ljus {
                Color::rgb(0.38, 0.47, 0.30)
            } else {
                Color::rgb(0.34, 0.43, 0.27)
            };
            world.spawn((
                EntityName::new(format!("Mark {x},{z}")),
                Transform {
                    translation: Vec3::new(x as f32 * CELL, -0.05, z as f32 * CELL),
                    rotation: ymer_core::Quat::IDENTITY,
                    scale: Vec3::new(CELL * 0.98, 0.1, CELL * 0.98),
                },
                GlobalTransform::default(),
                MeshInstance::new(ymer_core::BUILTIN_CUBE, color),
                Mark {
                    tile_x: x,
                    tile_z: z,
                },
            ));
            count += 1;
        }
    }

    // Stadshuset bär kassan och skriptet. Det har ingen mesh: det är
    // bokföringen, inte ett hus på kartan.
    world.spawn((
        EntityName::new("Stadshuset"),
        Transform::from_xyz(0.0, 0.0, 0.0),
        GlobalTransform::default(),
        Stadskassa::default(),
        Script::new(SCRIPT),
    ));
    count + 1
}

/// Rutan muspekaren pekar på, eller None om strålen missar marken.
///
/// Samma `view_proj` som renderaren använder, inverterad: två punkter på
/// nära och bortre klippplanet ger strålen, och den skär planet y=0.
/// Räknas den ut på något annat sätt än renderaren hamnar klicket på fel
/// ruta utan att något syns i koden.
pub fn ruta_under_musen(view_proj: Mat4, mus: Vec2, skarm: Vec2) -> Option<Tile> {
    if skarm.x <= 0.0 || skarm.y <= 0.0 {
        return None;
    }
    let inverse = view_proj.inverse();

    // wgpu: x och y i [-1,1] med y uppåt, djup i [0,1].
    let ndc_x = 2.0 * mus.x / skarm.x - 1.0;
    let ndc_y = 1.0 - 2.0 * mus.y / skarm.y;

    let unproject = |djup: f32| {
        let p = inverse * ymer_core::Vec4::new(ndc_x, ndc_y, djup, 1.0);
        if p.w.abs() < 1e-6 {
            None
        } else {
            Some(Vec3::new(p.x / p.w, p.y / p.w, p.z / p.w))
        }
    };

    let nara = unproject(0.0)?;
    let bortre = unproject(1.0)?;
    let riktning = bortre - nara;
    if riktning.y.abs() < 1e-6 {
        return None;
    }

    let t = -nara.y / riktning.y;
    if t < 0.0 {
        return None;
    }
    let traff = nara + riktning * t;

    let tile = (
        (traff.x / CELL).round() as i32,
        (traff.z / CELL).round() as i32,
    );
    inom_kartan(tile).then_some(tile)
}

/// Kamerans view-projection, uträknad precis som renderaren gör det.
pub fn view_proj(world: &mut World, aspect: f32) -> Option<Mat4> {
    let mut query = world.query::<(&ymer_core::Camera, &GlobalTransform)>();
    query
        .iter(world)
        .next()
        .map(|(camera, global)| camera.projection(aspect) * global.0.inverse())
}
