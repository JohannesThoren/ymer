//! Editorns gränssnitt, utan fönster.
//!
//! Testerna kör den riktiga frame-slingan – samma `Chrome::frame` som
//! editorn anropar – och driver den med pekarhändelser på de rektanglar
//! layouten faktiskt gav noderna. Inget här bygger ett träd för hand.
//!
//! Två sorters fel är värda testerna. Det ena är att bygget och
//! avläsningen glider isär, så att ett fält går att ändra men inte
//! sparas. Det andra är motsatsen: att avläsningen rapporterar en ändring
//! varje frame och skriver tillbaka ett värde som aldrig rörts – det
//! syns inte alls, förrän en rotation långsamt vrider sig av sig själv.

use bevy_ecs::prelude::*;
use ymer_core::{Color, EntityName, MeshInstance, Transform};
use ymer_editor::EditorState;
use ymer_editor::chrome::Chrome;
use ymer_scene::{TypeRegistry, register_builtin_types};
use ymer_ui::prelude::*;
use ymer_ui::{Input, LaidOut, State};

const FONSTER: Rect = Rect {
    x: 0.0,
    y: 0.0,
    width: 1280.0,
    height: 800.0,
};

struct Editor {
    chrome: Chrome,
    state: EditorState,
    world: World,
    registry: TypeRegistry,
    metrics: MonospaceMetrics,
}

impl Editor {
    fn start() -> Self {
        let mut registry = TypeRegistry::new();
        register_builtin_types(&mut registry);

        let mut world = World::new();
        world.spawn((
            EntityName::new("Kub"),
            Transform::from_xyz(1.0, 2.0, 3.0),
            MeshInstance::new(ymer_core::BUILTIN_CUBE, Color::rgb(0.8, 0.2, 0.2)),
        ));

        let mut editor = Self {
            chrome: Chrome::new(),
            state: EditorState::default(),
            world,
            registry,
            metrics: MonospaceMetrics::default(),
        };
        // Första framen bygger trädet; utan den finns inga noder att peka på.
        editor.frame(Input::default());
        editor
    }

    fn frame(&mut self, input: Input) -> ymer_editor::chrome::Frame {
        self.chrome.frame(
            &input,
            FONSTER,
            &mut self.state,
            &mut self.world,
            &self.registry,
            &self.metrics,
        )
    }

    fn layout(&self) -> LaidOut {
        layout(self.chrome.document(), FONSTER, &self.metrics)
    }

    fn rect(&self, id: &str) -> Rect {
        self.layout()
            .rect(id)
            .unwrap_or_else(|| panic!("{id} finns i layouten"))
    }

    fn finns(&self, id: &str) -> bool {
        self.chrome.document().find(id).is_some()
    }

    fn klicka(&mut self, id: &str) {
        let punkt = self.rect(id).center();
        self.frame(Input::from_pointer(Pointer {
            position: punkt,
            down: true,
            pressed: true,
            ..Default::default()
        }));
        self.frame(Input::from_pointer(Pointer {
            position: punkt,
            released: true,
            ..Default::default()
        }));
    }

    /// Klickar på rubrikraden i ett hopfällbart avsnitt.
    fn klicka_rubrik(&mut self, id: &str) {
        let punkt = self
            .layout()
            .placed(id)
            .unwrap_or_else(|| panic!("{id} finns"))
            .inner
            .center();
        self.frame(Input::from_pointer(Pointer {
            position: punkt,
            down: true,
            pressed: true,
            ..Default::default()
        }));
        self.frame(Input::from_pointer(Pointer {
            position: punkt,
            released: true,
            ..Default::default()
        }));
    }

    fn dra(&mut self, id: &str, delta: Vec2) {
        let start = self.rect(id).center();
        self.frame(Input::from_pointer(Pointer {
            position: start,
            down: true,
            pressed: true,
            ..Default::default()
        }));
        self.frame(Input::from_pointer(Pointer {
            position: start + delta,
            down: true,
            ..Default::default()
        }));
        self.frame(Input::from_pointer(Pointer {
            position: start + delta,
            released: true,
            ..Default::default()
        }));
    }

