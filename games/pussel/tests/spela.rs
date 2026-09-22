//! Spelar banan utan fönster: laddar scenen, matar in tangenttryck och
//! kollar att reglerna håller. Ingen GPU behövs – skript, scen och ECS
//! klarar sig utan renderare, så modellerna behöver inte ens laddas.

use bevy_ecs::world::World;
use ymer_core::{ConsoleLog, EntityName, Input, Time, Transform};
use ymer_runtime::ScriptHost;
use ymer_scene::{Scene, TypeRegistry, register_builtin_types};

const PROJECT: &str = "../../projects/pussel";

struct Game {
    world: World,
    host: ScriptHost,
    registry: TypeRegistry,
}

impl Game {
    fn start() -> Self {
        let mut registry = TypeRegistry::new();
        register_builtin_types(&mut registry);

        let mut world = World::new();
        world.insert_resource(Time::new());
        world.insert_resource(Input::default());
        world.insert_resource(ConsoleLog::default());

        let scene = Scene::load(format!("{PROJECT}/scenes/main.ron")).expect("scenen finns");
        scene
            .spawn_into(&mut world, &registry)
            .expect("scenen går att spawna");

        let wasm = std::fs::read("../../assets/script_host.wasm").expect("script_host.wasm");
        let host = ScriptHost::new(wasm, format!("{PROJECT}/scripts"));

        Self {
            world,
            host,
            registry,
        }
    }

    /// En frame med en tangent nedtryckt, sedan en utan – annars ligger
    /// "just pressed" kvar och spelaren fortsätter gå.
    fn press(&mut self, key: &str) {
        self.world.resource_mut::<Input>().press(key);
        self.frame();
        self.world.resource_mut::<Input>().release(key);
        self.frame();
    }

    fn frame(&mut self) {
        self.host
            .tick(&mut self.world, &self.registry, 1.0 / 60.0)
            .expect("skriptet kör");
        self.world.resource_mut::<Input>().end_frame();
    }

    fn transform(&mut self, name: &str) -> Transform {
        let mut query = self.world.query::<(&EntityName, &Transform)>();
        for (entity_name, transform) in query.iter(&self.world) {
            if entity_name.0 == name {
                return *transform;
            }
        }
        panic!("hittade ingen entitet som heter {name}");
    }

    fn position(&mut self, name: &str) -> (f32, f32) {
        let t = self.transform(name);
        (t.translation.x, t.translation.z)
    }

    fn height(&mut self, name: &str) -> f32 {
        self.transform(name).translation.y
    }

    fn log(&self) -> Vec<String> {
        self.world
            .resource::<ConsoleLog>()
            .entries()
            .map(|entry| entry.message.clone())
            .collect()
    }
}

/// Rutnätet som heltal, för läsbara jämförelser.
fn cell(position: (f32, f32)) -> (i32, i32) {
    (position.0.round() as i32, position.1.round() as i32)
}

#[test]
fn spelaren_gar_en_ruta_per_tryck() {
    let mut game = Game::start();
    let (x, z) = game.position("Spelare");

    game.press("KeyS");
    let (x2, z2) = game.position("Spelare");
    assert!((x2 - x).abs() < 0.01, "sidled ska inte ändras");
    assert!(
        (z2 - (z + 1.0)).abs() < 0.01,
        "S ska flytta en ruta neråt, hamnade på {z2} från {z}"
    );
}

#[test]
fn vaggen_stoppar() {
    let mut game = Game::start();
    let (x, _) = game.position("Spelare");
    game.press("KeyA");
    game.press("KeyA");
    let (x2, _) = game.position("Spelare");
    assert!(
        (x2 - (x - 1.0)).abs() < 0.01,
        "andra steget skulle ha stoppats av väggen, hamnade på {x2} från {x}"
    );
}

#[test]
fn ladan_skjuts_framfor_spelaren() {
    let mut game = Game::start();
    let before = cell(game.position("Låda 1"));

    game.press("KeyD");
    game.press("KeyD");
    let after = cell(game.position("Låda 1"));
    assert_eq!(
        after,
        (before.0 + 1, before.1),
        "lådan skulle ha skjutits en ruta åt höger"
    );
}

#[test]
fn huvudet_foljer_med_spelaren() {
    // Huvudet är en egen mesh, hopsatt med kroppen genom hierarkin. Går
    // ChildOf förlorad i scenfilen står huvudet kvar medan kroppen går.
    let mut game = Game::start();
    let before = game.transform("Spelare huvud").translation;
    game.press("KeyS");
    let after = game.transform("Spelare huvud").translation;
    assert_eq!(
        before, after,
        "huvudets *lokala* transform ska vara oförändrad – den ärver från kroppen"
    );

    // Och den globala ska ha följt med kroppen.
    ymer_core::propagate_transforms(&mut game.world);
    let body = game.transform("Spelare").translation;
    let mut query = game
        .world
        .query::<(&EntityName, &ymer_core::GlobalTransform)>();
    let head_global = query
        .iter(&game.world)
        .find(|(name, _)| name.0 == "Spelare huvud")
        .map(|(_, g)| g.0.transform_point3(ymer_core::Vec3::ZERO))
        .expect("huvudet finns");
    assert!(
        (head_global.x - body.x).abs() < 0.01 && (head_global.z - body.z).abs() < 0.01,
        "huvudet hamnade på {head_global:?} men kroppen står på {body:?}"
    );
}

#[test]
fn stangd_dorr_stoppar_spelaren() {
    let mut game = Game::start();
    game.press("KeyS");
    for _ in 0..3 {
        game.press("KeyD");
    }
    game.press("KeyS");
    game.press("KeyD");
    let (x, _) = game.position("Spelare");
    let door = game.position("Dörr 1");
    assert!(
        x < door.0 - 0.5,
        "spelaren gick igenom en stängd dörr: står på {x}, dörren på {}",
        door.0
    );
}

#[test]
fn ladan_pa_plattan_oppnar_dorren() {
    let mut game = Game::start();
    let height_closed = game.height("Dörr 1");

    // Tre steg höger skjuter Låda 1 upp på plattan.
    for _ in 0..3 {
        game.press("KeyD");
    }

    assert_eq!(
        cell(game.position("Låda 1")),
        cell(game.position("Platta 1")),
        "lådan skulle ha hamnat på plattan"
    );
    assert!(
        game.height("Dörr 1") < height_closed - 0.5,
        "dörren skulle ha sjunkit ner i golvet när plattan trycktes ner"
    );
}

#[test]
fn banan_gar_att_losa() {
    let mut game = Game::start();
    // Låda 1 upp på plattan, så dörren öppnas.
    for _ in 0..3 {
        game.press("KeyD");
    }
    // Runt till vänster om Låda 2 och skjut den genom dörren till målet.
    game.press("KeyS");
    for _ in 0..4 {
        game.press("KeyA");
    }
    game.press("KeyS");
    for _ in 0..5 {
        game.press("KeyD");
    }

    assert_eq!(
        cell(game.position("Låda 2")),
        cell(game.position("Mål 1")),
        "lådan skulle stå på målet"
    );
    assert!(
        game.log().iter().any(|line| line.contains("Klart!")),
        "loggen säger: {:?}",
        game.log()
    );
}
