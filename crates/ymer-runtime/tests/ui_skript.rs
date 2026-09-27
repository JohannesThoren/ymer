//! Gränssnittet styrt från TypeScript.
//!
//! Testet kör `tests/skript/hud.ts` genom den riktiga värden: transformen
//! till JS, QuickJS i wasm, kommandobufferten tillbaka. Interaktionen är
//! också riktig – pekaren trycks ned och släpps över nodernas verkliga
//! rektanglar, och de händelser skriptet läser är de `ymer-ui` självt
//! producerade. Ingenting här skriver `UiEvents` för hand, eftersom det
//! hade kunnat vara grönt medan träffprövningen var trasig.

use bevy_ecs::prelude::*;
use ymer_core::{ConsoleLog, Input as MotorInput, Script, Time, UiDocument, UiEvents};
use ymer_runtime::ScriptHost;
use ymer_scene::{TypeRegistry, register_builtin_types};
use ymer_ui::prelude::*;
use ymer_ui::{Input as UiInput, Key, LaidOut, State};

struct Prov {
    world: World,
    host: ScriptHost,
    registry: TypeRegistry,
    state: State,
    metrics: MonospaceMetrics,
    viewport: Rect,
}

impl Prov {
    fn start() -> Self {
        let mut registry = TypeRegistry::new();
        register_builtin_types(&mut registry);

        let mut world = World::new();
        world.insert_resource(Time::new());
        world.insert_resource(MotorInput::default());
        world.insert_resource(ConsoleLog::default());
        world.insert_resource(UiDocument(hud()));
        world.insert_resource(UiEvents::default());
        // Skriptet körs som ett system, men behöver ändå en entitet som
        // bär det – precis som i ett spel.
        world.spawn(Script("hud.ts".to_string()));

        let wasm = std::fs::read("../../assets/script_host.wasm").expect("script_host.wasm");
        let host = ScriptHost::new(wasm, "tests/skript");

        Self {
            world,
            host,
            registry,
            state: State::default(),
            metrics: MonospaceMetrics::default(),
            viewport: Rect::new(0.0, 0.0, 400.0, 600.0),
        }
    }

    fn layout(&mut self) -> LaidOut {
        let document = self.world.resource::<UiDocument>();
        layout(&document.0, self.viewport, &self.metrics)
    }

    /// Matar gränssnittet en frame och lägger händelserna där skriptet
    /// läser dem.
    fn mata(&mut self, input: &UiInput) {
        let laid_out = self.layout();
        let mut document = self.world.resource_mut::<UiDocument>();
        let events = self.state.update_with(&mut document.0, &laid_out, input);
        *self.world.resource_mut::<UiEvents>() = UiEvents(events);
    }

    /// Kör skriptet en frame och tömmer händelserna efteråt, som loopen gör.
    fn stega(&mut self) {
        self.host
            .tick(&mut self.world, &self.registry, 1.0 / 60.0)
            .expect("skriptet kör");
        *self.world.resource_mut::<UiEvents>() = UiEvents::default();
    }

    /// Ett riktigt klick: ned och upp över nodens egen rektangel.
    fn klicka(&mut self, id: &str) {
        let punkt = self.mitten(id);
        self.mata(&UiInput::from_pointer(Pointer {
            position: punkt,
            down: true,
            pressed: true,
            ..Default::default()
        }));
        self.stega();
        self.mata(&UiInput::from_pointer(Pointer {
            position: punkt,
            released: true,
            ..Default::default()
        }));
        self.stega();
    }

    /// Drar ett reglage till en andel av sitt spann, 0.0–1.0.
    ///
    /// Punkten räknas ut ur greppets geometri, inte ur rektangelns kant.
    /// Ett tryck på `rect.right()` ligger utanför rutan och träffar
    /// ingenting – reglaget hade då aldrig blivit aktivt, och testet hade
    /// mätt sin egen aritmetik istället för skriptet.
    fn dra(&mut self, id: &str, andel: f32) {
        let rect = self.rect(id);
        let spar = (rect.width - ymer_ui::metrics::HANDLE).max(1.0);
        let x = rect.x + ymer_ui::metrics::HANDLE * 0.5 + spar * andel;
        let punkt = Vec2::new(x, rect.center().y);
        self.mata(&UiInput::from_pointer(Pointer {
            position: punkt,
            down: true,
            pressed: true,
            ..Default::default()
        }));
        self.stega();
        // Släpp greppet. Ett reglage behåller pekaren så länge knappen är
        // nere, även utanför sin egen rektangel – utan släppet drog nästa
        // klick någon annanstans i fönstret reglaget med sig.
        self.mata(&UiInput::from_pointer(Pointer {
            position: punkt,
            released: true,
            ..Default::default()
        }));
        self.stega();
    }

