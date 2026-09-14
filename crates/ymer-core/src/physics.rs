//! Fysik: gravitation, hastighet och kollisionsupplösning.
//!
//! Medvetet litet och deterministiskt. Svept AABB per axel täcker
//! plattformsspel, top-down och arkad – alltså det mesta man bygger i en
//! motor som den här – utan att införa en andra sanning om var saker
//! befinner sig.
//!
//! Determinismen är inte gratis pynt: motorns starkaste testverktyg är att
//! rendera en frame och jämföra mot en referensbild. En fysikmotor med
//! egen intern värld och egna tidssteg hade gjort det opålitligt.
//!
//! Upplösningen sker en axel i taget. Det är den klassiska metoden för
//! plattformsspel: den tillåter att man glider längs en vägg i stället för
//! att fastna, och den gör "står på marken" till en enkel fråga i stället
//! för en kontaktnormalanalys.

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{GlobalTransform, Time, Transform, Vec3};

/// Världens gravitation. Byt ut resursen för att ändra den.
#[derive(Resource, Debug, Clone, Copy)]
pub struct Gravity(pub Vec3);

impl Default for Gravity {
    fn default() -> Self {
        // -9.82 är jordens, men spel mår nästan alltid bättre av mer.
        Self(Vec3::new(0.0, -20.0, 0.0))
    }
}

/// Axelinriktad låda kring entitetens origo.
#[derive(Component, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Collider {
    /// Halva storleken i världsenheter.
    pub half_extents: Vec3,
    /// Förskjutning från entitetens origo, för fall där lådan inte är
    /// centrerad kring pivoten.
    #[serde(default)]
    pub offset: Vec3,
}

impl Default for Collider {
    fn default() -> Self {
        Self {
            half_extents: Vec3::splat(0.5),
            offset: Vec3::ZERO,
        }
    }
}

impl Collider {
    pub fn cube(half: f32) -> Self {
        Self {
            half_extents: Vec3::splat(half),
            offset: Vec3::ZERO,
        }
    }

    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self {
            half_extents: Vec3::new(x, y, z),
            offset: Vec3::ZERO,
        }
    }

    fn bounds(&self, position: Vec3) -> (Vec3, Vec3) {
        let center = position + self.offset;
        (center - self.half_extents, center + self.half_extents)
    }
}

/// En kropp som rör sig och stoppas av kolliderare.
///
/// Entiteter med `Collider` men utan `RigidBody` är statisk geometri –
/// mark, väggar, plattformar. De flyttas aldrig av fysiken.
#[derive(Component, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RigidBody {
    pub velocity: Vec3,
    /// Multiplikator på världens gravitation. 0 ger en svävande kropp.
    #[serde(default = "one_f32")]
    pub gravity_scale: f32,
    /// Sätts av fysiken varje frame: står kroppen på något?
    /// Skrivs över varje steg, så att sätta den själv gör ingenting.
    #[serde(default)]
    pub grounded: bool,
}

fn one_f32() -> f32 {
    1.0
}

impl Default for RigidBody {
    fn default() -> Self {
        Self {
            velocity: Vec3::ZERO,
            gravity_scale: 1.0,
            grounded: false,
        }
    }
}

impl RigidBody {
    pub fn new() -> Self {
        Self::default()
    }

    /// Kropp som inte påverkas av gravitation – top-down-spel, projektiler.
    pub fn floating() -> Self {
        Self {
            gravity_scale: 0.0,
            ..Self::default()
        }
    }
}

fn overlaps(a: (Vec3, Vec3), b: (Vec3, Vec3)) -> bool {
    a.0.x < b.1.x
        && a.1.x > b.0.x
        && a.0.y < b.1.y
        && a.1.y > b.0.y
        && a.0.z < b.1.z
        && a.1.z > b.0.z
}

