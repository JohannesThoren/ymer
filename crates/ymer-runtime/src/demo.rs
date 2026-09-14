//! Tillfällig demo-scen. Försvinner när scener kan laddas från fil
//! (milstolpe 3) – tills dess är den vår enda "innehållspipeline".

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};
use ymer_core::{Camera, Color, EntityName, GlobalTransform, Quat, Time, Transform, Vec3};

/// Roterar en entitet kring en axel. Spelkomponent, inte motorkomponent –
/// men den registreras i typregistret precis som motorns egna och hamnar
/// därför i scenfiler och i editorns inspector utan särbehandling.
#[derive(Component, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Spin {
    pub axis: Vec3,
    pub radians_per_second: f32,
}

impl Default for Spin {
    fn default() -> Self {
        // Nollvektor skulle ge NaN i normalize().
        Self {
            axis: Vec3::Y,
            radians_per_second: 0.0,
        }
    }
}

pub fn spin_system(time: Res<Time>, mut query: Query<(&mut Transform, &Spin)>) {
    let dt = time.delta_seconds();
    for (mut transform, spin) in &mut query {
        let delta = Quat::from_axis_angle(spin.axis.normalize(), spin.radians_per_second * dt);
        transform.rotation = (delta * transform.rotation).normalize();
    }
}

pub fn setup(world: &mut World) {
    // Assetnamn, inga handtag: scenen bryr sig inte om laddordning.
    let cube = ymer_core::BUILTIN_CUBE;
    let ground = ymer_core::BUILTIN_PLANE;

    world.spawn((
        EntityName::new("Main Camera"),
        Transform::from_xyz(0.0, 5.0, 11.0).looking_at(Vec3::new(0.0, 0.8, 0.0), Vec3::Y),
        GlobalTransform::default(),
        Camera::default(),
    ));

    world.spawn((
        EntityName::new("Ground"),
        Transform::from_xyz(0.0, -0.75, 0.0),
        GlobalTransform::default(),
        ymer_core::MeshInstance::new(ground, Color::rgb(0.05, 0.055, 0.075)),
    ));

    // Rutnät av kuber som roterar i olika takt.
    for x in -2..=2 {
        for z in -2..=2 {
            let t = (x + 2) as f32 * 0.2 + (z + 2) as f32 * 0.06;
            let color = Color::rgb(0.25 + 0.6 * t, 0.35, 0.95 - 0.5 * t);

            world.spawn((
                EntityName::new(format!("Cube {x},{z}")),
                Transform::from_xyz(x as f32 * 2.2, 0.0, z as f32 * 2.2),
                GlobalTransform::default(),
                ymer_core::MeshInstance::new(cube, color),
                Spin {
                    axis: Vec3::new(0.3, 1.0, 0.15),
                    radians_per_second: 0.5 + t * 0.9,
                },
            ));
        }
    }

    // Hierarki-bevis: en osynlig snurrande nav med tre barn som får
    // sin världsposition enbart genom förälderns rotation.
    let hub = world
        .spawn((
            EntityName::new("Orbit Hub"),
            Transform::from_xyz(0.0, 3.2, 0.0),
            GlobalTransform::default(),
            Spin {
                axis: Vec3::Y,
                radians_per_second: 1.1,
            },
        ))
        .id();

    for (index, color) in [
        Color::rgb(0.95, 0.35, 0.45),
        Color::rgb(0.4, 0.9, 0.55),
        Color::rgb(0.95, 0.8, 0.3),
    ]
    .into_iter()
    .enumerate()
    {
        let angle = index as f32 * std::f32::consts::TAU / 3.0;
        let offset = Vec3::new(angle.cos() * 3.0, 0.0, angle.sin() * 3.0);

        world.spawn((
            EntityName::new(format!("Orbiter {index}")),
            Transform {
                translation: offset,
                ..Transform::IDENTITY
            }
            .with_scale(Vec3::splat(0.55)),
            GlobalTransform::default(),
            ymer_core::MeshInstance::new(cube, color),
            Spin {
                axis: Vec3::new(1.0, 0.4, 0.0),
                radians_per_second: 2.2,
            },
            ChildOf(hub),
        ));
    }
}
