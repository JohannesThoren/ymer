//! Ett litet stadsbyggarspel.
//!
//! Rutnät i xz-planet, ett hus per ruta. Bostäder ger platser, invånare
//! flyttar in, arbetsplatser behöver personal och ger guld, guld köper
//! fler hus. Hela slingan ryms i `tick`, som är ren logik utan fönster –
//! därför går ekonomin att testa headless.
//!
//! Grafiken är Kenneys "City Kit (Commercial)" (CC0), se
//! `models/KENNEY-LICENSE.txt`.

use std::collections::BTreeMap;

use bevy_ecs::prelude::*;
use ymer_core::{Color, EntityName, GlobalTransform, Mat4, MeshInstance, Transform, Vec2, Vec3};

pub mod ui;

/// Rutans storlek i världsenheter. Modellerna är gjorda för exakt det.
pub const CELL: f32 = 1.0;
/// Halva bredden på kartan i rutor: -HALF..=HALF i båda riktningar.
pub const HALF: i32 = 5;

/// En ruta i rutnätet.
pub type Tile = (i32, i32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
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

    pub fn kostnad(self) -> f32 {
        match self {
            Kind::Hus => 50.0,
            Kind::Butik => 80.0,
            Kind::Marknad => 140.0,
            Kind::Kontor => 300.0,
        }
    }

    /// Boendeplatser byggnaden tillför.
    pub fn platser(self) -> u32 {
        match self {
            Kind::Hus => 4,
            _ => 0,
        }
    }

    /// Hur många som behövs för att driva den.
    pub fn jobb(self) -> u32 {
        match self {
            Kind::Hus => 0,
            Kind::Butik => 2,
            Kind::Marknad => 5,
            Kind::Kontor => 12,
        }
    }

    /// Guld per sekund när den är fullt bemannad.
    pub fn inkomst(self) -> f32 {
        match self {
            Kind::Hus => 0.0,
            Kind::Butik => 2.0,
            Kind::Marknad => 5.0,
            Kind::Kontor => 14.0,
        }
    }

    pub fn beskrivning(self) -> &'static str {
        match self {
            Kind::Hus => "Ger 4 boendeplatser. Invånare flyttar in av sig själva.",
            Kind::Butik => "2 jobb, 2 guld/s när den är bemannad.",
            Kind::Marknad => "5 jobb, 5 guld/s när den är bemannad.",
            Kind::Kontor => "12 jobb, 14 guld/s när det är bemannat.",
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

#[derive(Component, Debug, Clone, Copy)]
pub struct Byggnad {
    pub kind: Kind,
    pub tile: Tile,
}

/// Marken. Egen komponent så att plocket kan skilja mark från byggnad.
#[derive(Component, Debug, Clone, Copy)]
pub struct Mark {
    pub tile: Tile,
}

/// Stadens tillstånd. Allt spelet vet om sig självt.
#[derive(Resource, Debug, Clone)]
pub struct Stad {
    pub guld: f32,
    /// Invånare som faktiskt flyttat in, växer mot `platser`.
    pub invanare: f32,
    /// Vad spelaren valt att bygga.
    pub vald: Kind,
    /// Markerad byggnad, om någon.
    pub markerad: Option<Tile>,
    /// Senaste meddelandet till spelaren, t.ex. varför ett bygge nekades.
    pub status: String,
    upptagna: BTreeMap<Tile, Kind>,
}

impl Default for Stad {
    fn default() -> Self {
        Self {
            guld: 200.0,
            invanare: 0.0,
            vald: Kind::Hus,
            markerad: None,
            status: "Bygg ett hus för att få invånare.".to_string(),
            upptagna: BTreeMap::new(),
        }
    }
}

/// Sammanräknad ställning, det HUD:en visar.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Nyckeltal {
    pub platser: u32,
    pub jobb: u32,
    pub sysselsatta: u32,
    pub inkomst: f32,
}

impl Stad {
    pub fn upptagen(&self, tile: Tile) -> Option<Kind> {
        self.upptagna.get(&tile).copied()
    }

    pub fn antal(&self) -> usize {
        self.upptagna.len()
    }

    pub fn nyckeltal(&self) -> Nyckeltal {
        let mut platser = 0;
        let mut jobb = 0;
        for kind in self.upptagna.values() {
            platser += kind.platser();
            jobb += kind.jobb();
        }

        // Arbetsplatserna bemannas i tur och ordning; den sista kan stå
        // halvbemannad och ger då bara sin andel.
        let arbetsfor = self.invanare.floor() as u32;
        let sysselsatta = arbetsfor.min(jobb);

        let mut kvar = sysselsatta;
        let mut inkomst = 0.0;
        for kind in self.upptagna.values() {
            let behov = kind.jobb();
            if behov == 0 {
                continue;
            }
            let fick = kvar.min(behov);
            kvar -= fick;
            inkomst += kind.inkomst() * (fick as f32 / behov as f32);
        }

        Nyckeltal {
            platser,
            jobb,
            sysselsatta,
            inkomst,
        }
    }
}

/// Varför ett bygge inte gick.
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

/// Bygger på en ruta. Returnerar entiteten, eller varför det inte gick.
pub fn bygg(world: &mut World, tile: Tile, kind: Kind) -> Result<Entity, Neka> {
    {
        let stad = world.resource::<Stad>();
        if !inom_kartan(tile) {
            return Err(Neka::UtanforKartan);
        }
        if stad.upptagen(tile).is_some() {
            return Err(Neka::Upptaget);
        }
        if stad.guld < kind.kostnad() {
            return Err(Neka::ForDyrt);
        }
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
            Byggnad { kind, tile },
        ))
        .id();

    let mut stad = world.resource_mut::<Stad>();
    stad.guld -= kind.kostnad();
    stad.upptagna.insert(tile, kind);
    stad.status = format!("{} byggt.", kind.namn());
    Ok(entity)
}

