//! `EguiOverlay` bor i `ymer-render` numera – både editorn och spel som
//! använder `ymer-runtime` behöver den. Kvar här som re-export så
//! befintliga `use ymer_editor::overlay::EguiOverlay` inte går sönder.

pub use ymer_render::EguiOverlay;
