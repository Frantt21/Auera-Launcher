use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::config;

/// Lado máximo del fondo cacheado: suficiente para pantalla completa sin arrastrar
/// el arte original, que en SteamGridDB puede llegar a 4K o más.
const MAX_SIDE: u32 = 1600;
const QUALITY: u8 = 82;

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
    let path = cache_path(id);
    if path.is_file() {
        return Some(path);
    }
    encode(source, &path).then_some(path)
}

/// Escribe el JPEG reducido. `DynamicImage::save` falla con una ruta temporal
/// sin extensión conocida, así que el codificador va explícito.
fn encode(source: &Path, path: &Path) -> bool {
    let Ok(image) = image::open(source) else {
        return false;
    };
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

    let Some(folder) = path.parent() else {
        return false;
    };
    if std::fs::create_dir_all(folder).is_err() {
        return false;
    }
    // Escritura atómica y con nombre único: nadie debe leer un JPEG a medio
    // escribir ni chocar con otro hilo generando el mismo fondo.
    let temp = temp_path(path);
    let Ok(file) = std::fs::File::create(&temp) else {
        return false;
    };
    let mut writer = std::io::BufWriter::new(file);
    let encoded = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut writer, QUALITY)
        .encode_image(&resized.to_rgb8())
        .is_ok()
        && writer.flush().is_ok();
    drop(writer);
    if encoded && std::fs::rename(&temp, path).is_ok() {
        true
    } else {
        let _ = std::fs::remove_file(&temp);
        false
    }
}

pub fn invalidate(id: &str) {
    let path = cache_path(id);
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(path.with_extension("tmp"));
}

fn temp_path(path: &Path) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    path.with_extension(format!("{unique}.tmp"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escribe_un_jpeg_reducido() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/backdrop-test");
        std::fs::create_dir_all(&dir).unwrap();
        let source = dir.join("origen.png");
        image::RgbImage::from_pixel(2400, 800, image::Rgb([30, 60, 120]))
            .save(&source)
            .unwrap();
        let target = dir.join("copia.jpg");
        let _ = std::fs::remove_file(&target);

        assert!(encode(&source, &target));
        assert!(target.is_file());
        let written = image::open(&target).unwrap();
        assert!(written.width().max(written.height()) <= MAX_SIDE);
    }

}
