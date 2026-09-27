//! Hur ett JSON-värde ser ut som fält – och hur det hittas igen.
//!
//! Inspektorn bygger noder ur komponenternas JSON och läser sedan
//! tillbaka det användaren ändrade. De två måste vara överens om exakt
//! två saker: vilken *sorts* fält ett värde blir, och vilket *id* fältet
//! får. Båda besluten bor här, i var sin funktion som bygget och
//! avläsningen delar. Ligger de på varsitt håll glider de isär, och
//! symptomet är ett fält som går att ändra men inte sparas.

use serde_json::Value;
use ymer_core::glam::{Quat, Vec3};

/// Vad som finns att välja på i fält som pekar ut en fil.
pub struct Assets<'a> {
    pub scripts: &'a [String],
    pub textures: &'a [String],
    pub meshes: &'a [String],
}

/// Vilken sorts fält ett värde blir.
pub enum Shape<'a> {
    Bool,
    /// Heltal är index och antal – inte 0.00, och inte steg om hundradelar.
    Int,
    Float {
        step: f64,
    },
    Text,
    /// En lista att välja ur i stället för fritext.
    Asset {
        options: &'a [String],
        placeholder: &'static str,
    },
    /// Två till fyra tal med axelnamn.
    Vector {
        axes: &'static [&'static str],
    },
    /// En kvaternion, visad som tre grader. Omöjlig att redigera annars.
    Euler,
    /// Ett objekt med r/g/b/a.
    Color,
    Object,
    Array,
    Null,
}

const AXES: [&str; 4] = ["x", "y", "z", "w"];

/// Beslutet bygget och avläsningen delar.
///
/// `label` är nyckeln värdet ligger under, när det ligger under någon.
/// Den avgör mer än typen gör: ett `rotation` med fyra tal är en
/// kvaternion, fyra tal utan namn är bara fyra tal.
pub fn shape_of<'a>(
    component: &str,
    label: Option<&str>,
    value: &Value,
    assets: &Assets<'a>,
) -> Shape<'a> {
    match value {
        Value::Bool(_) => Shape::Bool,
        Value::Number(number) => {
            if number.is_i64() || number.is_u64() {
                Shape::Int
            } else {
                Shape::Float {
                    step: label.map(step_for).unwrap_or(0.05),
                }
            }
        }
        Value::String(_) => {
            // Script är en newtype och serialiseras som sitt innehåll, så
            // den har ingen nyckel att känna igen den på – komponentens
            // namn är det enda som finns.
            if component == "Script" && label.is_none() {
                return Shape::Asset {
                    options: assets.scripts,
                    placeholder: "– välj skript –",
                };
            }
            match label {
                Some("mesh") => Shape::Asset {
                    options: assets.meshes,
                    placeholder: "– välj mesh –",
                },
                Some("texture") => Shape::Asset {
                    options: assets.textures,
                    placeholder: "– ingen textur –",
                },
                _ => Shape::Text,
            }
        }
        Value::Array(items) => {
            let numeric = items.iter().all(Value::is_number);
            if numeric && items.len() == 4 && label == Some("rotation") {
                Shape::Euler
            } else if numeric && (2..=4).contains(&items.len()) {
                Shape::Vector {
                    axes: &AXES[..items.len()],
                }
            } else {
                Shape::Array
            }
        }
        Value::Object(map) => {
            let keys: Vec<&str> = map.keys().map(String::as_str).collect();
            if keys.len() == 4 && ["a", "b", "g", "r"].iter().all(|key| keys.contains(key)) {
                Shape::Color
            } else {
                Shape::Object
            }
        }
        Value::Null => Shape::Null,
    }
}

/// Fältets id: komponent plus sökvägen ner i värdet.
///
/// `f/Transform/translation/0`. Samma funktion används när noden skapas
/// och när den slås upp, så de kan inte stava olika.
pub fn field_id(component: &str, path: &[String]) -> String {
    let mut id = String::with_capacity(16 + component.len());
    id.push_str("f/");
    id.push_str(component);
    for part in path {
        id.push('/');
        id.push_str(part);
    }
    id
}

/// Steget för ett tal i en vektor. Nyckeln sitter på vektorn, inte på
/// talet, så `translation` ger alla tre axlarna samma steg.
pub fn vector_step(label: &str) -> f64 {
    step_for(label)
}

/// Vinklar i radianer vill ha finare steg än positioner.
fn step_for(label: &str) -> f64 {
    if label.ends_with("_radians") {
        0.01
    } else {
        0.05
    }
}

/// Kvaternion till grader, i den ordning editorn visar dem.
pub fn euler_degrees(items: &[Value]) -> Vec3 {
    let quat = Quat::from_xyzw(
        items.first().and_then(Value::as_f64).unwrap_or(0.0) as f32,
        items.get(1).and_then(Value::as_f64).unwrap_or(0.0) as f32,
        items.get(2).and_then(Value::as_f64).unwrap_or(0.0) as f32,
        items.get(3).and_then(Value::as_f64).unwrap_or(1.0) as f32,
    )
    .normalize();
    let (y, x, z) = quat.to_euler(ymer_core::glam::EulerRot::YXZ);
    Vec3::new(x.to_degrees(), y.to_degrees(), z.to_degrees())
}

/// Grader tillbaka till kvaternion.
pub fn euler_quat(degrees: Vec3) -> [f64; 4] {
    let quat = Quat::from_euler(
        ymer_core::glam::EulerRot::YXZ,
        degrees.y.to_radians(),
        degrees.x.to_radians(),
        degrees.z.to_radians(),
    )
    .normalize();
    [quat.x as f64, quat.y as f64, quat.z as f64, quat.w as f64]
}

pub fn number(value: f64) -> Value {
    serde_json::Number::from_f64(value)
        .map(Value::Number)
        .unwrap_or(Value::Null)
}