    /// Skriver text i det fokuserade fältet, och trycker sedan tangenter.
    fn skriv(&mut self, text: &str, keys: &[Key]) {
        self.mata(&UiInput {
            text: text.to_string(),
            keys: keys.to_vec(),
            ..Default::default()
        });
        self.stega();
    }

    fn rect(&mut self, id: &str) -> Rect {
        self.layout()
            .rect(id)
            .unwrap_or_else(|| panic!("{id} finns i layouten"))
    }

    fn mitten(&mut self, id: &str) -> Vec2 {
        self.rect(id).center()
    }

    fn text(&self, id: &str) -> String {
        match self.world.resource::<UiDocument>().value(id) {
            Some(NodeValue::Text(text)) => text,
            annat => panic!("{id} bär ingen text: {annat:?}"),
        }
    }

    /// Etiketternas text ligger i noden, inte som värde.
    fn etikett(&self, id: &str) -> String {
        self.world
            .resource::<UiDocument>()
            .find(id)
            .and_then(|node| node.kind.text())
            .unwrap_or_else(|| panic!("{id} är ingen etikett"))
            .to_string()
    }

    fn synlig(&self, id: &str) -> bool {
        self.world
            .resource::<UiDocument>()
            .find(id)
            .map(|node| node.style.visible)
            .unwrap_or_else(|| panic!("{id} finns"))
    }

    fn varde(&self, id: &str) -> Option<NodeValue> {
        self.world.resource::<UiDocument>().value(id)
    }

    /// Fel i skriptet hamnar i konsolen, inte i ett Err. Utan den här
    /// kontrollen blir ett stavfel i .ts-filen ett tyst grönt test.
    fn inga_fel(&self) {
        let fel = klagomal(&self.world);
        assert!(fel.is_empty(), "skriptet klagade: {fel:?}");
    }
}

/// Varningar och fel i konsolen. Ett stavfel i .ts-filen syns bara här –
/// `tick` returnerar Ok, så utan den här kontrollen blir ett trasigt
/// skript ett tyst grönt test.
fn klagomal(world: &World) -> Vec<String> {
    world
        .resource::<ConsoleLog>()
        .entries()
        .filter(|entry| entry.level >= ymer_core::LogLevel::Warn)
        .map(|entry| format!("{}: {}", entry.source, entry.message))
        .collect()
}

fn hud() -> Document {
    Document::new(
        Node::panel()
            .with_style(
                Style::column()
                    .with_size(Size::Fill, Size::Fill)
                    .with_padding(Edges::all(10.0))
                    .with_gap(8.0),
            )
            .with_children([
                Node::button("bygg", "Bygg hus")
                    .with_style(Style::default().with_size(Size::Fixed(160.0), Size::Fixed(30.0))),
                Node::button("aterstall", "Återställ")
                    .with_style(Style::default().with_size(Size::Fixed(160.0), Size::Fixed(30.0))),
                Node::label("status").with_id("status"),
                Node::label("volym 50").with_id("volym_text"),
                Node::label("hej").with_id("halsning"),
                Node::checkbox("avancerat", "Avancerat", true),
                Node::slider("volym", 50.0, 0.0, 100.0)
                    .with_style(Style::default().with_size(Size::Fixed(200.0), Size::Fixed(20.0))),
                Node::dropdown("svarighet", ["Lätt", "Normal", "Svår"], Some(1))
                    .with_style(Style::default().with_size(Size::Fixed(200.0), Size::Fixed(26.0))),
                Node::text_input("namn", "")
                    .with_style(Style::default().with_size(Size::Fixed(200.0), Size::Fixed(26.0))),
            ]),
    )
}

#[test]
fn klick_lases_och_text_skrivs_tillbaka() {
    let mut prov = Prov::start();
    assert_eq!(prov.etikett("status"), "status");

    prov.klicka("bygg");
    assert_eq!(prov.etikett("status"), "byggt 1");

    prov.klicka("bygg");
    assert_eq!(
        prov.etikett("status"),
        "byggt 2",
        "skriptet håller sitt eget tillstånd mellan frames"
    );

    prov.inga_fel();
}

