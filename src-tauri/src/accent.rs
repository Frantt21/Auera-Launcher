use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};

use serde::{Deserialize, Serialize};

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
