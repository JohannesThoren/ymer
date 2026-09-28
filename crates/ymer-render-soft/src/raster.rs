//! Rasteriseringens räknande: klippning, barycentriska koordinater och
//! interpolation.
//!
//! Skilt från resten för att det går att läsa och pröva utan att veta
//! något om renderaren i övrigt.

use ymer_core::{Vec2, Vec3, Vec4};

/// En triangel som överlevt klippningen.
pub struct Part {
    pub clip: [Vec4; 3],
    pub normals: [Vec3; 3],
    pub uvs: [Vec2; 3],
}

/// Hur nära kameran en vertex får komma innan den klipps bort.
///
/// Divisionen med `w` sprängs när `w` går mot noll, så det räcker inte
/// att kasta trianglar som *helt* ligger bakom kameran – de som skär
/// planet måste kapas.
const NEAR: f32 = 1e-4;

/// Klipper en triangel mot närplanet.
///
/// Sutherland–Hodgman mot det enda plan som spelar roll här. En triangel
/// som skärs blir en fyrhörning, alltså två trianglar; en som ligger helt
/// bakom blir ingen.
pub fn clip_near(clip: &[Vec4; 3], normals: &[Vec3; 3], uvs: &[Vec2; 3]) -> Vec<Part> {
    let inside = |v: Vec4| v.w > NEAR;

    if clip.iter().all(|v| inside(*v)) {
        return vec![Part {
            clip: *clip,
            normals: *normals,
            uvs: *uvs,
        }];
    }
    if clip.iter().all(|v| !inside(*v)) {
        return Vec::new();
    }

    // Polygonen byggs upp kant för kant.
    let mut poly: Vec<(Vec4, Vec3, Vec2)> = Vec::with_capacity(4);
    for index in 0..3 {
        let next = (index + 1) % 3;
        let (a, b) = (clip[index], clip[next]);
        let (a_in, b_in) = (inside(a), inside(b));

        if a_in {
            poly.push((a, normals[index], uvs[index]));
        }
        if a_in != b_in {
            // Var på kanten planet skär.
            let t = (NEAR - a.w) / (b.w - a.w);
            poly.push((
                a + (b - a) * t,
                normals[index] + (normals[next] - normals[index]) * t,
                uvs[index] + (uvs[next] - uvs[index]) * t,
            ));
        }
    }

    // Solfjäder från första hörnet.
    let mut parts = Vec::new();
    for index in 1..poly.len().saturating_sub(1) {
        parts.push(Part {
            clip: [poly[0].0, poly[index].0, poly[index + 1].0],
            normals: [poly[0].1, poly[index].1, poly[index + 1].1],
            uvs: [poly[0].2, poly[index].2, poly[index + 1].2],
        });
    }
    parts
}

/// Dubbla den tecknade arean. Positiv = medurs i skärmrymd.
pub fn signed_area(a: Vec2, b: Vec2, c: Vec2) -> f32 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}

/// Rutan en triangel täcker, klippt mot skärmen.
pub struct Bounds {
    pub min_x: u32,
    pub min_y: u32,
    pub max_x: u32,
    pub max_y: u32,
}

pub fn bounds(screen: &[Vec3; 3], width: u32, height: u32) -> Bounds {
    let min_x = screen.iter().map(|v| v.x).fold(f32::INFINITY, f32::min);
    let max_x = screen.iter().map(|v| v.x).fold(f32::NEG_INFINITY, f32::max);
    let min_y = screen.iter().map(|v| v.y).fold(f32::INFINITY, f32::min);
    let max_y = screen.iter().map(|v| v.y).fold(f32::NEG_INFINITY, f32::max);

    Bounds {
        min_x: min_x.floor().clamp(0.0, width as f32) as u32,
        min_y: min_y.floor().clamp(0.0, height as f32) as u32,
        max_x: (max_x.ceil() + 1.0).clamp(0.0, width as f32) as u32,
        max_y: (max_y.ceil() + 1.0).clamp(0.0, height as f32) as u32,
    }
}

/// Barycentriska vikter för en punkt, eller `None` om den ligger utanför.
///
/// Båda vindningarna accepteras. Motorns mesh-pipeline i wgpu har
/// `cull_mode: None`, och den här backenden ska svara likadant — annars
/// syns marken i den ena renderaren och inte i den andra, vilket är
/// precis den sortens skillnad en andra implementation finns för att
/// hitta.
pub fn barycentric(screen: &[Vec3; 3], point: Vec2, area: f32) -> Option<Vec3> {
    let w0 = signed_area(screen[1].truncate(), screen[2].truncate(), point);
    let w1 = signed_area(screen[2].truncate(), screen[0].truncate(), point);
    let w2 = signed_area(screen[0].truncate(), screen[1].truncate(), point);

    // Punkten ligger inne om den är på samma sida om alla tre kanter,
    // och vilken sida det är avgörs av triangelns vindning.
    if area > 0.0 {
        if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
            return None;
        }
    } else if w0 > 0.0 || w1 > 0.0 || w2 > 0.0 {
        return None;
    }

    // Divisionen med den tecknade arean normaliserar bort tecknet.
    Some(Vec3::new(w0 / area, w1 / area, w2 / area))
}

/// Det som interpolerats fram till en pixel.
pub struct Fragment {
    pub depth: f32,
    pub normal: Vec3,
    pub uv: Vec2,
}

