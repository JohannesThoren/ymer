//! Gizmos: plocka entiteter med musen och dra dem längs en axel.
//!
//! Allt räknas i världsrymd mot AABB:er. Ingen rotation tas med i
//! träffytorna, vilket är osant för roterade objekt men fullt användbart.

use bevy_ecs::prelude::*;
use ymer_core::{Camera, Color, GlobalTransform, Mat4, MeshInstance, Transform, Vec3};
use ymer_render::{Assets, DrawItem, MeshRegistry};

/// Gizmots armlängd i skärmandelar – handtaget håller samma storlek
/// oavsett hur långt bort objektet ligger.
const HANDLE_SCREEN_SIZE: f32 = 0.16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
    Z,
}

impl Axis {
    pub fn direction(self) -> Vec3 {
        match self {
            Axis::X => Vec3::X,
            Axis::Y => Vec3::Y,
            Axis::Z => Vec3::Z,
        }
    }

    fn color(self) -> Color {
        match self {
            Axis::X => Color::rgb(0.95, 0.25, 0.3),
            Axis::Y => Color::rgb(0.35, 0.9, 0.4),
            Axis::Z => Color::rgb(0.3, 0.5, 1.0),
        }
    }
}

/// En stråle i världsrymd.
#[derive(Debug, Clone, Copy)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
}

/// Pågående dragning. Offseten sparas så att objektet inte hoppar till
/// musen när man greppar handtaget en bit från centrum.
#[derive(Debug, Clone, Copy)]
pub struct Drag {
    pub entity: Entity,
    pub axis: Axis,
    pub offset: f32,
}

/// Bygger en stråle från kameran genom en pixel.
pub fn screen_ray(view_proj: Mat4, mouse: (f32, f32), size: (u32, u32)) -> Ray {
    let ndc_x = 2.0 * mouse.0 / size.0.max(1) as f32 - 1.0;
    let ndc_y = 1.0 - 2.0 * mouse.1 / size.1.max(1) as f32;

    let inverse = view_proj.inverse();
    // wgpu har djup 0 vid near-planet och 1 vid far.
    let near = inverse * glam_vec4(ndc_x, ndc_y, 0.0);
    let far = inverse * glam_vec4(ndc_x, ndc_y, 1.0);

    let near = near.truncate() / near.w;
    let far = far.truncate() / far.w;

    Ray {
        origin: near,
        direction: (far - near).normalize_or_zero(),
    }
}

fn glam_vec4(x: f32, y: f32, z: f32) -> ymer_core::Vec4 {
    ymer_core::Vec4::new(x, y, z, 1.0)
}

/// Kamerans view_proj för en given bildkvot, tagen från första kameran.
pub fn camera_view_proj(world: &mut World, aspect: f32) -> Option<Mat4> {
    let mut query = world.query::<(&Camera, &GlobalTransform)>();
    let (camera, global) = query.iter(world).next()?;
    Some(camera.projection(aspect) * global.0.inverse())
}

/// Slab-test mot en axelinriktad låda. Returnerar avståndet till träffen.
fn ray_box(ray: &Ray, center: Vec3, half: Vec3) -> Option<f32> {
    let mut near: f32 = 0.0;
    let mut far = f32::INFINITY;

    for axis in 0..3 {
        let direction = ray.direction[axis];
        let origin = ray.origin[axis];
        let min = center[axis] - half[axis];
        let max = center[axis] + half[axis];

        if direction.abs() < 1e-8 {
            if origin < min || origin > max {
                return None;
            }
            continue;
        }

        let mut t0 = (min - origin) / direction;
        let mut t1 = (max - origin) / direction;
        if t0 > t1 {
            std::mem::swap(&mut t0, &mut t1);
        }
        near = near.max(t0);
        far = far.min(t1);
        if near > far {
            return None;
        }
    }
    Some(near)
}

/// Närmaste ritbara entitet längs strålen.
pub fn pick(
    world: &mut World,
    meshes: &MeshRegistry,
    assets: &Assets,
    ray: &Ray,
) -> Option<Entity> {
    let mut best: Option<(f32, Entity)> = None;

    let mut query = world.query::<(Entity, &MeshInstance, &GlobalTransform)>();
    for (entity, instance, global) in query.iter(world) {
        // Namnet slås upp här också – plockytan ska matcha det som ritas.
        let Some(half) = meshes.half_extents(assets.mesh(&instance.mesh)) else {
            continue;
        };
        let (scale, _, translation) = global.0.to_scale_rotation_translation();
        let Some(distance) = ray_box(ray, translation, half * scale) else {
            continue;
        };
        if best.is_none_or(|(closest, _)| distance < closest) {
            best = Some((distance, entity));
        }
    }

    best.map(|(_, entity)| entity)
}