/// River byggnaden på en ruta och betalar tillbaka halva kostnaden.
pub fn riv(world: &mut World, tile: Tile) -> bool {
    let Some(kind) = world.resource::<Stad>().upptagen(tile) else {
        return false;
    };

    let offer: Vec<Entity> = {
        let mut query = world.query::<(Entity, &Byggnad)>();
        query
            .iter(world)
            .filter(|(_, b)| b.tile == tile)
            .map(|(e, _)| e)
            .collect()
    };
    for entity in offer {
        world.despawn(entity);
    }

    let mut stad = world.resource_mut::<Stad>();
    stad.guld += kind.kostnad() * 0.5;
    stad.upptagna.remove(&tile);
    if stad.markerad == Some(tile) {
        stad.markerad = None;
    }
    stad.status = format!("{} rivet, halva kostnaden tillbaka.", kind.namn());
    true
}

/// Ett tidssteg: invånare flyttar in och arbetsplatserna betalar ut.
///
/// Ren funktion av `dt`, utan fönster eller klocka, så att ett test kan
/// stega ekonomin hur snabbt det vill.
pub fn tick(stad: &mut Stad, dt: f32) {
    let tal = stad.nyckeltal();

    // Inflyttning: en person var halva sekund så länge det finns plats.
    let tak = tal.platser as f32;
    if stad.invanare < tak {
        stad.invanare = (stad.invanare + dt * 2.0).min(tak);
    } else if stad.invanare > tak {
        // Rivet boende: folk flyttar ut direkt.
        stad.invanare = tak;
    }

    stad.guld += tal.inkomst * dt;
}

/// Systemversionen, för `App::add_systems`.
pub fn ekonomi_system(mut stad: ResMut<Stad>, time: Res<ymer_core::Time>) {
    let dt = time.delta_seconds();
    tick(&mut stad, dt);
}

/// Lägger ut marken. Kallas en gång vid uppstart.
pub fn bygg_mark(world: &mut World) -> usize {
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
                Mark { tile: (x, z) },
            ));
            count += 1;
        }
    }
    count
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
        // Strålen är parallell med marken.
        return None;
    }

    let t = -nara.y / riktning.y;
    if t < 0.0 {
        // Marken ligger bakom kameran.
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
