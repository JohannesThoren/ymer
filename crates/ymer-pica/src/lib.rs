//! Dataomvandlingar för PICA200.
//!
//! Allt som måste vara *exakt* rätt för att en 3DS ska visa något, men
//! som inte behöver en 3DS för att räknas ut. Ligger skilt från
//! backenden med flit: texturtiling och matrisordning är den sortens kod
//! där ett fel ger en bild som ser nästan rätt ut, och den sortens fel
//! vill man hitta med ett test på skrivbordet och inte genom att titta
//! på en handhållen skärm.
//!
//! Ingenting här länkar mot libctru, så crate:n bygger och testas var
//! som helst.

use ymer_core::Mat4;

/// Texlar per sida i en PICA-bricka.
pub const TILE: usize = 8;

/// Största textur GPU:n tar.
pub const MAX_TEXTURE: u32 = 1024;

/// Minsta. Under det vägrar hårdvaran.
pub const MIN_TEXTURE: u32 = 8;

/// Varför en textur inte gick att använda.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureError {
    /// PICA200 kan bara adressera tvåpotenser.
    NotPowerOfTwo { width: u32, height: u32 },
    /// Utanför 8..=1024.
    OutOfRange { width: u32, height: u32 },
    /// Pixeldatan stämmer inte med måtten.
    WrongLength { expected: usize, got: usize },
}

impl core::fmt::Display for TextureError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NotPowerOfTwo { width, height } => {
                write!(f, "{width}x{height} är inte tvåpotenser")
            }
            Self::OutOfRange { width, height } => write!(
                f,
                "{width}x{height} ligger utanför {MIN_TEXTURE}..={MAX_TEXTURE}"
            ),
            Self::WrongLength { expected, got } => {
                write!(f, "väntade {expected} byte pixeldata, fick {got}")
            }
        }
    }
}

impl std::error::Error for TextureError {}

/// Duger måtten åt hårdvaran?
pub fn check_size(width: u32, height: u32) -> Result<(), TextureError> {
    if !width.is_power_of_two() || !height.is_power_of_two() {
        return Err(TextureError::NotPowerOfTwo { width, height });
    }
    if !(MIN_TEXTURE..=MAX_TEXTURE).contains(&width)
        || !(MIN_TEXTURE..=MAX_TEXTURE).contains(&height)
    {
        return Err(TextureError::OutOfRange { width, height });
    }
    Ok(())
}

/// Var i en 8x8-bricka en texel hamnar.
///
/// PICA lägger texlarna i Z-ordning (Morton) inom brickan: bitarna i x
/// och y varvas. Det är inte en optimering man kan hoppa över — GPU:n
/// läser så, och en textur som lagts rad för rad kommer ut som brus i
/// åttondelsrutor.
pub const fn morton(x: usize, y: usize) -> usize {
    (x & 1) | ((y & 1) << 1) | ((x & 2) << 1) | ((y & 2) << 2) | ((x & 4) << 2) | ((y & 4) << 3)
}

/// Gör om rad-för-rad RGBA8 till det GPU:n vill ha.
///
/// Två saker händer på en gång.
///
/// *Brickorna*: bilden delas i 8x8-rutor som läggs efter varandra, och
/// inom varje ruta ligger texlarna i Morton-ordning.
///
/// *Byteordningen*: det PICA kallar `Rgba8` ligger som ABGR i minnet.
/// Skickar man RGBA rakt av blir rött blått och bilden genomskinlig där
/// den skulle vara ogenomskinlig — ett fel som ser ut som en trasig
/// textur snarare än som fel ordning.
pub fn tile_rgba8(src: &[u8], width: u32, height: u32) -> Result<Vec<u8>, TextureError> {
    check_size(width, height)?;
    let expected = (width * height * 4) as usize;
    if src.len() != expected {
        return Err(TextureError::WrongLength {
            expected,
            got: src.len(),
        });
    }

    let (width, height) = (width as usize, height as usize);
    let tiles_across = width / TILE;
    let mut out = vec![0u8; expected];

    for y in 0..height {
        for x in 0..width {
            let tile = (y / TILE) * tiles_across + (x / TILE);
            let slot = tile * TILE * TILE + morton(x % TILE, y % TILE);

            let from = (y * width + x) * 4;
            let to = slot * 4;
            // RGBA in, ABGR ut.
            out[to] = src[from + 3];
            out[to + 1] = src[from + 2];
            out[to + 2] = src[from + 1];
            out[to + 3] = src[from];
        }
    }
    Ok(out)
}

