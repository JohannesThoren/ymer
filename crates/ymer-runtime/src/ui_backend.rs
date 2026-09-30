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
//!
//! Klippningen sker *geometriskt*, genom att varje kvadrat skärs mot sitt
//! klipp och dess UV trimmas lika mycket. Alternativet hade varit scissor
//! i renderaren, men det kräver att ritpasset delas upp per klipprektangel
//! och att `RenderList` bär renderingstillstånd. Klippen här är
//! axelriktade, så snittet är exakt – och en halv glyf i kanten på en
//! rullad lista får sin UV trimmad och ser rätt ut.

use ymer_core::{Mat4, MeshId, TextureId, Vec3};
use ymer_render::{Assets, DrawItem, Renderer};
use ymer_ui::{Command, DrawList, FontAtlas};

/// En rektangel från logiska punkter till målets pixlar.
///
/// Gränssnittet läggs ut i punkter, så att en knapp är lika stor på en
/// skärm med dubbel pixeltäthet. Renderarens skärmpass räknar i pixlar.
/// Skalan mellan de två måste läggas på någonstans, och det här är
/// stället — före klippningen, så att klipprektanglarna följer med.
///
/// Missar man det ritas hela gränssnittet i halv storlek uppe i vänstra
/// hörnet på en näthinneskärm, medan musen träffar rätt. Det ser ut som
/// ett layoutfel och är ett enhetsfel.
fn scaled(rect: ymer_ui::Rect, scale: f32) -> ymer_ui::Rect {
    if scale == 1.0 {
        return rect;
    }
    ymer_ui::Rect::new(
        rect.x * scale,
        rect.y * scale,
        rect.width * scale,
        rect.height * scale,
    )
}

/// En textrad som ska bli glyfkvadrater.
#[derive(Debug, Clone, Copy)]
struct TextRun {
    rect: ymer_ui::Rect,
    size: f32,
    color: ymer_ui::Color,
    clip: ymer_ui::Rect,
}

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
    /// Texturen skrivs om *på plats*. Det är hela förutsättningen för den
    /// ordningen: `build` har redan stoppat in texturens id i varje
    /// kvadrat, så en ny textur hade lämnat framens glyfer pekande på den
    /// gamla, tomma atlasen. Symptomet är lömskt – panelerna ritas, för
    /// de använder den vita pixeln som finns från början, och bara texten
    /// försvinner.
    ///
    /// Atlasens mått ändras aldrig, så UV:erna i en redan byggd lista
    /// håller även om nya glyfer tillkommit.
    pub fn upload(&mut self, renderer: &mut Renderer) {
        let dirty = self.atlas.take_dirty();
        let (width, height) = self.atlas.size();
        self.size = (width, height);

        match self.texture {
            Some(texture) if dirty => {
                let rgba = self.atlas.rgba();
                if !renderer.update_texture(texture, &rgba, width, height) {
                    // Måtten gick isär – atlasen har bytts ut. Då är en ny
                    // textur rätt, och framens lista får ritas om nästa
                    // frame.
                    self.texture = Some(renderer.add_texture(&rgba, width, height, "ui atlas"));
                }
            }
            Some(_) => {}
            None => {
                let rgba = self.atlas.rgba();
                self.texture = Some(renderer.add_texture(&rgba, width, height, "ui atlas"));
            }
        }
    }

    /// Bygger renderarens items ur en ritlista.
    ///
    /// Resultatet läggs i `RenderList::ui_items`, som ritas i skärmrymd.
    pub fn build(&mut self, list: &DrawList, assets: &Assets, scale: f32) -> Vec<DrawItem> {
        let texture = match self.texture {
            Some(texture) => texture,
            // Utan atlas ritas ingenting hellre än fel sak.
            None => return Vec::new(),
        };
        let mut items = Vec::with_capacity(list.commands.len());

        for item in &list.commands {
            let clip = scaled(item.clip, scale);
            match &item.command {
                Command::Rect { rect, color, .. } => {
                    // Den vita pixeln är enfärgad, så UV:n behöver inte
                    // trimmas – men rektangeln måste skäras.
                    items.extend(self.quad_item(
                        scaled(*rect, scale),
                        texture,
                        *color,
                        self.atlas_uv(self.white),
                        clip,
                    ));
                }
                Command::Image { rect, source, tint } => {
                    let texture = assets.texture(source);
                    // Hela texturen, inte en ruta ur atlasen.
                    items.extend(self.quad_item(*rect, texture, *tint, [0.0, 0.0, 1.0, 1.0], clip));
                }
                Command::Text {
                    rect,
                    text,
                    size,
                    color,
                } => self.glyphs(
                    &mut items,
                    text,
                    TextRun {
                        rect: scaled(*rect, scale),
                        // Glyferna rastreras i *målets* storlek, inte i
                        // den logiska. Rastrerar man 13 punkter och
                        // förstorar dubbelt blir texten suddig; ber man
                        // atlasen om 26 blir den skarp.
                        size: *size * scale,
                        color: *color,
                        clip,
                    },
                    texture,
                ),
            }
        }

        items
    }

    /// En atlasruta i pixlar till normaliserad UV.
    fn atlas_uv(&self, atlas: ymer_ui::Rect) -> [f32; 4] {
        let (aw, ah) = (self.size.0 as f32, self.size.1 as f32);
        [
            atlas.x / aw,
            atlas.y / ah,
            atlas.width / aw,
            atlas.height / ah,
        ]
    }

    fn glyphs(&mut self, items: &mut Vec<DrawItem>, text: &str, run: TextRun, texture: TextureId) {
        let TextRun {
            rect,
            size,
            color,
            clip,
        } = run;
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
                    let uv = self.atlas_uv(glyph.atlas);
                    items.extend(self.quad_item(slot, texture, color, uv, clip));
                }
                pen_x += glyph.advance;
            }
            pen_y += line_height;
        }
    }

    /// En kvadrat i skärmrymd, skuren mot sitt klipp.
    ///
    /// `None` när ingenting av den syns. UV:n trimmas i samma andelar som
    /// rektangeln, så en halv glyf visar halva glyfen och inte en
    /// ihoptryckt hel.
    fn quad_item(
        &self,
        rect: ymer_ui::Rect,
        texture: TextureId,
        color: ymer_ui::Color,
        uv: [f32; 4],
        clip: ymer_ui::Rect,
    ) -> Option<DrawItem> {
        let visible = rect.intersect(clip);
        if visible.is_empty() {
            return None;
        }
        let uv = if visible == rect {
            uv
        } else {
            // Andelen av originalrektangeln som blev kvar, per kant.
            let fx = (visible.x - rect.x) / rect.width.max(f32::EPSILON);
            let fy = (visible.y - rect.y) / rect.height.max(f32::EPSILON);
            let fw = visible.width / rect.width.max(f32::EPSILON);
            let fh = visible.height / rect.height.max(f32::EPSILON);
            [
                uv[0] + uv[2] * fx,
                uv[1] + uv[3] * fy,
                uv[2] * fw,
                uv[3] * fh,
            ]
        };
        Some(self.quad(visible, texture, color, uv))
    }

    /// En kvadrat i skärmrymd med färdig UV.
    fn quad(
        &self,
        rect: ymer_ui::Rect,
        texture: TextureId,
        color: ymer_ui::Color,
        uv: [f32; 4],
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

        let mut item = DrawItem::new(
            self.quad,
            texture,
            transform,
            ymer_core::Color::rgba(color.r, color.g, color.b, color.a),
        );
        item.uv_transform = uv;
        item
    }
}
