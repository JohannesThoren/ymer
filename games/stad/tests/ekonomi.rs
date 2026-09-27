//! Spelar staden utan fönster. Ekonomin och rutnätet är ren logik, så
//! hela slingan går att verifiera utan GPU.

use bevy_ecs::prelude::*;
use stad::{Kind, Neka, Stad, bygg, riv, tick};
use ymer_core::{Camera, EntityName, GlobalTransform, Transform, Vec2, Vec3};

fn varld() -> World {
    let mut world = World::new();
    world.insert_resource(Stad::default());
    world
}

/// Stegar ekonomin i små steg, som spelloopen gör.
fn stega(stad: &mut Stad, sekunder: f32) {
    let steg: f32 = 1.0 / 60.0;
    let mut kvar = sekunder;
    while kvar > 0.0 {
        tick(stad, steg.min(kvar));
        kvar -= steg;
    }
}

#[test]
fn bygge_drar_guld_och_upptar_rutan() {
    let mut world = varld();
    let fore = world.resource::<Stad>().guld;

    bygg(&mut world, (0, 0), Kind::Hus).expect("ska gå att bygga");

    let stad = world.resource::<Stad>();
    assert_eq!(stad.guld, fore - Kind::Hus.kostnad());
    assert_eq!(stad.upptagen((0, 0)), Some(Kind::Hus));
    assert_eq!(stad.antal(), 1);
}

#[test]
fn upptagen_ruta_nekas() {
    let mut world = varld();
    bygg(&mut world, (1, 1), Kind::Hus).unwrap();
    assert_eq!(bygg(&mut world, (1, 1), Kind::Butik), Err(Neka::Upptaget));
}

#[test]
fn utanfor_kartan_nekas() {
    let mut world = varld();
    assert_eq!(
        bygg(&mut world, (stad::HALF + 1, 0), Kind::Hus),
        Err(Neka::UtanforKartan)
    );
}

#[test]
fn utan_rad_nekas() {
    let mut world = varld();
    world.resource_mut::<Stad>().guld = 10.0;
    assert_eq!(bygg(&mut world, (0, 0), Kind::Hus), Err(Neka::ForDyrt));
}

#[test]
fn invanare_flyttar_in_upp_till_platserna() {
    let mut world = varld();
    bygg(&mut world, (0, 0), Kind::Hus).unwrap();

    let mut stad = world.resource::<Stad>().clone();
    assert_eq!(stad.nyckeltal().platser, 4);

    stega(&mut stad, 10.0);
    assert_eq!(
        stad.invanare, 4.0,
        "inflyttningen ska stanna vid antalet platser"
    );
}

#[test]
fn butik_utan_folk_ger_inget() {
    let mut world = varld();
    bygg(&mut world, (0, 0), Kind::Butik).unwrap();

    let mut stad = world.resource::<Stad>().clone();
    let fore = stad.guld;
    stega(&mut stad, 5.0);
    assert_eq!(
        stad.guld, fore,
        "en obemannad butik ska inte producera något"
    );
}

#[test]
fn bemannad_butik_ger_guld() {
    let mut world = varld();
    bygg(&mut world, (0, 0), Kind::Hus).unwrap();
    bygg(&mut world, (1, 0), Kind::Butik).unwrap();

    let mut stad = world.resource::<Stad>().clone();
    stega(&mut stad, 3.0); // hinner fylla husets platser
    let tal = stad.nyckeltal();
    assert_eq!(tal.jobb, 2);
    assert_eq!(tal.sysselsatta, 2);
    assert!(
        (tal.inkomst - Kind::Butik.inkomst()).abs() < 0.001,
        "fullt bemannad butik ska ge full inkomst, gav {}",
        tal.inkomst
    );

    let fore = stad.guld;
    stega(&mut stad, 2.0);
    assert!(stad.guld > fore + 3.0, "guldet skulle ha ökat");
}

#[test]
fn halvbemannad_arbetsplats_ger_andel() {
    let mut world = varld();
    // Ett kontor kostar mer än startkassan – det är meningen, så testet
    // får fylla på i stället för att spela sig dit.
    world.resource_mut::<Stad>().guld = 1000.0;
    // Ett hus ger fyra platser; ett kontor vill ha tolv.
    bygg(&mut world, (0, 0), Kind::Hus).unwrap();
    bygg(&mut world, (1, 0), Kind::Kontor).unwrap();

    let mut stad = world.resource::<Stad>().clone();
    stega(&mut stad, 3.0);
    let tal = stad.nyckeltal();
    assert_eq!(tal.sysselsatta, 4);
    let vantad = Kind::Kontor.inkomst() * 4.0 / 12.0;
    assert!(
        (tal.inkomst - vantad).abs() < 0.001,
        "väntade {vantad}, fick {}",
        tal.inkomst
    );
}

#[test]
fn rivning_ger_tillbaka_halva_och_folk_flyttar_ut() {
    let mut world = varld();
    bygg(&mut world, (0, 0), Kind::Hus).unwrap();

    let mut stad = world.resource::<Stad>().clone();
    stega(&mut stad, 5.0);
    assert_eq!(stad.invanare, 4.0);
    *world.resource_mut::<Stad>() = stad;

    let fore = world.resource::<Stad>().guld;
    assert!(riv(&mut world, (0, 0)));

    let stad = world.resource::<Stad>();
    assert_eq!(stad.guld, fore + Kind::Hus.kostnad() * 0.5);
    assert_eq!(stad.antal(), 0);

    // Utan boende finns ingen plats kvar; nästa tick flyttar ut folket.
    let mut stad = stad.clone();
    tick(&mut stad, 1.0 / 60.0);
    assert_eq!(stad.invanare, 0.0, "folk ska flytta ut när boendet rivs");
}

#[test]
fn rivning_tar_bort_entiteten() {
    let mut world = varld();
    bygg(&mut world, (2, 2), Kind::Hus).unwrap();
    let mut query = world.query::<&stad::Byggnad>();
    assert_eq!(query.iter(&world).count(), 1);

    riv(&mut world, (2, 2));
    let mut query = world.query::<&stad::Byggnad>();
    assert_eq!(query.iter(&world).count(), 0);
}

/// En värld med samma kamera som spelet sätter upp.
fn varld_med_kamera(skarm: Vec2) -> (World, ymer_core::Mat4) {
    let mut world = varld();
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
