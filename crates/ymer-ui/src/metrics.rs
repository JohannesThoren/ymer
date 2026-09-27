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
