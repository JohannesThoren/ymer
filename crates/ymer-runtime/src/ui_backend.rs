//! Från `ymer-ui` till motorns renderare.
//!
//! Biblioteket producerar rektanglar och textrader; renderaren ritar
//! texturerade kvadrater i skärmrymd. Den här modulen är översättningen
//! mellan de två, och den är medvetet det *enda* stället som känner till
//! båda. `ymer-ui` vet inget om wgpu, och `ymer-render` vet inget om
//! gränssnittsträd.
//!
//! Glyfatlasen laddas upp som en vanlig textur, en gång och sedan när den
//! vuxit. Fyllda ytor ritas med atlasens vita pixel, så text och
//! rektanglar delar material och kan slås ihop till samma draw call.

use ymer_core::{Mat4, MeshId, TextureId, Vec3};
use ymer_render::{Assets, DrawItem, Renderer};
use ymer_ui::{Command, DrawList, FontAtlas};

/// Översätter ritlistor och äger atlasens textur.
pub struct UiBackend {
    atlas: FontAtlas,
    texture: Option<TextureId>,
    /// Atlasens storlek, för att räkna om glyfrutor till UV.
    size: (u32, u32),
    quad: MeshId,
    /// Rutan i atlasen som är helt vit.
    white: ymer_ui::Rect,
}

impl UiBackend {
    pub fn new(atlas: FontAtlas, renderer: &mut Renderer, assets: &Assets) -> Self {
        let mut atlas = atlas;
        let white = atlas.white_pixel();
        let size = atlas.size();
        let mut backend = Self {
            atlas,
            texture: None,
            size,
            quad: assets.mesh(ymer_core::BUILTIN_QUAD),
            white,
        };
        backend.upload(renderer);
        backend
    }

    pub fn atlas(&self) -> &FontAtlas {
        &self.atlas
    }

    pub fn atlas_mut(&mut self) -> &mut FontAtlas {
        &mut self.atlas
    }

    /// Laddar upp atlasen om något rastrerats sedan sist.
    ///
    /// Anropas *efter* [`build`](Self::build): det är den som rastrerar
    /// framens glyfer. Laddar man upp före ritas varje tecken som tomrum
    /// första gången det används – ett fel som bara syns en frame och
    /// därför är lätt att missa.
    ///
    /// Atlasens mått ändras aldrig, så UV:erna i en redan byggd lista
    /// håller även om nya glyfer tillkommit.
    pub fn upload(&mut self, renderer: &mut Renderer) {
        if self.texture.is_some() && !self.atlas.take_dirty() {
            return;
        }
        let (width, height) = self.atlas.size();
        self.size = (width, height);
        let rgba = self.atlas.rgba();
        // add_texture skapar en ny textur varje gång. För en atlas som
        // växer sällan är det gott nog; skulle den växa varje frame vore
        // en uppdatering på plats rätt.
        self.texture = Some(renderer.add_texture(&rgba, width, height, "ui atlas"));
    }

    /// Bygger renderarens items ur en ritlista.
    ///
    /// Resultatet läggs i `RenderList::ui_items`, som ritas i skärmrymd.
    pub fn build(&mut self, list: &DrawList, assets: &Assets) -> Vec<DrawItem> {
        let texture = match self.texture {
            Some(texture) => texture,
            // Utan atlas ritas ingenting hellre än fel sak.
            None => return Vec::new(),
        };
        let mut items = Vec::with_capacity(list.commands.len());

        for command in &list.commands {
            match command {
                Command::Rect { rect, color, .. } => {
                    items.push(self.quad_item(*rect, texture, *color, self.white));
                }
                Command::Image { rect, source, tint } => {
                    let texture = assets.texture(source);
                    // Hela texturen, inte en ruta ur atlasen.
                    let mut item = self.quad_item(*rect, texture, *tint, self.white);
                    item.uv_transform = [0.0, 0.0, 1.0, 1.0];
                    items.push(item);
                }
                Command::Text {
                    rect,
                    text,
                    size,
                    color,
                } => self.glyphs(&mut items, *rect, text, *size, *color, texture),
            }
        }

        items
    }

    fn glyphs(
        &mut self,
        items: &mut Vec<DrawItem>,
        rect: ymer_ui::Rect,
        text: &str,
        size: f32,
        color: ymer_ui::Color,
        texture: TextureId,
    ) {
        let line_height = {
            use ymer_ui::TextMeasure;
            self.atlas.measure("M", size).y
        };
        let ascent = {
            use ymer_ui::TextMeasure;
            self.atlas.ascent(size)
        };

        let mut pen_y = rect.y + ascent;
        for line in text.split('\n') {
            let mut pen_x = rect.x;
            for ch in line.chars() {
                let Some(glyph) = self.atlas.glyph(ch, size) else {
                    continue;
                };
                if !glyph.atlas.is_empty() {
                    let slot = ymer_ui::Rect::new(
                        pen_x + glyph.offset.x,
                        pen_y + glyph.offset.y,
                        glyph.atlas.width,
                        glyph.atlas.height,
                    );
                    items.push(self.quad_item(slot, texture, color, glyph.atlas));
                }
                pen_x += glyph.advance;
            }
            pen_y += line_height;
        }
    }

    /// En kvadrat i skärmrymd med en ruta ur atlasen som UV.
    fn quad_item(
        &self,
        rect: ymer_ui::Rect,
        texture: TextureId,
        color: ymer_ui::Color,
        atlas: ymer_ui::Rect,
    ) -> DrawItem {
        // BUILTIN_QUAD är 1x1 med origo i mitten; skala till rutan och
        // flytta till dess mittpunkt.
        //
        // Höjden är *negativ* med flit. Kvadratens uv har v=0 vid lokalt
        // +y, vilket passar en värld där y pekar uppåt. Skärmrymden här
        // har y nedåt, så utan vändningen hamnar varje glyf upp och ner –
        // panelerna såg rätt ut, för en enfärgad yta har ingen riktning.
        // Rutan är symmetrisk kring sin mitt, så ytan den täcker är
        // densamma.
        let center = rect.center();
        let transform = Mat4::from_translation(Vec3::new(center.x, center.y, 0.0))
            * Mat4::from_scale(Vec3::new(rect.width, -rect.height, 1.0));

        let (aw, ah) = (self.size.0 as f32, self.size.1 as f32);
        let mut item = DrawItem::new(
            self.quad,
            texture,
            transform,
            ymer_core::Color::rgba(color.r, color.g, color.b, color.a),
        );
        item.uv_transform = [
            atlas.x / aw,
            atlas.y / ah,
            atlas.width / aw,
            atlas.height / ah,
        ];
        item
    }
}
