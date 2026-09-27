//! Spelar staden utan fönster.
//!
//! Testerna kör det *riktiga* skriptet: samma `ekonomi.ts` som spelet
//! laddar, genom samma `ScriptHost`, mot samma scenfil. Ändras en regel i
//! skriptet ändras testet med – det är poängen. En Rust-kopia av
//! formlerna hade kunnat vara grön medan spelet räknade fel.

use bevy_ecs::prelude::*;
use stad::{Byggnad, Kind, Neka, Stadskassa, bygg, riv};
use ymer_core::{
    Camera, ConsoleLog, EntityName, GlobalTransform, Input, Time, Transform, Vec2, Vec3,
};
use ymer_runtime::ScriptHost;
use ymer_scene::{Scene, TypeRegistry, register_builtin_types};

const PROJECT: &str = "../../projects/stad";

struct Spel {
    world: World,
    host: ScriptHost,
    registry: TypeRegistry,
}

impl Spel {
    fn start() -> Self {
        let mut registry = TypeRegistry::new();
        register_builtin_types(&mut registry);
        stad::register_types(&mut registry);

        let mut world = World::new();
        world.insert_resource(Time::new());
        world.insert_resource(Input::default());
        world.insert_resource(ConsoleLog::default());

        // Samma scenfil som spelet laddar – inte en handbyggd värld.
        let scene = Scene::load(format!("{PROJECT}/scenes/main.ron")).expect("kartan finns");
        scene
            .spawn_into(&mut world, &registry)
            .expect("kartan går att spawna");

        let wasm = std::fs::read("../../assets/script_host.wasm").expect("script_host.wasm");
        let host = ScriptHost::new(wasm, format!("{PROJECT}/scripts"));

        Self {
            world,
            host,
            registry,
        }
    }

    /// Stegar spelet som loopen gör, i 60-delar.
    fn stega(&mut self, sekunder: f32) {
        let steg: f32 = 1.0 / 60.0;
        let mut kvar = sekunder;
        while kvar > 0.0 {
            self.host
                .tick(&mut self.world, &self.registry, steg.min(kvar))
                .expect("skriptet kör");
            self.world.resource_mut::<Input>().end_frame();
            kvar -= steg;
        }
    }

    fn kassa(&mut self) -> Stadskassa {
        stad::kassa(&mut self.world)
    }

    fn byggnad(&mut self, tile: stad::Tile) -> Byggnad {
        stad::byggnad_pa(&mut self.world, tile)
            .map(|(_, b)| b)
            .unwrap_or_else(|| panic!("ingen byggnad på {tile:?}"))
    }

    fn ge_guld(&mut self, guld: f32) {
        let entity = stad::kassa_entity(&mut self.world).expect("stadshuset finns");
        self.world.get_mut::<Stadskassa>(entity).unwrap().guld = guld;
    }
}

#[test]
fn kartan_laddas_ur_scenfilen() {
    let mut spel = Spel::start();
    let mark = {
        let mut query = spel.world.query::<&stad::Mark>();
        query.iter(&spel.world).count()
    };
    let sidan = (stad::HALF * 2 + 1) as usize;
    assert_eq!(mark, sidan * sidan, "hela rutnätet ska komma ur scenen");
    assert_eq!(
        spel.kassa().guld,
        200.0,
        "startkassan står i scenfilen, inte i koden"
    );
}

#[test]
fn bygge_drar_guld_och_upptar_rutan() {
    let mut spel = Spel::start();
    let fore = spel.kassa().guld;

    bygg(&mut spel.world, (0, 0), Kind::Hus).expect("ska gå att bygga");

    assert_eq!(spel.kassa().guld, fore - Kind::Hus.kostnad());
    assert_eq!(spel.byggnad((0, 0)).kind, Kind::Hus);
}

#[test]
fn upptagen_ruta_nekas() {
    let mut spel = Spel::start();
    bygg(&mut spel.world, (1, 1), Kind::Hus).unwrap();
    assert_eq!(
        bygg(&mut spel.world, (1, 1), Kind::Butik),
        Err(Neka::Upptaget)
    );
}

#[test]
fn utanfor_kartan_nekas() {
    let mut spel = Spel::start();
    assert_eq!(
        bygg(&mut spel.world, (stad::HALF + 1, 0), Kind::Hus),
        Err(Neka::UtanforKartan)
    );
}

#[test]
fn utan_rad_nekas() {
    let mut spel = Spel::start();
    spel.ge_guld(10.0);
    assert_eq!(bygg(&mut spel.world, (0, 0), Kind::Hus), Err(Neka::ForDyrt));
}

#[test]
fn skriptet_fyller_i_vad_byggnaden_gor() {
    // Rust sätter bara sorten. Att ett hus ger fyra platser står i
    // ekonomi.ts, och syns först när skriptet kört en frame.
    let mut spel = Spel::start();
    bygg(&mut spel.world, (0, 0), Kind::Hus).unwrap();
    assert_eq!(spel.byggnad((0, 0)).platser, 0, "innan skriptet kört");

    spel.stega(1.0 / 60.0);
    assert_eq!(spel.byggnad((0, 0)).platser, 4, "skriptet ska ha fyllt i");
}

#[test]
fn invanare_flyttar_in_upp_till_platserna() {
    let mut spel = Spel::start();
    bygg(&mut spel.world, (0, 0), Kind::Hus).unwrap();

    spel.stega(10.0);
    let k = spel.kassa();
    assert_eq!(k.platser, 4);
    assert_eq!(k.invanare, 4.0, "inflyttningen ska stanna vid platserna");
}

