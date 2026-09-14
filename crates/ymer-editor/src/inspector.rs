//! Typad inspector.
//!
//! Komponenterna har inget schema – men `read_json` ger deras *form*, och
//! det räcker längre än man tror. Tal blir dragfält, bool blir kryssruta,
//! `[x,y,z]` blir tre fält på rad, `{r,g,b,a}` blir en färgväljare.
//!
//! Ovanpå formen ligger en handfull namnbaserade specialfall: `rotation`
//! med fyra tal redigeras som eulervinklar i grader, och `Script` får en
//! lista över projektets skriptfiler istället för en textruta.

use serde_json::Value;
use ymer_core::{Quat, Vec3};

pub struct Context<'a> {
    /// Skriptfiler, relativa skriptkatalogen.
    pub scripts: &'a [String],
    /// Texturer, namngivna relativt projektroten – samma namn som i assets.
    pub textures: &'a [String],
    /// Alla kända meshnamn: `builtin/*` plus importerade glTF-meshar.
    pub meshes: &'a [String],
}

/// Ritar en hel komponent. Returnerar true om något ändrades.
pub fn edit_component(
    ui: &mut egui::Ui,
    component: &str,
    value: &mut Value,
    ctx: &Context,
) -> bool {
    // Newtype-komponenter serialiseras som sitt innehåll.
    if component == "Script"
        && let Value::String(current) = value
    {
        return asset_picker(ui, "script_picker", current, ctx.scripts, "– välj skript –");
    }

    edit_value(ui, None, value, ctx)
}

fn edit_value(ui: &mut egui::Ui, label: Option<&str>, value: &mut Value, ctx: &Context) -> bool {
    match value {
        Value::Bool(flag) => ui.checkbox(flag, "").changed(),

        Value::Number(number) => {
            // Heltal är index och antal – de ska inte visas som 0.00 och
            // inte dras i steg om hundradelar.
            if number.is_i64() || number.is_u64() {
                let mut integer = value.as_i64().unwrap_or_default();
                if ui
                    .add(egui::DragValue::new(&mut integer).speed(1.0))
                    .changed()
                {
                    *value = Value::Number(integer.into());
                    return true;
                }
                return false;
            }

            let mut float = value.as_f64().unwrap_or_default();
            let speed = label.map(step_for).unwrap_or(0.05);
            if ui
                .add(egui::DragValue::new(&mut float).speed(speed))
                .changed()
            {
                *value = number_value(float);
                return true;
            }
            false
        }

        Value::String(text) => {
            // Mesh- och texturfälten är assetreferenser, inte fritext.
            if label == Some("mesh") {
                return asset_picker(ui, "mesh_picker", text, ctx.meshes, "– välj mesh –");
            }
            if label == Some("texture") {
                return asset_picker(ui, "texture_picker", text, ctx.textures, "– ingen textur –");
            }
            ui.text_edit_singleline(text).changed()
        }

        Value::Array(items) => edit_array(ui, label, items, ctx),

        Value::Object(map) => {
            if let Some(changed) = color_picker(ui, map) {
                return changed;
            }

            let mut changed = false;
            for (key, field) in map.iter_mut() {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(key).weak());
                    changed |= edit_value(ui, Some(key), field, ctx);
                });
            }
            changed
        }

        Value::Null => {
            ui.label(egui::RichText::new("null").weak());
            false
        }
    }
}

fn edit_array(ui: &mut egui::Ui, label: Option<&str>, items: &mut [Value], ctx: &Context) -> bool {
    let numeric = items.iter().all(Value::is_number);

    // Kvaternioner är omöjliga att redigera för hand – visa grader istället.
    if numeric && items.len() == 4 && label == Some("rotation") {
        return euler_editor(ui, items);
    }

    if numeric && (2..=4).contains(&items.len()) {
        const AXES: [&str; 4] = ["x", "y", "z", "w"];
        let mut changed = false;
        ui.horizontal(|ui| {
            for (index, item) in items.iter_mut().enumerate() {
                ui.label(egui::RichText::new(AXES[index]).weak().monospace());
                let mut number = item.as_f64().unwrap_or_default();
                if ui
                    .add(egui::DragValue::new(&mut number).speed(0.05))
                    .changed()
                {
                    *item = number_value(number);
                    changed = true;
                }
            }
        });
        return changed;
    }

    let mut changed = false;
    for (index, item) in items.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(format!("[{index}]")).weak().monospace());
            changed |= edit_value(ui, None, item, ctx);
        });
    }
    changed
}