/// En matris i den ordning citro3d vill ha den.
///
/// glam lagrar kolumnvis, PICA:s uniformer läses radvis: varje `dp4` i
/// shadern tar en *rad* av matrisen mot vertexen. Utan transponeringen
/// blir scenen inte fel på ett sätt man känner igen — den blir skjuvad,
/// vilket är lätt att tro är en trasig kamera.
pub fn rows(matrix: Mat4) -> [[f32; 4]; 4] {
    let columns = matrix.to_cols_array_2d();
    let mut out = [[0.0f32; 4]; 4];
    for (row, slot) in out.iter_mut().enumerate() {
        for (column, value) in slot.iter_mut().enumerate() {
            *value = columns[column][row];
        }
    }
    out
}

/// Modellmatrisens övre 3x3, radvis, för normalen.
pub fn normal_rows(matrix: Mat4) -> [[f32; 4]; 3] {
    let full = rows(matrix);
    [
        [full[0][0], full[0][1], full[0][2], 0.0],
        [full[1][0], full[1][1], full[1][2], 0.0],
        [full[2][0], full[2][1], full[2][2], 0.0],
    ]
}

/// Så många hörn en mesh får ha.
///
/// Index är 16 bitar på PICA. En mesh som spränger det måste delas, och
/// det ska sägas när den laddas och inte visa sig som saknad geometri.
pub const MAX_VERTICES: usize = u16::MAX as usize + 1;

/// Smalnar index från 32 till 16 bitar.
pub fn narrow_indices(indices: &[u32]) -> Result<Vec<u16>, usize> {
    if let Some(&worst) = indices.iter().max()
        && worst as usize >= MAX_VERTICES
    {
        return Err(worst as usize);
    }
    Ok(indices.iter().map(|&index| index as u16).collect())
}