/// Flyttar kroppar, löser kollisioner och sätter `grounded`.
///
/// Kör efter spelets egna system men före `propagate_transforms`, så att
/// hierarkin räknas om med de slutliga positionerna.
pub fn step_physics(
    time: Res<Time>,
    gravity: Option<Res<Gravity>>,
    mut bodies: Query<(Entity, &mut Transform, &Collider, &mut RigidBody)>,
    statics: Query<(Entity, &GlobalTransform, &Collider), Without<RigidBody>>,
) {
    let dt = time.delta_seconds();
    if dt <= 0.0 {
        return;
    }
    let gravity = gravity.map(|g| g.0).unwrap_or(Gravity::default().0);

    // Statisk geometri samlas en gång; den rör sig inte under steget.
    let obstacles: Vec<(Vec3, Vec3)> = statics
        .iter()
        .map(|(_, global, collider)| collider.bounds(global.translation()))
        .collect();

    for (_, mut transform, collider, mut body) in &mut bodies {
        let scale = body.gravity_scale;
        body.velocity += gravity * scale * dt;

        let mut position = transform.translation;
        let motion = body.velocity * dt;
        let mut grounded = false;

        // En axel i taget: tillåter att man glider längs en vägg i stället
        // för att fastna i den.
        for axis in 0..3 {
            if motion[axis] == 0.0 {
                continue;
            }

            let mut candidate = position;
            candidate[axis] += motion[axis];

            let moved = collider.bounds(candidate);
            let hit = obstacles.iter().any(|obstacle| overlaps(moved, *obstacle));

            if !hit {
                position = candidate;
                continue;
            }

            // Backa till kant mot kant i stället för att bara avbryta,
            // annars blir det ett synligt glapp mot marken.
            let mut closest = position[axis];
            for obstacle in &obstacles {
                let still = collider.bounds(position);
                // Bara hinder som faktiskt är i vägen längs den här axeln.
                let blocking = (0..3).all(|other| {
                    other == axis
                        || (still.0[other] < obstacle.1[other]
                            && still.1[other] > obstacle.0[other])
                });
                if !blocking {
                    continue;
                }

                if motion[axis] > 0.0 {
                    let limit =
                        obstacle.0[axis] - collider.half_extents[axis] - collider.offset[axis];
                    if limit >= position[axis] {
                        closest = closest.min(limit).max(position[axis]);
                    }
                } else {
                    let limit =
                        obstacle.1[axis] + collider.half_extents[axis] - collider.offset[axis];
                    if limit <= position[axis] {
                        closest = closest.max(limit).min(position[axis]);
                    }
                }
            }
            position[axis] = closest;

            // Stoppad längs en axel betyder att farten längs den är slut.
            body.velocity[axis] = 0.0;
            if axis == 1 && motion[axis] < 0.0 {
                grounded = true;
            }
        }

        transform.translation = position;
        body.grounded = grounded;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// Bygger en värld med mark vid y=0 (ovansida) och en kropp ovanför.
    fn world_with_ground() -> World {
        let mut world = World::new();
        let mut time = Time::new();
        time.advance_by(Duration::from_millis(16));
        world.insert_resource(time);
        world.insert_resource(Gravity::default());

        // Mark: topp vid y = 0.
        world.spawn((
            Transform::from_xyz(0.0, -1.0, 0.0),
            GlobalTransform(Transform::from_xyz(0.0, -1.0, 0.0).matrix()),
            Collider::new(50.0, 1.0, 50.0),
        ));
        world
    }

    fn run(world: &mut World, steps: usize) {
        let mut schedule = Schedule::default();
        schedule.add_systems(step_physics);
        for _ in 0..steps {
            world
                .resource_mut::<Time>()
                .advance_by(Duration::from_millis(16));
            schedule.run(world);
        }
    }

    #[test]
    fn kropp_faller_och_landar_pa_marken() {
        let mut world = world_with_ground();
        let body = world
            .spawn((
                Transform::from_xyz(0.0, 5.0, 0.0),
                GlobalTransform::default(),
                Collider::cube(0.5),
                RigidBody::new(),
            ))
            .id();

        run(&mut world, 120);

        let transform = world.get::<Transform>(body).unwrap();
        // Kroppens underkant ska vila mot markens ovansida: y = 0 + 0.5.
        assert!(
            (transform.translation.y - 0.5).abs() < 0.01,
            "landade på {} i stället för 0.5",
            transform.translation.y
        );
        assert!(
            world.get::<RigidBody>(body).unwrap().grounded,
            "grounded sattes inte"
        );
    }

    #[test]
    fn vagg_stoppar_horisontell_rorelse_men_inte_fall() {
        let mut world = world_with_ground();
        // Vägg med insida vid x = 2.
        world.spawn((
            Transform::from_xyz(3.0, 5.0, 0.0),
            GlobalTransform(Transform::from_xyz(3.0, 5.0, 0.0).matrix()),
            Collider::new(1.0, 10.0, 50.0),
        ));

        let body = world
            .spawn((
                Transform::from_xyz(0.0, 2.0, 0.0),
                GlobalTransform::default(),
                Collider::cube(0.5),
                RigidBody {
                    velocity: Vec3::new(10.0, 0.0, 0.0),
                    ..RigidBody::new()
                },
            ))
            .id();

        run(&mut world, 60);

        let transform = world.get::<Transform>(body).unwrap();
        assert!(
            transform.translation.x <= 1.51,
            "gick igenom väggen, x = {}",
            transform.translation.x
        );
        // Att stoppas i sidled får inte hindra fallet.
        assert!(
            (transform.translation.y - 0.5).abs() < 0.01,
            "föll inte ner till marken, y = {}",
            transform.translation.y
        );
    }

    #[test]
    fn svavande_kropp_paverkas_inte_av_gravitation() {
        let mut world = world_with_ground();
        let body = world
            .spawn((
                Transform::from_xyz(0.0, 5.0, 0.0),
                GlobalTransform::default(),
                Collider::cube(0.5),
                RigidBody::floating(),
            ))
            .id();

        run(&mut world, 60);

        let y = world.get::<Transform>(body).unwrap().translation.y;
        assert!((y - 5.0).abs() < 0.001, "svävande kropp rörde sig till {y}");
    }

    #[test]
    fn statisk_geometri_flyttas_aldrig() {
        let mut world = world_with_ground();
        let wall = world
            .spawn((
                Transform::from_xyz(3.0, 5.0, 0.0),
                GlobalTransform(Transform::from_xyz(3.0, 5.0, 0.0).matrix()),
                Collider::cube(1.0),
            ))
            .id();

        run(&mut world, 30);

        let transform = world.get::<Transform>(wall).unwrap();
        assert_eq!(transform.translation, Vec3::new(3.0, 5.0, 0.0));
    }
}