/// Kvaternion in, tre gradfält ut, kvaternion tillbaka.
fn euler_editor(ui: &mut egui::Ui, items: &mut [Value]) -> bool {
    let quat = Quat::from_xyzw(
        items[0].as_f64().unwrap_or(0.0) as f32,
        items[1].as_f64().unwrap_or(0.0) as f32,
        items[2].as_f64().unwrap_or(0.0) as f32,
        items[3].as_f64().unwrap_or(1.0) as f32,
    )
    .normalize();

    let (y, x, z) = quat.to_euler(ymer_core::glam::EulerRot::YXZ);
    let mut degrees = Vec3::new(x.to_degrees(), y.to_degrees(), z.to_degrees());
    let original = degrees;

    ui.horizontal(|ui| {
        for (index, axis) in ["x", "y", "z"].iter().enumerate() {
            ui.label(egui::RichText::new(*axis).weak().monospace());
            ui.add(
                egui::DragValue::new(&mut degrees[index])
                    .speed(1.0)
                    .suffix("°"),
            );
        }
    });

    if degrees == original {
        return false;
    }

    let updated = Quat::from_euler(
        ymer_core::glam::EulerRot::YXZ,
        degrees.y.to_radians(),
        degrees.x.to_radians(),
        degrees.z.to_radians(),
    )
    .normalize();

    items[0] = number_value(updated.x as f64);
    items[1] = number_value(updated.y as f64);
    items[2] = number_value(updated.z as f64);
    items[3] = number_value(updated.w as f64);
    true
}

/// Objekt med nycklarna r/g/b/a redigeras som färg, inte som fyra tal.
fn color_picker(ui: &mut egui::Ui, map: &mut serde_json::Map<String, Value>) -> Option<bool> {
    let keys: Vec<&str> = map.keys().map(String::as_str).collect();
    if keys.len() != 4 || !["a", "b", "g", "r"].iter().all(|key| keys.contains(key)) {
        return None;
    }

    let mut rgba = [
        map["r"].as_f64().unwrap_or(0.0) as f32,
        map["g"].as_f64().unwrap_or(0.0) as f32,
        map["b"].as_f64().unwrap_or(0.0) as f32,
        map["a"].as_f64().unwrap_or(1.0) as f32,
    ];

    let mut changed = false;
    ui.horizontal(|ui| {
        if ui.color_edit_button_rgba_unmultiplied(&mut rgba).changed() {
            for (index, key) in ["r", "g", "b", "a"].iter().enumerate() {
                map.insert((*key).to_string(), number_value(rgba[index] as f64));
            }
            changed = true;
        }
        ui.label(
            egui::RichText::new(format!("{:.2}  {:.2}  {:.2}", rgba[0], rgba[1], rgba[2]))
                .weak()
                .monospace(),
        );
    });

    Some(changed)
}

/// Fält som pekar på en fil i projektet får en lista, inte en textruta.
fn asset_picker(
    ui: &mut egui::Ui,
    id: &str,
    current: &mut String,
    options: &[String],
    placeholder: &str,
) -> bool {
    let mut changed = false;

    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt(id)
            .selected_text(if current.is_empty() {
                placeholder
            } else {
                current.as_str()
            })
            .width(190.0)
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(current.is_empty(), placeholder)
                    .clicked()
                {
                    current.clear();
                    changed = true;
                }
                for option in options {
                    if ui.selectable_label(current == option, option).clicked() {
                        *current = option.clone();
                        changed = true;
                    }
                }
                if options.is_empty() {
                    ui.label(egui::RichText::new("inget att välja på").weak());
                }
            });

        if ui.text_edit_singleline(current).changed() {
            changed = true;
        }
    });

    changed
}

/// Vinklar i radianer vill ha finare steg än positioner.
fn step_for(label: &str) -> f64 {
    if label.ends_with("_radians") {
        0.01
    } else {
        0.05
    }
}

fn number_value(number: f64) -> Value {
    serde_json::Number::from_f64(number)
        .map(Value::Number)
        .unwrap_or(Value::Null)
}

/// Alla `.ts`-filer i en katalog, som sökvägar relativa till den.
pub fn list_scripts(dir: &std::path::Path) -> Vec<String> {
    list_files(dir, dir, "ts")
}

/// Filer med en viss ändelse, namngivna relativt `base`.
pub fn list_files(dir: &std::path::Path, base: &std::path::Path, extension: &str) -> Vec<String> {
    fn walk(
        dir: &std::path::Path,
        base: &std::path::Path,
        extension: &str,
        found: &mut Vec<String>,
    ) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, base, extension, found);
            } else if path.extension().is_some_and(|found| found == extension) {
                let relative = path.strip_prefix(base).unwrap_or(&path);
                found.push(relative.to_string_lossy().replace('\\', "/"));
            }
        }
    }

    let mut found = Vec::new();
    walk(dir, base, extension, &mut found);
    found.sort();
    found
}