/// Färg till det ord citro3d vill ha för klarfärg och konstantfärg.
///
/// 0xRRGGBBAA. Motorns färger är linjära och skärmen väntar sig sRGB, så
/// omvandlingen sker här — samma som mjukvarurenderaren gör när den
/// lämnar ut sin bild.
pub fn color_word(color: ymer_core::Color) -> u32 {
    let encode = |value: f32| -> u32 {
        let value = value.clamp(0.0, 1.0);
        let srgb = if value <= 0.0031308 {
            value * 12.92
        } else {
            1.055 * value.powf(1.0 / 2.4) - 0.055
        };
        (srgb * 255.0 + 0.5) as u32
    };
    (encode(color.r) << 24) | (encode(color.g) << 16) | (encode(color.b) << 8) | encode(color.a)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn morton_varvar_bitarna() {
        // Hörnen i en bricka, enligt Z-ordningen.
        assert_eq!(morton(0, 0), 0);
        assert_eq!(morton(1, 0), 1);
        assert_eq!(morton(0, 1), 2);
        assert_eq!(morton(1, 1), 3);
        assert_eq!(morton(7, 7), 63, "sista texeln ska ligga sist");
    }

    #[test]
    fn morton_ar_en_bijektion_over_brickan() {
        // Varje plats används exakt en gång. Missar den någon hamnar en
        // texel ovanpå en annan och bilden får hål.
        let mut seen = [false; 64];
        for y in 0..TILE {
            for x in 0..TILE {
                let slot = morton(x, y);
                assert!(!seen[slot], "platsen {slot} användes två gånger");
                seen[slot] = true;
            }
        }
        assert!(seen.iter().all(|&used| used));
    }

    #[test]
    fn tiling_behaller_alla_texlar() {
        // 8x8 med unika röda värden: efter tiling ska varje värde finnas
        // kvar exakt en gång.
        let mut src = Vec::new();
        for i in 0..64u32 {
            src.extend_from_slice(&[i as u8, 0, 0, 255]);
        }
        let tiled = tile_rgba8(&src, 8, 8).unwrap();

        let mut found: Vec<u8> = tiled
            .as_chunks::<4>()
            .0
            .iter()
            .map(|texel| texel[3])
            .collect();
        found.sort_unstable();
        assert_eq!(found, (0..64u8).collect::<Vec<_>>());
    }

    #[test]
    fn tiling_lagger_om_till_abgr() {
        let src = [10, 20, 30, 40];
        let tiled = tile_rgba8(&src.repeat(64), 8, 8).unwrap();
        assert_eq!(&tiled[0..4], &[40, 30, 20, 10], "ska vara ABGR");
    }

    #[test]
    fn tiling_placerar_andra_brickan_efter_den_forsta() {
        // 16x8 = två brickor. Texeln på (8,0) är första i bricka två,
        // alltså byte 64*4 i utdatan.
        let mut src = vec![0u8; 16 * 8 * 4];
        let index = 8 * 4;
        src[index] = 123;
        let tiled = tile_rgba8(&src, 16, 8).unwrap();
        assert_eq!(tiled[64 * 4 + 3], 123);
    }

    #[test]
    fn matten_provas() {
        assert!(check_size(64, 64).is_ok());
        assert!(check_size(1024, 1024).is_ok());
        assert_eq!(
            check_size(100, 64),
            Err(TextureError::NotPowerOfTwo {
                width: 100,
                height: 64
            })
        );
        assert_eq!(
            check_size(4, 4),
            Err(TextureError::OutOfRange {
                width: 4,
                height: 4
            })
        );
        assert_eq!(
            check_size(2048, 64),
            Err(TextureError::OutOfRange {
                width: 2048,
                height: 64
            })
        );
    }

    #[test]
    fn fel_mangd_pixeldata_fangas() {
        assert!(matches!(
            tile_rgba8(&[0; 10], 8, 8),
            Err(TextureError::WrongLength { .. })
        ));
    }

    #[test]
    fn matrisen_transponeras() {
        // En ren flytt: i kolumnordning ligger flytten i sista kolumnen,
        // i radordning i sista *elementet* av de tre första raderna.
        let matrix = Mat4::from_translation(ymer_core::Vec3::new(1.0, 2.0, 3.0));
        let rows = rows(matrix);
        assert_eq!(rows[0][3], 1.0);
        assert_eq!(rows[1][3], 2.0);
        assert_eq!(rows[2][3], 3.0);
        assert_eq!(rows[3], [0.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn normalmatrisen_tappar_flytten() {
        let matrix = Mat4::from_scale_rotation_translation(
            ymer_core::Vec3::splat(2.0),
            ymer_core::Quat::IDENTITY,
            ymer_core::Vec3::new(5.0, 6.0, 7.0),
        );
        let normal = normal_rows(matrix);
        assert_eq!(normal[0], [2.0, 0.0, 0.0, 0.0]);
        assert_eq!(normal[1], [0.0, 2.0, 0.0, 0.0]);
        assert_eq!(normal[2], [0.0, 0.0, 2.0, 0.0]);
    }

    #[test]
    fn index_smalnar_och_klagar_i_tid() {
        assert_eq!(narrow_indices(&[0, 1, 2]).unwrap(), vec![0u16, 1, 2]);
        assert_eq!(narrow_indices(&[70000]), Err(70000));
    }

    #[test]
    fn fargordet_ar_rrggbbaa() {
        let word = color_word(ymer_core::Color::rgba(1.0, 0.0, 0.0, 1.0));
        assert_eq!(word, 0xFF0000FF, "rött ska ligga högst: {word:08X}");

        let svart = color_word(ymer_core::Color::rgba(0.0, 0.0, 0.0, 1.0));
        assert_eq!(svart, 0x000000FF);
    }

    #[test]
    fn fargen_kodas_till_srgb() {
        // Linjärt halvljus är inte 128 i sRGB utan omkring 188. Missar
        // man det blir hela scenen märkbart mörkare på hårdvaran än i
        // editorn.
        let word = color_word(ymer_core::Color::rgba(0.5, 0.5, 0.5, 1.0));
        let red = (word >> 24) & 0xFF;
        assert!((186..=190).contains(&red), "blev {red}");
    }
}