/// Handtagens storlek beror på avståndet till kameran.
fn handle_scale(position: Vec3, camera: Vec3) -> f32 {
    (position - camera).length().max(0.5) * HANDLE_SCREEN_SIZE
}

/// Träffar strålen ett av gizmots handtag?
pub fn pick_axis(ray: &Ray, position: Vec3, camera: Vec3) -> Option<Axis> {
    let scale = handle_scale(position, camera);
    let mut best: Option<(f32, Axis)> = None;

    for axis in [Axis::X, Axis::Y, Axis::Z] {
        let direction = axis.direction();
        let center = position + direction * scale * 0.6;
        // Tjock låda längs axeln: lätt att träffa, svår att missa.
        let half = direction * scale * 0.6 + (Vec3::ONE - direction) * scale * 0.12;

        if let Some(distance) = ray_box(ray, center, half)
            && best.is_none_or(|(closest, _)| distance < closest)
        {
            best = Some((distance, axis));
        }
    }

    best.map(|(_, axis)| axis)
}

/// Punkten på axeln som ligger närmast strålen. Grunden för dragning:
/// två linjer i rymden, närmaste punkt på den ena.
fn closest_point_on_axis(ray: &Ray, origin: Vec3, axis: Vec3) -> Option<f32> {
    let cross = axis.cross(ray.direction);
    let denominator = cross.length_squared();
    if denominator < 1e-8 {
        // Strålen är parallell med axeln – ingen vettig projektion.
        return None;
    }
    let delta = ray.origin - origin;
    Some(delta.cross(ray.direction).dot(cross) / denominator)
}

/// Startar en dragning om strålen träffar ett handtag.
pub fn begin_drag(world: &mut World, entity: Entity, ray: &Ray, camera: Vec3) -> Option<Drag> {
    let position = world.get::<GlobalTransform>(entity)?.translation();
    let axis = pick_axis(ray, position, camera)?;
    let along = closest_point_on_axis(ray, position, axis.direction())?;
    Some(Drag {
        entity,
        axis,
        offset: along,
    })
}

/// Flyttar entiteten så att greppunkten följer musen.
pub fn update_drag(world: &mut World, drag: &Drag, ray: &Ray) -> Option<Vec3> {
    let position = world.get::<GlobalTransform>(drag.entity)?.translation();
    let along = closest_point_on_axis(ray, position, drag.axis.direction())?;
    let delta = drag.axis.direction() * (along - drag.offset);

    let mut transform = world.get_mut::<Transform>(drag.entity)?;
    transform.translation += delta;
    Some(transform.translation)
}

/// Ritobjekt för gizmot: tre armar plus en kub i mitten.
/// Ritas i overlay-passet, så de syns genom geometri.
pub fn gizmo_items(
    world: &mut World,
    entity: Entity,
    camera: Vec3,
    cube: ymer_core::MeshId,
    active: Option<Axis>,
) -> Vec<DrawItem> {
    let Some(global) = world.get::<GlobalTransform>(entity) else {
        return Vec::new();
    };
    let position = global.translation();
    let scale = handle_scale(position, camera);

    let mut items = Vec::with_capacity(4);

    items.push(DrawItem::new(
        cube,
        ymer_core::TextureId(0),
        Mat4::from_scale_rotation_translation(
            Vec3::splat(scale * 0.14),
            ymer_core::Quat::IDENTITY,
            position,
        ),
        Color::rgb(0.9, 0.9, 0.95),
    ));

    for axis in [Axis::X, Axis::Y, Axis::Z] {
        let direction = axis.direction();
        let arm = direction * scale * 0.6 + (Vec3::ONE - direction) * scale * 0.07;
        let mut color = axis.color();
        if active == Some(axis) {
            // Den axel man drar i lyser upp.
            color = Color::rgb(1.0, 0.95, 0.4);
        }

        items.push(DrawItem::new(
            cube,
            ymer_core::TextureId(0),
            Mat4::from_scale_rotation_translation(
                arm * 2.0,
                ymer_core::Quat::IDENTITY,
                position + direction * scale * 0.6,
            ),
            color,
        ));
    }

    items
}