    /// Entiteter i scenen – inte allt världen råkar innehålla. bevy
    /// lagrar resurser som entiteter, så `world.entities().len()` räknar
    /// saker som inte syns i hierarkin.
    fn antal_entiteter(&mut self) -> usize {
        self.world
            .query_filtered::<Entity, With<EntityName>>()
            .iter(&self.world)
            .count()
    }

    fn kuben(&mut self) -> Entity {
        self.world
            .query_filtered::<Entity, With<EntityName>>()
            .iter(&self.world)
            .next()
            .expect("kuben finns")
    }

    fn transform(&mut self) -> Transform {
        let entity = self.kuben();
        *self.world.get::<Transform>(entity).expect("har transform")
    }

    fn markera_kuben(&mut self) {
        let entity = self.kuben();
        self.klicka(&format!("hier/{}", entity.to_bits()));
    }
}

#[test]
fn hierarkin_visar_scenen() {
    let mut editor = Editor::start();
    let entity = editor.kuben();
    assert!(editor.finns(&format!("hier/{}", entity.to_bits())));
    assert_eq!(
        editor
            .chrome
            .document()
            .find(&format!("hier/{}", entity.to_bits()))
            .and_then(|node| node.kind.text()),
        Some("Kub")
    );
}

#[test]
fn ett_klick_i_hierarkin_markerar() {
    let mut editor = Editor::start();
    assert!(editor.state.selected.is_none());
    assert!(
        !editor.finns("f/Transform/translation/0"),
        "utan markering finns inga fält"
    );

    editor.markera_kuben();

    let kuben = editor.kuben();
    assert_eq!(editor.state.selected, Some(kuben));
    assert!(
        editor.finns("f/Transform/translation/0"),
        "inspektorn byggs om för den markerade entiteten"
    );
}

#[test]
fn ett_falt_visar_varldens_varde() {
    let mut editor = Editor::start();
    editor.markera_kuben();

    let node = editor
        .chrome
        .document()
        .find("f/Transform/translation/1")
        .expect("y-fältet finns");
    assert_eq!(node.value(), Some(NodeValue::Number(2.0)));
}

#[test]
fn ett_drag_i_ett_falt_skriver_till_varlden() {
    let mut editor = Editor::start();
    editor.markera_kuben();
    assert_eq!(editor.transform().translation.x, 1.0);

    // 20 px, steg 0.05: +1.0.
    editor.dra("f/Transform/translation/0", Vec2::new(20.0, 0.0));

    let x = editor.transform().translation.x;
    assert!((x - 2.0).abs() < 1e-5, "x blev {x}");
    assert_eq!(
        editor.transform().translation.y,
        2.0,
        "grannfälten ska inte följa med"
    );
}

#[test]
fn ororda_falt_skriver_inte_tillbaka() {
    // Det tystaste felet i en inspector: avläsningen ser en ändring varje
    // frame och skriver tillbaka. Ett värde som passerar fram och åter
    // genom en kvaternion driver då iväg av sig självt.
    let mut editor = Editor::start();
    editor.markera_kuben();

    let fore = editor.transform();
    for _ in 0..120 {
        editor.frame(Input::default());
    }
    let efter = editor.transform();

    assert_eq!(fore.translation, efter.translation);
    assert_eq!(
        fore.rotation, efter.rotation,
        "rotationen visas som grader och räknas tillbaka – den får inte vandra"
    );
    assert_eq!(fore.scale, efter.scale);
}

#[test]
fn rotationen_redigeras_i_grader() {
    let mut editor = Editor::start();
    editor.markera_kuben();

    // 30 px, steg 1.0 grad.
    editor.dra("f/Transform/rotation/deg1", Vec2::new(30.0, 0.0));

    let (y, _, _) = editor
        .transform()
        .rotation
        .to_euler(ymer_core::glam::EulerRot::YXZ);
    assert!(
        (y.to_degrees() - 30.0).abs() < 0.5,
        "blev {} grader",
        y.to_degrees()
    );
}

