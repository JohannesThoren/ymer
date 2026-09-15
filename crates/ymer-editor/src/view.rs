//! Editorns vykamera.
//!
//! Medvetet *inte* en entitet i världen. Scenens kamera är speldata som
//! sparas i scenfilen; att rotera vyn får inte ändra den. Den här kameran
//! lever bara i editorn och matas in i renderingen som en
//! view-projektionsmatris, så att världen förblir orörd.

use ymer_core::{Mat4, Vec3};

/// Orbitkamera: tittar alltid mot `focus`, på `distance` avstånd.
pub struct EditorCamera {
    pub focus: Vec3,
    /// Radianer kring världens Y-axel.
    pub yaw: f32,
    /// Radianer över/under horisonten.
    pub pitch: f32,
    pub distance: f32,
    pub fov_y_radians: f32,
}

impl Default for EditorCamera {
    fn default() -> Self {
        Self {
            focus: Vec3::new(0.0, 0.5, 0.0),
            yaw: 0.6,
            pitch: 0.5,
            distance: 12.0,
            fov_y_radians: 60f32.to_radians(),
        }
    }
}

/// Hur nära lodrätt kameran får komma. Exakt rakt uppifrån gör upp-vektorn
/// tvetydig och får vyn att slå runt.
const PITCH_LIMIT: f32 = 1.5533; // ~89 grader

impl EditorCamera {
    pub fn position(&self) -> Vec3 {
        let horizontal = self.distance * self.pitch.cos();
        self.focus
            + Vec3::new(
                horizontal * self.yaw.sin(),
                self.distance * self.pitch.sin(),
                horizontal * self.yaw.cos(),
            )
    }

    pub fn view(&self) -> Mat4 {
        ymer_core::glam::camera::rh::view::look_at_mat4(self.position(), self.focus, Vec3::Y)
    }

    pub fn view_proj(&self, aspect: f32) -> Mat4 {
        let projection = ymer_core::glam::camera::rh::proj::directx::perspective(
            self.fov_y_radians,
            aspect.max(0.0001),
            0.05,
            2000.0,
        );
        projection * self.view()
    }

    /// Högerdrag: snurra kring fokuspunkten.
    pub fn orbit(&mut self, delta_x: f32, delta_y: f32) {
        const SENSITIVITY: f32 = 0.008;
        self.yaw -= delta_x * SENSITIVITY;
        self.pitch = (self.pitch + delta_y * SENSITIVITY).clamp(-PITCH_LIMIT, PITCH_LIMIT);
    }

    /// Mittendrag: flytta fokuspunkten i kamerans eget plan, så att
    /// panoreringen känns likadan oavsett hur vyn är vriden.
    pub fn pan(&mut self, delta_x: f32, delta_y: f32) {
        let forward = (self.focus - self.position()).normalize_or_zero();
        let right = forward.cross(Vec3::Y).normalize_or_zero();
        let up = right.cross(forward).normalize_or_zero();

        // Skala med avståndet: långt bort ska ett musdrag flytta mer.
        let speed = self.distance * 0.0015;
        self.focus += (-right * delta_x + up * delta_y) * speed;
    }

    /// Scroll: multiplikativ zoom, så att stegen känns lika stora nära
    /// som långt bort.
    pub fn zoom(&mut self, scroll: f32) {
        self.distance = (self.distance * (1.0 - scroll * 0.1)).clamp(0.5, 500.0);
    }

    /// Rama in en punkt – kopplat till F-tangenten.
    pub fn focus_on(&mut self, target: Vec3, radius: f32) {
        self.focus = target;
        // Lite marginal så att objektet inte fyller hela bilden.
        self.distance = (radius * 3.0).clamp(1.5, 500.0);
    }
}
