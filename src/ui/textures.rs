use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};

use egui::{ColorImage, Context, TextureHandle, TextureOptions, Vec2};

/// Lado máximo de textura; el arte de SteamGridDB llega hasta 10.000 px.
const MAX_SIDE: u32 = 2048;

#[derive(Clone)]
pub struct Texture {
    pub handle: TextureHandle,
    pub size: Vec2,
}

/// Carga de imágenes en un hilo propio para no bloquear el repintado.
pub struct Textures {
    cache: HashMap<PathBuf, Option<Texture>>,
    requested: HashSet<PathBuf>,
    jobs: Sender<PathBuf>,
    loaded: Receiver<(PathBuf, Option<ColorImage>, Vec2)>,
}

impl Default for Textures {
    fn default() -> Self {
        Self::new()
    }
}

impl Textures {
    pub fn new() -> Self {
        let (jobs, pending) = mpsc::channel::<PathBuf>();
        let (done, loaded) = mpsc::channel();
        std::thread::spawn(move || {
            while let Ok(path) = pending.recv() {
                let (image, size) = load(&path);
                if done.send((path, image, size)).is_err() {
                    break;
                }
            }
        });
        Self {
            cache: HashMap::new(),
            requested: HashSet::new(),
            jobs,
            loaded,
        }
    }

    /// Devuelve la textura si ya está lista; si no, encola su carga.
    pub fn get(&mut self, path: Option<&Path>) -> Option<Texture> {
        let path = path?;
        if let Some(cached) = self.cache.get(path) {
            return cached.clone();
        }
        if !self.requested.contains(path) {
            self.requested.insert(path.to_path_buf());
            let _ = self.jobs.send(path.to_path_buf());
        }
        None
    }

    pub fn forget(&mut self, path: &Path) {
        self.cache.remove(path);
        self.requested.remove(path);
    }

    pub fn is_busy(&self) -> bool {
        !self.requested.is_empty()
    }

    /// Sube a GPU las imágenes ya decodificadas. Devuelve true si hubo cambios.
    pub fn poll(&mut self, ctx: &Context) -> bool {
        let mut updated = false;
        while let Ok((path, image, size)) = self.loaded.try_recv() {
            self.requested.remove(&path);
            let texture = image.map(|image| Texture {
                handle: ctx.load_texture(path.to_string_lossy(), image, TextureOptions::LINEAR),
                size,
            });
            self.cache.insert(path, texture);
            updated = true;
        }
        updated
    }
}

fn load(path: &Path) -> (Option<ColorImage>, Vec2) {
    let Ok(image) = image::open(path) else {
        return (None, Vec2::ZERO);
    };
    let image = if image.width() > MAX_SIDE || image.height() > MAX_SIDE {
        image.thumbnail(MAX_SIDE, MAX_SIDE)
    } else {
        image
    };
    let rgba = image.to_rgba8();
    let size = Vec2::new(rgba.width() as f32, rgba.height() as f32);
    let color_image = ColorImage::from_rgba_unmultiplied(
        [rgba.width() as usize, rgba.height() as usize],
        rgba.as_raw(),
    );
    (Some(color_image), size)
}
