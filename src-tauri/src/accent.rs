use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};

use serde::{Deserialize, Serialize};

use crate::config;

/// Color RGB independiente de la interfaz, para que el núcleo sirva al
/// frontend web de Tauri.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

/// Entrada persistida: el tono solo vale mientras el arte no cambie.
#[derive(Serialize, Deserialize)]
struct Entry {
    modified: u64,
    color: Option<Rgb>,
}

fn cache_path() -> PathBuf {
    config::data_dir().join("accents.json")
}

fn modified(path: &Path) -> u64 {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Tonos resueltos en arranques anteriores; se descartan si el arte cambió.
pub fn load_cache() -> HashMap<PathBuf, Option<Rgb>> {
    read(&cache_path())
}

pub fn save_cache(cache: &HashMap<PathBuf, Option<Rgb>>) {
    write(&cache_path(), cache);
}

fn read(path: &Path) -> HashMap<PathBuf, Option<Rgb>> {
    let Ok(bytes) = std::fs::read(path) else {
        return HashMap::new();
    };
    let entries: HashMap<String, Entry> = serde_json::from_slice(&bytes).unwrap_or_default();
    entries
        .into_iter()
        .filter_map(|(path, entry)| {
            let path = PathBuf::from(path);
            (path.is_file() && modified(&path) == entry.modified).then_some((path, entry.color))
        })
        .collect()
}

fn write(path: &Path, cache: &HashMap<PathBuf, Option<Rgb>>) {
    let entries: HashMap<String, Entry> = cache
        .iter()
        .map(|(path, color)| {
            (
                path.to_string_lossy().into_owned(),
                Entry { modified: modified(path), color: *color },
            )
        })
        .collect();
    let Ok(bytes) = serde_json::to_vec(&entries) else {
        return;
    };
    let temp = path.with_extension("tmp");
    if std::fs::write(&temp, bytes).is_ok() {
        let _ = std::fs::rename(&temp, path);
    }
}

/// Analiza el color dominante del arte en un hilo aparte: decodificar una
/// carátula en mitad del fotograma congela la navegación.
pub struct Accents {
    cache: HashMap<PathBuf, Option<Rgb>>,
    requested: HashSet<PathBuf>,
    jobs: Sender<PathBuf>,
    loaded: Receiver<(PathBuf, Option<Rgb>)>,
}

impl Default for Accents {
    fn default() -> Self {
        Self::new()
    }
}

impl Accents {
    pub fn new() -> Self {
        let (jobs, pending) = mpsc::channel::<PathBuf>();
        let (done, loaded) = mpsc::channel();
        std::thread::spawn(move || {
            while let Ok(path) = pending.recv() {
                if done.send((path.clone(), analyze(&path))).is_err() {
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

    /// Color ya calculado; si falta, encola su análisis y devuelve None.
    pub fn get(&mut self, path: &Path) -> Option<Rgb> {
        if let Some(color) = self.cache.get(path) {
            return *color;
        }
        self.request(path);
        None
    }

    /// False cuando el análisis ya terminó (con o sin color).
    pub fn is_pending(&self, path: &Path) -> bool {
        !self.cache.contains_key(path)
    }

    pub fn request(&mut self, path: &Path) {
        if self.cache.contains_key(path) || !self.requested.insert(path.to_path_buf()) {
            return;
        }
        let _ = self.jobs.send(path.to_path_buf());
    }

    pub fn is_busy(&self) -> bool {
        !self.requested.is_empty()
    }

    pub fn poll(&mut self) -> bool {
        let mut updated = false;
        while let Ok((path, color)) = self.loaded.try_recv() {
            self.requested.remove(&path);
            self.cache.insert(path, color);
            updated = true;
        }
        updated
    }
}

/// Tono vivo del arte, ignorando transparencias, grises y extremos de brillo.
pub fn analyze(path: &Path) -> Option<Rgb> {
    let image = image::open(path).ok()?.thumbnail(32, 32).to_rgba8();
    let mut sum = [0f32; 3];
    let mut weight = 0f32;
    for pixel in image.pixels() {
        let [r, g, b, a] = pixel.0;
        if a < 200 {
            continue;
        }
        let max = r.max(g).max(b) as f32;
        let min = r.min(g).min(b) as f32;
        let saturation = (max - min) / max.max(1.0);
        if saturation < 0.22 || max < 70.0 {
            continue;
        }
        let luminance = 0.2126 * r as f32 + 0.7152 * g as f32 + 0.0722 * b as f32;
        if luminance > 236.0 {
            continue;
        }
        let w = saturation * (1.0 - ((luminance - 150.0).abs() / 260.0)).clamp(0.25, 1.0);
        sum[0] += r as f32 * w;
        sum[1] += g as f32 * w;
        sum[2] += b as f32 * w;
        weight += w;
    }
    if weight <= 0.0 {
        return None;
    }
    let mut rgb = [sum[0] / weight, sum[1] / weight, sum[2] / weight];
    // Se sube el brillo si el tono queda apagado, sin destrozar la saturación.
    let peak = rgb[0].max(rgb[1]).max(rgb[2]);
    if peak < 110.0 {
        let gain = 150.0 / peak.max(1.0);
        for channel in &mut rgb {
            *channel = (*channel * gain).min(235.0);
        }
    }
    Some(Rgb::new(rgb[0] as u8, rgb[1] as u8, rgb[2] as u8))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guarda_y_recupera_el_tono() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/accent-test");
        std::fs::create_dir_all(&dir).unwrap();
        let art = dir.join("arte.png");
        image::RgbImage::from_pixel(32, 32, image::Rgb([200, 40, 90]))
            .save(&art)
            .unwrap();
        let file = dir.join("accents.json");
        let _ = std::fs::remove_file(&file);

        let missing = dir.join("no-existe.png");
        let cache = HashMap::from([(art.clone(), analyze(&art)), (missing.clone(), None)]);
        write(&file, &cache);

        let loaded = read(&file);
        assert_eq!(loaded.get(&art), Some(&cache[&art]));
        assert!(!loaded.contains_key(&missing));
    }
}
