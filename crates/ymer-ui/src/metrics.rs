//! Mått som flera delar måste vara överens om.
//!
//! Layouten mäter en kryssruta, ritaren ritar den och träffdetekteringen
//! avgör var man kan klicka. Blir de oense hamnar bocken bredvid rutan,
//! och sådana fel är svåra att se i koden. Därför står talen här, en gång.

/// Mellanrum mellan en ruta och dess etikett.
pub const GAP: f32 = 6.0;
/// Greppets bredd på ett reglage.
pub const HANDLE: f32 = 12.0;
/// Pilen till höger i en dropdown.
pub const ARROW: f32 = 10.0;
/// Minsta bredd på ett textfält, så att ett tomt fält syns.
pub const MIN_FIELD: f32 = 80.0;
/// Textmarkörens bredd.
pub const CARET: f32 = 1.0;
/// Rullningslistens bredd.
pub const SCROLLBAR: f32 = 6.0;
/// Sifferfältets vilobredd. Bred nog för ett tal med tecken och två
/// decimaler, så att en rad fält inte hoppar i bredd när man drar.
pub const NUMBER_FIELD: f32 = 62.0;
/// Avdelarens tjocklek.
pub const DIVIDER: f32 = 5.0;
