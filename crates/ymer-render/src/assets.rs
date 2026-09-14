//! Assetregistret. Scener och komponenter refererar till **namn**;
//! registret slår upp namnet till ett GPU-handtag vid laddning.
//!
//! Det här är skillnaden mot milstolpe 2, där `MeshInstance` innehöll ett
//! råindex i renderarens vektor. Index gick sönder så fort något laddades
//! i annan ordning, och gick inte att förstå i en scenfil.

use std::collections::BTreeMap;
use std::path::Path;

use ymer_core::{AssetSource, BUILTIN_CUBE, BUILTIN_PLANE, BUILTIN_QUAD, MeshId, TextureId};

use crate::{Renderer, primitives};

#[derive(Default)]
pub struct Assets {
    meshes: BTreeMap<String, MeshId>,
    textures: BTreeMap<String, TextureId>,
}

impl Assets {
    /// Registrerar de inbyggda meshar som alltid ska finnas.
    pub fn new(renderer: &mut Renderer) -> Self {
        let mut assets = Self::default();
        let cube = renderer.add_mesh(&primitives::cube(1.0));
        let plane = renderer.add_mesh(&primitives::plane(40.0));
        let quad = renderer.add_mesh(&primitives::quad(1.0));
        assets.meshes.insert(BUILTIN_CUBE.to_string(), cube);
        assets.meshes.insert(BUILTIN_PLANE.to_string(), plane);
        assets.meshes.insert(BUILTIN_QUAD.to_string(), quad);
        assets
    }

    pub fn register_mesh(&mut self, name: impl Into<String>, id: MeshId) {
        self.meshes.insert(name.into(), id);
    }

    pub fn register_texture(&mut self, name: impl Into<String>, id: TextureId) {
        self.textures.insert(name.into(), id);
    }

    /// Okända namn faller tillbaka på kuben istället för att försvinna –
    /// en scen med ett stavfel ska gå att öppna och rätta i editorn.
    pub fn mesh(&self, name: &str) -> MeshId {
        self.meshes
            .get(name)
            .copied()
            .unwrap_or_else(|| self.meshes.get(BUILTIN_CUBE).copied().unwrap_or_default())
    }

    /// Tom sträng eller okänt namn ger den vita 1x1-texturen.
    pub fn texture(&self, name: &str) -> TextureId {
        if name.is_empty() {
            return TextureId(0);
        }
        self.textures.get(name).copied().unwrap_or(TextureId(0))
    }

    pub fn mesh_names(&self) -> impl Iterator<Item = &str> {
        self.meshes.keys().map(String::as_str)
    }

    pub fn texture_names(&self) -> impl Iterator<Item = &str> {
        self.textures.keys().map(String::as_str)
    }

    /// Laddar alla PNG-filer ur en assetkälla. Namnen blir sökvägarna
    /// källan rapporterar, alltså samma strängar som står i scenfilerna –
    /// oavsett om de kommer från disk eller ur ett arkiv.
    pub fn load_textures_from(
        &mut self,
        renderer: &mut Renderer,
        source: &dyn AssetSource,
        prefix: &str,
    ) -> usize {
        let mut loaded = 0;
        for name in source.list(prefix, "png") {
            match source
                .read(&name)
                .map_err(|err| anyhow::anyhow!("{err}"))
                .and_then(|bytes| renderer.add_texture_from_png(&bytes, &name))
            {
                Ok(id) => {
                    self.textures.insert(name, id);
                    loaded += 1;
                }
                Err(err) => log::warn!("kunde inte ladda {name}: {err}"),
            }
        }
        loaded
    }

    /// Laddar alla PNG-filer under `dir` och namnger dem efter sin sökväg
    /// relativt `root`, t.ex. `textures/tegel.png`.
    pub fn load_textures(&mut self, renderer: &mut Renderer, root: &Path, dir: &Path) -> usize {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return 0;
        };

        let mut loaded = 0;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                loaded += self.load_textures(renderer, root, &path);
                continue;
            }
            if path.extension().is_none_or(|extension| extension != "png") {
                continue;
            }

            let name = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");

            match std::fs::read(&path)
                .map_err(|err| anyhow::anyhow!("{err}"))
                .and_then(|bytes| renderer.add_texture_from_png(&bytes, &name))
            {
                Ok(id) => {
                    self.textures.insert(name, id);
                    loaded += 1;
                }
                Err(err) => log::warn!("kunde inte ladda {}: {err}", path.display()),
            }
        }
        loaded
    }
}