#[test]
fn en_farg_gar_att_andra() {
    let mut editor = Editor::start();
    editor.markera_kuben();

    // Steg 0.01, 20 px: +0.2.
    editor.dra("f/MeshInstance/color/g", Vec2::new(20.0, 0.0));

    let entity = editor.kuben();
    let color = editor.world.get::<MeshInstance>(entity).unwrap().color;
    assert!((color.g - 0.4).abs() < 1e-4, "g blev {}", color.g);
    assert!((color.r - 0.8).abs() < 1e-4, "r ska stå kvar");
}

#[test]
fn ett_avsnitt_gar_att_falla_ihop_och_stannar_hopfallt() {
    let mut editor = Editor::start();
    editor.markera_kuben();
    assert!(editor.finns("f/Transform/translation/0"));

    editor.klicka_rubrik("insp/Transform");
    assert!(
        editor.layout().rect("f/Transform/translation/0").is_none(),
        "innehållet ska vara borta ur layouten"
    );

    // Och det ska stanna så, trots att trädet byggs om varje frame.
    for _ in 0..10 {
        editor.frame(Input::default());
    }
    assert!(editor.layout().rect("f/Transform/translation/0").is_none());
}

#[test]
fn en_komponent_gar_att_lagga_till_och_ta_bort() {
    let mut editor = Editor::start();
    editor.markera_kuben();
    let entity = editor.kuben();

    assert!(editor.world.get::<ymer_core::Camera>(entity).is_none());
    assert!(editor.finns("add/Camera"), "knappen ska finnas");

    editor.klicka("add/Camera");
    assert!(editor.world.get::<ymer_core::Camera>(entity).is_some());
    assert!(
        !editor.finns("add/Camera"),
        "knappen ska försvinna när komponenten finns"
    );

    editor.klicka("insp/Camera/remove");
    assert!(editor.world.get::<ymer_core::Camera>(entity).is_none());
}

#[test]
fn knapparna_i_topplisten_gor_nagot() {
    let mut editor = Editor::start();
    let fore = editor.antal_entiteter();

    editor.klicka("tb/spawn");
    assert_eq!(editor.antal_entiteter(), fore + 1);
    assert!(editor.state.selected.is_some(), "den nya blir markerad");

    editor.klicka("tb/del");
    assert_eq!(editor.antal_entiteter(), fore);
    assert!(editor.state.selected.is_none());
}

#[test]
fn spela_knappen_vaxlar() {
    let mut editor = Editor::start();
    assert!(!editor.state.playing);
    editor.klicka("tb/play");
    assert!(editor.state.playing);
    editor.klicka("tb/play");
    assert!(!editor.state.playing);
}

#[test]
fn panelerna_gar_att_dra_isar() {
    let mut editor = Editor::start();
    let fore = editor.rect("split/left").x;

    editor.dra("split/left", Vec2::new(60.0, 0.0));

    let efter = editor.rect("split/left").x;
    assert!(
        (efter - fore - 60.0).abs() < 1.0,
        "avdelaren flyttade sig {} px",
        efter - fore
    );
    // Och scenens yta krympte lika mycket.
    assert!(editor.chrome.viewport(FONSTER).x > fore);
}

#[test]
fn hogerpanelen_dras_at_ratt_hall() {
    // Avdelaren för en panel i högerkanten mäter sin bredd från andra
    // hållet: dras den åt höger ska panelen bli smalare, inte bredare.
    let mut editor = Editor::start();
    let fore = editor.chrome.viewport(FONSTER).width;

    editor.dra("split/right", Vec2::new(40.0, 0.0));

    let efter = editor.chrome.viewport(FONSTER).width;
    assert!(
        efter > fore,
        "scenen ska bli bredare när högerpanelen krymper: {fore} -> {efter}"
    );
}

