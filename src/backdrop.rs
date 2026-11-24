use std::path::{Path, PathBuf};

use crate::config;

/// Lado máximo del fondo cacheado: suficiente para pantalla completa sin PNG de 10k.
const MAX_SIDE: u32 = 1600;

pub fn cache_path(id: &str) -> PathBuf {
    config::data_dir()
        .join("backdrops")
        .join(format!("{}.jpg", slug(id)))
}

/// Nombre corto y estable: ids de carpeta pueden superar el límite del sistema de archivos.
fn slug(id: &str) -> String {
    let short: String = id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .take(48)
        .collect();
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in id.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    format!("{short}-{hash:016x}")
}

/// Reduce el arte original y lo guarda para reutilizarlo entre arranques.
pub fn generate(id: &str, source: &Path) -> Option<PathBuf> {
    let image = image::open(source).ok()?;
    let (width, height) = (image.width(), image.height());
    let longest = width.max(height);
    let resized = if longest > MAX_SIDE {
        let scale = MAX_SIDE as f32 / longest as f32;
        image.resize(
            ((width as f32 * scale).round() as u32).max(1),
            ((height as f32 * scale).round() as u32).max(1),
            image::imageops::FilterType::Triangle,
        )
    } else {
        image
    };

    let path = cache_path(id);
    let folder = path.parent()?;
    std::fs::create_dir_all(folder).ok()?;
    // Escritura atómica: nadie debe leer un JPEG a medio escribir.
    let temp = path.with_extension("tmp");
    resized.to_rgb8().save(&temp).ok()?;
    std::fs::rename(&temp, &path).ok()?;
    Some(path)
}

pub fn invalidate(id: &str) {
    let path = cache_path(id);
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(path.with_extension("tmp"));
}