#[test]
fn butik_utan_folk_ger_inget() {
    let mut spel = Spel::start();
    bygg(&mut spel.world, (0, 0), Kind::Butik).unwrap();
    let fore = spel.kassa().guld;

    spel.stega(5.0);
    assert_eq!(
        spel.kassa().guld,
        fore,
        "en obemannad butik ska inte producera något"
    );
}

#[test]
fn bemannad_butik_ger_guld() {
    let mut spel = Spel::start();
    bygg(&mut spel.world, (0, 0), Kind::Hus).unwrap();
    bygg(&mut spel.world, (1, 0), Kind::Butik).unwrap();

    spel.stega(3.0);
    let k = spel.kassa();
    assert_eq!(k.jobb, 2);
    assert_eq!(k.sysselsatta, 2);
    assert!(k.inkomst > 1.9, "full bemanning ska ge full inkomst");
    assert_eq!(spel.byggnad((1, 0)).bemanning, 2);

    let fore = spel.kassa().guld;
    spel.stega(2.0);
    assert!(spel.kassa().guld > fore + 3.0, "guldet skulle ha ökat");
}

#[test]
fn halvbemannad_arbetsplats_ger_andel() {
    let mut spel = Spel::start();
    spel.ge_guld(1000.0);
    // Ett hus ger fyra platser; ett kontor vill ha tolv.
    bygg(&mut spel.world, (0, 0), Kind::Hus).unwrap();
    bygg(&mut spel.world, (1, 0), Kind::Kontor).unwrap();

    spel.stega(3.0);
    let k = spel.kassa();
    let kontor = spel.byggnad((1, 0));
    assert_eq!(k.sysselsatta, 4);
    assert_eq!(kontor.bemanning, 4);

    let vantad = kontor.inkomst * 4.0 / 12.0;
    assert!(
        (k.inkomst - vantad).abs() < 0.01,
        "väntade {vantad}, fick {}",
        k.inkomst
    );
}

#[test]
fn rivning_ger_tillbaka_halva_och_folk_flyttar_ut() {
    let mut spel = Spel::start();
    bygg(&mut spel.world, (0, 0), Kind::Hus).unwrap();
    spel.stega(5.0);
    assert_eq!(spel.kassa().invanare, 4.0);

    let fore = spel.kassa().guld;
    assert!(riv(&mut spel.world, (0, 0)));
    assert_eq!(spel.kassa().guld, fore + Kind::Hus.kostnad() * 0.5);

    // Utan boende finns ingen plats kvar; skriptet flyttar ut folket.
    spel.stega(1.0 / 30.0);
    assert_eq!(
        spel.kassa().invanare,
        0.0,
        "folk ska flytta ut när boendet rivs"
    );
}

#[test]
fn rivning_tar_bort_entiteten() {
    let mut spel = Spel::start();
    bygg(&mut spel.world, (2, 2), Kind::Hus).unwrap();
    riv(&mut spel.world, (2, 2));
    assert!(stad::byggnad_pa(&mut spel.world, (2, 2)).is_none());
}

/// En värld med samma kamera som spelet sätter upp.
fn varld_med_kamera(skarm: Vec2) -> (World, ymer_core::Mat4) {
    let mut world = World::new();
    world.spawn((
        EntityName::new("Main Camera"),
        Transform::from_xyz(0.0, 11.0, 11.0).looking_at(Vec3::ZERO, Vec3::Y),
        GlobalTransform::default(),
        Camera::default(),
    ));
    ymer_core::propagate_transforms(&mut world);
    let view_proj = stad::view_proj(&mut world, skarm.x / skarm.y).expect("kameran finns");
    (world, view_proj)
}

/// Plocket måste använda exakt samma matris som renderaren, annars
/// hamnar klicket på fel ruta. Testet går andra vägen: projicera en känd
/// ruta till skärmen och plocka tillbaka den – genom motorns egen
/// kameraväg, inte en handbyggd matris.
#[test]
fn plocket_hittar_ratt_ruta() {
    let skarm = Vec2::new(1280.0, 720.0);
    let (_world, view_proj) = varld_med_kamera(skarm);

    for tile in [(0, 0), (3, -2), (-4, 4), (stad::HALF, stad::HALF)] {
        let varld_punkt = Vec3::new(tile.0 as f32, 0.0, tile.1 as f32);
        let klipp = view_proj * varld_punkt.extend(1.0);
        let ndc = klipp.truncate() / klipp.w;
        let mus = Vec2::new((ndc.x + 1.0) * 0.5 * skarm.x, (1.0 - ndc.y) * 0.5 * skarm.y);

        assert_eq!(
            stad::ruta_under_musen(view_proj, mus, skarm),
            Some(tile),
            "rutan {tile:?} projicerad till {mus:?} plockades inte tillbaka"
        );
    }
}

#[test]
fn plock_utanfor_kartan_ger_inget() {
    let skarm = Vec2::new(1280.0, 720.0);
    let (_world, view_proj) = varld_med_kamera(skarm);

    // Högst upp i bild pekar strålen mot horisonten, långt utanför kartan.
    assert_eq!(
        stad::ruta_under_musen(view_proj, Vec2::new(640.0, 2.0), skarm),
        None
    );
}