#[test]
fn hierarkin_gar_att_rulla() {
    let mut editor = Editor::start();
    for i in 0..60 {
        editor.world.spawn((
            EntityName::new(format!("Kub {i}")),
            Transform::IDENTITY,
            MeshInstance::new(ymer_core::BUILTIN_CUBE, Color::WHITE),
        ));
    }
    editor.frame(Input::default());

    let forsta = editor.kuben();
    let fore = editor.rect(&format!("hier/{}", forsta.to_bits())).y;

    let punkt = editor.rect("hier/scroll").center();
    editor.frame(Input::from_pointer(Pointer {
        position: punkt,
        scroll: Vec2::new(0.0, 80.0),
        ..Default::default()
    }));

    let efter = editor.rect(&format!("hier/{}", forsta.to_bits())).y;
    assert_eq!(efter, fore - 80.0, "listan ska ha rullat");

    // Och rullningen ska överleva att trädet byggs om.
    editor.frame(Input::default());
    assert_eq!(editor.rect(&format!("hier/{}", forsta.to_bits())).y, efter);
}

#[test]
fn pekaren_over_en_panel_tillhor_inte_scenen() {
    let mut editor = Editor::start();

    let over_panel = editor.rect("hier/scroll").center();
    let frame = editor.frame(Input::from_pointer(Pointer {
        position: over_panel,
        ..Default::default()
    }));
    assert!(frame.pointer_over_ui);

    let i_scenen = editor.chrome.viewport(FONSTER).center();
    let frame = editor.frame(Input::from_pointer(Pointer {
        position: i_scenen,
        ..Default::default()
    }));
    assert!(
        !frame.pointer_over_ui,
        "hålet i mitten ska släppa igenom klick till scenen"
    );
}

#[test]
fn scenens_yta_ligger_mellan_panelerna() {
    let editor = Editor::start();
    let viewport = editor.chrome.viewport(FONSTER);

    let vanster = editor.rect("split/left");
    let hoger = editor.rect("split/right");

    assert!(viewport.x >= vanster.right() - 1.0);
    assert!(viewport.right() <= hoger.x + 1.0);
    assert!(viewport.height > 0.0 && viewport.width > 0.0);
}

#[test]
fn ritlistan_ar_inte_tom() {
    let mut editor = Editor::start();
    editor.markera_kuben();
    let frame = editor.frame(Input::default());
    assert!(
        frame.list.len() > 20,
        "en hel editor ska ge fler än {} kommandon",
        frame.list.len()
    );
}

#[test]
fn ett_falt_gar_att_skriva_i() {
    let mut editor = Editor::start();
    editor.markera_kuben();

    editor.klicka("f/Transform/scale/0");
    editor.frame(Input {
        text: "3".to_string(),
        ..Default::default()
    });
    editor.frame(Input {
        keys: vec![ymer_ui::Key::Enter],
        ..Default::default()
    });

    assert!((editor.transform().scale.x - 3.0).abs() < 1e-5);
}

#[test]
fn namnet_gar_att_andra() {
    let mut editor = Editor::start();
    editor.markera_kuben();

    // Klicka i namnfältet, radera och skriv nytt.
    editor.klicka("f/Name");
    for _ in 0..8 {
        editor.frame(Input {
            keys: vec![ymer_ui::Key::Backspace],
            ..Default::default()
        });
    }
    editor.frame(Input {
        text: "Hus".to_string(),
        ..Default::default()
    });

    let entity = editor.kuben();
    assert_eq!(
        editor.world.get::<EntityName>(entity).map(|n| n.0.clone()),
        Some("Hus".to_string())
    );
}

#[test]
fn ett_dokument_utan_markering_bygger_anda() {
    // Regressionsskydd: tidiga versioner byggde inspektorn även utan
    // markering och slog upp en entitet som inte fanns.
    let mut editor = Editor::start();
    editor.state.selected = Some(Entity::from_raw_u32(9999).unwrap());
    let frame = editor.frame(Input::default());
    assert!(!frame.list.is_empty());
}

#[test]
fn state_ar_kvar_mellan_frames() {
    // Hovring är tillstånd i `State`, inte i dokumentet, och ska överleva
    // att trädet byggs om.
    let mut editor = Editor::start();
    let punkt = editor.rect("tb/save").center();
    editor.frame(Input::from_pointer(Pointer {
        position: punkt,
        ..Default::default()
    }));
    assert_eq!(editor.chrome.state().hovered.as_deref(), Some("tb/save"));

    let _: &State = editor.chrome.state();
}
