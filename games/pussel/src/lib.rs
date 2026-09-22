//! Ett litet skjutpussel, byggt på motorn för att se var den skaver.
//!
//! Banan är ett rutnät i xz-planet: spelaren skjuter lådor mot mål, en ruta
//! i taget, och en dörr öppnas av en tryckplatta. Reglerna bor i
//! `scripts/pussel.ts` – den här filen bygger bara banan.
//!
//! Grafiken är Kenneys "Mini Dungeon" (CC0), se `models/KENNEY-LICENSE.txt`.
//! Modellerna är gjorda för exakt det här: en ruta är en enhet, och allt
//! har sitt ursprung i golvhöjd, så inget behöver skalas eller lyftas.

use bevy_ecs::hierarchy::ChildOf;
use ymer_core::{Color, EntityName, GlobalTransform, MeshInstance, Quat, Script, Transform, Vec3};
use ymer_scene::{Scene, TypeRegistry};

pub const CELL: f32 = 1.0;
pub const SCRIPT: &str = "pussel.ts";

/// Assetnamnen modellerna får vid import. Motorn namnger en mesh efter
/// filens sökväg relativt projektroten plus meshens namn i filen, så de
/// här strängarna måste matcha vad `gltf_probe` skriver ut.
///
/// Varje modell får också ett eget texturnamn, trots att alla sju delar
/// samma `colormap.png`: importen laddar upp bilder per fil, så samma
/// palett hamnar en gång per modell i registret.
/// Namnen kommer ur *meshen* i filen, inte ur filnamnet, och de två är
/// inte alltid samma: `gate.glb` innehåller en mesh som heter `door`. Ett
/// namn som inte finns i registret faller tyst tillbaka på inbyggda kuben,
/// så de här strängarna är verifierade med `gltf_probe`.
pub mod asset {
    /// (mesh, textur)
    pub const WALL: (&str, &str) = ("models/wall.glb#wall", "models/wall.glb#tex0");
    pub const FLOOR: (&str, &str) = ("models/floor.glb#floor", "models/floor.glb#tex0");
    pub const BARREL: (&str, &str) = ("models/barrel.glb#barrel", "models/barrel.glb#tex0");
    pub const GATE: (&str, &str) = ("models/gate.glb#door", "models/gate.glb#tex0");
    pub const COIN: (&str, &str) = ("models/coin.glb#coin", "models/coin.glb#tex0");
    pub const TRAP: (&str, &str) = ("models/trap.glb#trap", "models/trap.glb#tex0");
    /// Figuren är riggad: skelettet bär ingen geometri, och kropp och
    /// huvud är två separata meshar i bindpose. Motorn har ingen skinning,
    /// så de sätts ihop med transformhierarkin i stället – huvudet blir
    /// barn till kroppen och följer med när spelaren flyttar sig.
    pub const PLAYER_BODY: (&str, &str) = (
        "models/character-human.glb#body-mesh",
        "models/character-human.glb#tex0",
    );
    pub const PLAYER_HEAD: (&str, &str) = (
        "models/character-human.glb#head-mesh",
        "models/character-human.glb#tex0",
    );
}

/// Banan som text. `#` vägg, `.` golv, `@` spelare, `$` låda, `*` mål,
/// `P` tryckplatta, `D` dörr.
///
/// Dörren är stängd tills något står på plattan. Lådan som håller den
/// öppen kan inte användas till målet – det är hela pusslet.
pub const LEVEL: &[&str] = &[
    "#########",
    "#.@.$.P.#",
    "#.......#",
    "#.$...D*#",
    "#########",
];

/// Vad en ruta i banan ska bli.
struct Piece {
    /// (mesh, textur) ur `asset`.
    asset: (&'static str, &'static str),
    /// Vitt låter texturen tala för sig själv; `MeshInstance.color` tonar
    /// den, den ersätter den inte.
    tint: Color,
}

