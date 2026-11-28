use std::path::{Path, PathBuf};


use image::GenericImageView;
use percent_encoding::percent_decode_str;
use tauri::http::Response;

pub fn base() -> &'static str {
    if cfg!(windows) {
        "http://auera.localhost"
    } else {
        "auera://localhost"
    }
}

pub fn url(path: &Path, width: u32) -> String {
    let owned = path.to_string_lossy().into_owned();
    let encoded = percent_encoding::utf8_percent_encode(&owned, percent_encoding::NON_ALPHANUMERIC);
    if width == 0 {
        format!("{}/media?path={}", base(), encoded)
    } else {
        format!("{}/media?path={}&w={}", base(), encoded, width)
    }
}

pub fn respond(request: &tauri::http::Request<Vec<u8>>) -> Response<Vec<u8>> {
    let uri = request.uri();
    let query = uri.query().unwrap_or("");
    let path_param = query_param(query, "path");
    let width: u32 = query_param(query, "w").and_then(|s| s.parse().ok()).unwrap_or(0);

    let Some(path_param) = path_param else {
        return not_found();
    };
    let Ok(path) = PathBuf::from(&path_param).canonicalize() else {
        return not_found();
    };
    if !inside(&path) {
        return forbidden();
    }

    if width == 0 {
        let Ok(bytes) = std::fs::read(&path) else {
            return not_found();
        };
        let mime = mime_for(&path);
        return Response::builder()
            .status(200)
            .header("Content-Type", mime)
            .header("Access-Control-Allow-Origin", "*")
            .header("Cache-Control", "no-store")
            .body(bytes)
            .unwrap_or_else(|_| not_found());
    }

    match resize_cached(&path, width) {
        Ok((bytes, mime)) => Response::builder()
            .status(200)
            .header("Content-Type", mime)
            .header("Access-Control-Allow-Origin", "*")
            .header("Cache-Control", "no-store")
            .body(bytes)
            .unwrap_or_else(|_| not_found()),
        Err(_) => not_found(),
    }
}

fn inside(path: &Path) -> bool {
    let root = std::fs::canonicalize(crate::config::data_dir()).unwrap_or_else(|_| crate::config::data_dir());
    path.starts_with(root)
}

/// Solo se sirve y prepara arte dentro del directorio de datos de la app.
pub fn allowed(path_param: &str) -> Option<PathBuf> {
    let path = PathBuf::from(path_param).canonicalize().ok()?;
    inside(&path).then_some(path)
}

/// Deja lista la versión reducida fuera del hilo del protocolo: generarla al
/// servirla bloquea la interfaz mientras se decodifica y recodifica el arte.
pub fn warm(path: &Path, width: u32) -> bool {
    width > 0 && resize_cached(path, width).is_ok()
}

fn not_found() -> Response<Vec<u8>> {
    Response::builder()
        .status(404)
        .header("Access-Control-Allow-Origin", "*")
        .body(Vec::new())
        .unwrap()
}

fn forbidden() -> Response<Vec<u8>> {
    Response::builder()
        .status(403)
        .header("Access-Control-Allow-Origin", "*")
        .body(Vec::new())
        .unwrap()
}

fn query_param(query: &str, key: &str) -> Option<String> {
    query.split('&').find_map(|pair| {
        let mut it = pair.splitn(2, '=');
        if it.next() == Some(key) {
            let v = it.next()?;
            Some(percent_decode_str(v).decode_utf8_lossy().into_owned())
        } else {
            None
        }
    })
}

fn mime_for(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase()
        .as_str()
    {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        _ => "application/octet-stream",
    }
}

fn resize_cached(path: &Path, width: u32) -> Result<(Vec<u8>, &'static str), Box<dyn std::error::Error>> {
    let cache_dir = crate::config::data_dir().join("ui-cache");
    let _ = std::fs::create_dir_all(&cache_dir);

    let meta = std::fs::metadata(path)?;
    let mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("bin");
    let key = format!(
        "{}-{}-{}-{}",
        path.to_string_lossy().replace(['/', '\\', ':'], "_"),
        width,
        mtime,
        ext
    );
    let cached = cache_dir.join(format!("{}.bin", key));
    if cached.exists() {
        let bytes = std::fs::read(&cached)?;
        let mime = if bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
            "image/png"
        } else {
            "image/jpeg"
        };
        return Ok((bytes, mime));
    }

    let img = image::open(path)?;
    let (w, h) = img.dimensions();
    let scale = width as f32 / w as f32;
    let target_h = ((h as f32 * scale).round() as u32).max(1);
    let resized = img.resize_exact(width, target_h, image::imageops::FilterType::Triangle);
    let has_alpha = resized.color().has_alpha();

    let mut bytes = Vec::new();
    let mime = if has_alpha {
        use image::ImageEncoder;
        image::codecs::png::PngEncoder::new(&mut bytes).write_image(
            resized.as_bytes(),
            resized.width(),
            resized.height(),
            resized.color().into(),
        )?;
        "image/png"
    } else {
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, 86)
            .encode_image(&resized.to_rgb8())?;
        "image/jpeg"
    };

    let _ = std::fs::write(&cached, &bytes);
    Ok((bytes, mime))
}