#[test]
fn en_frame_utan_handelser_ror_ingenting() {
    let mut prov = Prov::start();
    prov.stega();
    prov.stega();
    assert_eq!(prov.etikett("status"), "status");
    prov.inga_fel();
}

#[test]
fn kryssruta_styr_synlighet() {
    let mut prov = Prov::start();
    assert!(prov.synlig("bygg"));

    // Bocka av: changed + checked(false) -> knappen göms.
    prov.klicka("avancerat");
    assert_eq!(prov.varde("avancerat"), Some(NodeValue::Bool(false)));
    assert!(!prov.synlig("bygg"), "skriptet gömde knappen");

    prov.klicka("avancerat");
    assert!(prov.synlig("bygg"));
    prov.inga_fel();
}

#[test]
fn reglage_lases_som_siffra() {
    let mut prov = Prov::start();
    prov.dra("volym", 1.0);
    assert_eq!(prov.etikett("volym_text"), "volym 100");

    prov.dra("volym", 0.0);
    assert_eq!(prov.etikett("volym_text"), "volym 0");

    // Mitt i spannet: etiketten ska följa dokumentets faktiska värde, inte
    // ett tal testet räknat ut på egen hand.
    prov.dra("volym", 0.25);
    let Some(NodeValue::Number(varde)) = prov.varde("volym") else {
        panic!("reglaget bär en siffra");
    };
    assert_eq!(
        prov.etikett("volym_text"),
        format!("volym {}", varde.round())
    );
    prov.inga_fel();
}

#[test]
fn faltet_tas_emot_med_enter_och_nollas() {
    let mut prov = Prov::start();

    // Fokusera fältet genom att klicka i det.
    let punkt = prov.mitten("namn");
    prov.mata(&UiInput::from_pointer(Pointer {
        position: punkt,
        down: true,
        pressed: true,
        ..Default::default()
    }));
    prov.stega();

    prov.skriv("Johannes", &[]);
    assert_eq!(prov.text("namn"), "Johannes");
    assert_eq!(
        prov.etikett("halsning"),
        "hej",
        "inget ska hända förrän Enter"
    );

    prov.skriv("", &[Key::Enter]);
    assert_eq!(prov.etikett("halsning"), "hej Johannes");
    assert_eq!(prov.text("namn"), "", "skriptet nollade fältet");
    prov.inga_fel();
}

#[test]
fn dropdown_ger_index_och_kan_nollstallas() {
    let mut prov = Prov::start();
    assert_eq!(prov.varde("svarighet"), Some(NodeValue::Index(Some(1))));

    // Listan öppnas av ett fullbordat klick. Raderna ligger under fältet
    // och är inga egna noder, så punkten räknas ut på samma sätt som
    // träffprövningen gör det.
    prov.klicka("svarighet");
    let rect = prov.rect("svarighet");
    let rad = Rect::new(
        rect.x,
        rect.bottom() + rect.height * 2.0,
        rect.width,
        rect.height,
    );
    prov.mata(&UiInput::from_pointer(Pointer {
        position: rad.center(),
        down: true,
        pressed: true,
        ..Default::default()
    }));
    prov.stega();

    assert_eq!(prov.varde("svarighet"), Some(NodeValue::Index(Some(2))));
    assert_eq!(prov.etikett("status"), "grad 2");
    prov.inga_fel();
}

#[test]
fn skriptet_skriver_i_andra_riktningen() {
    let mut prov = Prov::start();
    prov.dra("volym", 1.0);

    prov.klicka("aterstall");

    assert_eq!(prov.varde("volym"), Some(NodeValue::Number(0.0)));
    assert_eq!(
        prov.varde("svarighet"),
        Some(NodeValue::Index(None)),
        "setSelected(null) nollställer valet"
    );
    assert_eq!(prov.varde("avancerat"), Some(NodeValue::Bool(false)));
    prov.inga_fel();
}

#[test]
fn skript_mot_tomt_granssnitt_kastar_inte() {
    // Ett skript ska kunna skrivas mot en HUD som inte är laddad. Utan
    // fallbacken i värden kastar ui.text() på undefined, och hela
    // skriptet dör i en frame utan gränssnitt.
    let mut prov = Prov::start();
    prov.world.remove_resource::<UiDocument>();
    prov.world.remove_resource::<UiEvents>();

    prov.host
        .tick(&mut prov.world, &prov.registry, 1.0 / 60.0)
        .expect("skriptet kör utan gränssnitt");

    let fel = klagomal(&prov.world);
    assert!(fel.is_empty(), "skriptet klagade: {fel:?}");
}