/// Bygger banan i en värld och returnerar antalet entiteter.
pub fn build(world: &mut bevy_ecs::world::World) -> usize {
    let mut count = 0;
    let mut boxes = 0;
    let mut goals = 0;
    let mut plates = 0;
    let mut doors = 0;

    for (row, line) in LEVEL.iter().enumerate() {
        for (column, tile) in line.chars().enumerate() {
            let x = column as f32 * CELL;
            let z = row as f32 * CELL;

            if tile == '#' {
                spawn(
                    world,
                    format!("Vägg {row}-{column}"),
                    [x, 0.0, z],
                    Piece {
                        asset: asset::WALL,
                        tint: Color::WHITE,
                    },
                    None,
                    None,
                );
                count += 1;
                continue;
            }

            // Golv under allt som inte är vägg.
            spawn(
                world,
                format!("Golv {row}-{column}"),
                [x, 0.0, z],
                Piece {
                    asset: asset::FLOOR,
                    tint: Color::WHITE,
                },
                None,
                None,
            );
            count += 1;

            match tile {
                '@' => {
                    let body = spawn(
                        world,
                        "Spelare".to_string(),
                        [x, 0.0, z],
                        Piece {
                            asset: asset::PLAYER_BODY,
                            tint: Color::WHITE,
                        },
                        Some(SCRIPT),
                        None,
                    );
                    // Huvudet är en egen mesh i samma bindpose, så det
                    // hamnar rätt med identitetstransform – men bara om
                    // det är barn till kroppen, annars står det kvar i
                    // origo när spelaren går.
                    let head = spawn(
                        world,
                        "Spelare huvud".to_string(),
                        [0.0, 0.0, 0.0],
                        Piece {
                            asset: asset::PLAYER_HEAD,
                            tint: Color::WHITE,
                        },
                        None,
                        None,
                    );
                    world.entity_mut(head).insert(ChildOf(body));
                    count += 2;
                }
                '$' => {
                    boxes += 1;
                    spawn(
                        world,
                        format!("Låda {boxes}"),
                        [x, 0.0, z],
                        Piece {
                            asset: asset::BARREL,
                            tint: Color::WHITE,
                        },
                        None,
                        None,
                    );
                    count += 1;
                }
                'P' => {
                    plates += 1;
                    spawn(
                        world,
                        format!("Platta {plates}"),
                        [x, 0.0, z],
                        Piece {
                            asset: asset::TRAP,
                            tint: Color::WHITE,
                        },
                        None,
                        None,
                    );
                    count += 1;
                }
                'D' => {
                    doors += 1;
                    // Grinden är tunn i z och står som en vägg i en
                    // öppning; den här dörren spärrar färd i x-led, så
                    // den vrids ett kvarts varv.
                    spawn(
                        world,
                        format!("Dörr {doors}"),
                        [x, 0.0, z],
                        Piece {
                            asset: asset::GATE,
                            tint: Color::WHITE,
                        },
                        None,
                        Some(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
                    );
                    count += 1;
                }
                '*' => {
                    goals += 1;
                    spawn(
                        world,
                        format!("Mål {goals}"),
                        [x, 0.0, z],
                        Piece {
                            asset: asset::COIN,
                            tint: Color::WHITE,
                        },
                        None,
                        None,
                    );
                    count += 1;
                }
                _ => {}
            }
        }
    }
    count
}

fn spawn(
    world: &mut bevy_ecs::world::World,
    name: String,
    position: [f32; 3],
    piece: Piece,
    script: Option<&str>,
    rotation: Option<Quat>,
) -> bevy_ecs::entity::Entity {
    let mut transform = Transform::from_xyz(position[0], position[1], position[2]);
    transform.scale = Vec3::ONE;
    if let Some(rotation) = rotation {
        transform.rotation = rotation;
    }

    let mut entity = world.spawn((
        EntityName::new(name),
        transform,
        GlobalTransform::default(),
        MeshInstance {
            mesh: piece.asset.0.to_string(),
            texture: piece.asset.1.to_string(),
            color: piece.tint,
        },
    ));
    if let Some(script) = script {
        entity.insert(Script::new(script));
    }
    entity.id()
}

/// Banan som en scenfil, så att editorn kan öppna den som vilket projekt
/// som helst.
pub fn scene(registry: &TypeRegistry) -> anyhow::Result<Scene> {
    let mut world = bevy_ecs::world::World::new();
    build(&mut world);
    Ok(Scene::from_world(&mut world, registry))
}