impl Fragment {
    /// Perspektivkorrekt interpolation.
    ///
    /// Attributen vägs med `1/w`, inte rakt av. Utan det glider texturen
    /// på stora trianglar som lutar från kameran – golvet i en scen är
    /// alltid den första som avslöjar det.
    pub fn interpolate(
        bary: Vec3,
        screen: &[Vec3; 3],
        inv_w: &[f32; 3],
        normals: &[Vec3; 3],
        uvs: &[Vec2; 3],
        perspective: bool,
    ) -> Self {
        let depth = bary.x * screen[0].z + bary.y * screen[1].z + bary.z * screen[2].z;

        if !perspective {
            return Self {
                depth,
                normal: Vec3::Y,
                uv: uvs[0] * bary.x + uvs[1] * bary.y + uvs[2] * bary.z,
            };
        }

        let weight = bary.x * inv_w[0] + bary.y * inv_w[1] + bary.z * inv_w[2];
        if weight.abs() < 1e-9 {
            return Self {
                depth,
                normal: normals[0],
                uv: uvs[0],
            };
        }
        let scale = 1.0 / weight;
        let uv = (uvs[0] * (bary.x * inv_w[0])
            + uvs[1] * (bary.y * inv_w[1])
            + uvs[2] * (bary.z * inv_w[2]))
            * scale;
        let normal = (normals[0] * (bary.x * inv_w[0])
            + normals[1] * (bary.y * inv_w[1])
            + normals[2] * (bary.z * inv_w[2]))
            * scale;

        Self { depth, normal, uv }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tri(w: [f32; 3]) -> [Vec4; 3] {
        [
            Vec4::new(0.0, 1.0, 0.5, w[0]),
            Vec4::new(-1.0, -1.0, 0.5, w[1]),
            Vec4::new(1.0, -1.0, 0.5, w[2]),
        ]
    }

    #[test]
    fn helt_framfor_kameran_klipps_inte() {
        let parts = clip_near(&tri([1.0, 1.0, 1.0]), &[Vec3::Y; 3], &[Vec2::ZERO; 3]);
        assert_eq!(parts.len(), 1);
    }

    #[test]
    fn helt_bakom_kameran_forsvinner() {
        let parts = clip_near(&tri([-1.0, -1.0, -1.0]), &[Vec3::Y; 3], &[Vec2::ZERO; 3]);
        assert!(parts.is_empty());
    }

    #[test]
    fn en_hornpunkt_bakom_ger_tva_trianglar() {
        // Skärs triangeln av planet blir resten en fyrhörning.
        let parts = clip_near(&tri([-1.0, 1.0, 1.0]), &[Vec3::Y; 3], &[Vec2::ZERO; 3]);
        assert_eq!(parts.len(), 2);
        for part in &parts {
            for corner in &part.clip {
                assert!(
                    corner.w >= NEAR - 1e-6,
                    "en hörnpunkt blev kvar bakom planet"
                );
            }
        }
    }

    #[test]
    fn tva_hornpunkter_bakom_ger_en_triangel() {
        let parts = clip_near(&tri([1.0, -1.0, -1.0]), &[Vec3::Y; 3], &[Vec2::ZERO; 3]);
        assert_eq!(parts.len(), 1);
    }

    #[test]
    fn ryggsidan_far_negativ_area() {
        let a = Vec2::new(0.0, 0.0);
        let b = Vec2::new(10.0, 0.0);
        let c = Vec2::new(0.0, 10.0);
        assert!(signed_area(a, b, c) > 0.0);
        assert!(signed_area(a, c, b) < 0.0, "vänd ordning ska byta tecken");
    }

    #[test]
    fn omvand_vindning_rasteriseras_ocksa() {
        // Marken i motorns primitiver är vänd åt andra hållet än kuberna.
        // Med bortsortering försvann den helt.
        let screen = [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 10.0, 0.0),
            Vec3::new(10.0, 0.0, 0.0),
        ];
        let area = signed_area(
            screen[0].truncate(),
            screen[1].truncate(),
            screen[2].truncate(),
        );
        assert!(area < 0.0, "triangeln ska vara motvänd");

        let bary = barycentric(&screen, Vec2::new(1.0, 1.0), area).expect("punkten ligger inne");
        for weight in [bary.x, bary.y, bary.z] {
            assert!((0.0..=1.0).contains(&weight), "vikt utanför 0..1: {weight}");
        }
        assert!((bary.x + bary.y + bary.z - 1.0).abs() < 1e-5);
        assert!(barycentric(&screen, Vec2::new(9.0, 9.0), area).is_none());
    }

    #[test]
    fn punkter_utanfor_triangeln_avvisas() {
        let screen = [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(10.0, 0.0, 0.0),
            Vec3::new(0.0, 10.0, 0.0),
        ];
        let area = signed_area(
            screen[0].truncate(),
            screen[1].truncate(),
            screen[2].truncate(),
        );
        assert!(barycentric(&screen, Vec2::new(1.0, 1.0), area).is_some());
        assert!(barycentric(&screen, Vec2::new(9.0, 9.0), area).is_none());
    }

    #[test]
    fn perspektivkorrekt_uv_skiljer_sig_fran_rak() {
        // Två hörn dubbelt så långt bort som det tredje. Mitt i triangeln
        // ska UV:n dras mot det närmaste hörnet.
        let screen = [
            Vec3::new(0.0, 0.0, 0.5),
            Vec3::new(10.0, 0.0, 0.5),
            Vec3::new(0.0, 10.0, 0.5),
        ];
        let inv_w = [1.0, 0.5, 0.5];
        let uvs = [
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, 1.0),
        ];
        let bary = Vec3::new(1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0);

        let korrekt = Fragment::interpolate(bary, &screen, &inv_w, &[Vec3::Y; 3], &uvs, true);
        let rakt = Fragment::interpolate(bary, &screen, &[1.0; 3], &[Vec3::Y; 3], &uvs, true);

        assert!(
            korrekt.uv.x < rakt.uv.x,
            "{} mot {}",
            korrekt.uv.x,
            rakt.uv.x
        );
    }
}
