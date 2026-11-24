use std::collections::HashSet;
use std::path::PathBuf;

use super::{detect_platform, home_dir};
use crate::model::{Game, Source};

/// Hydra guarda su biblioteca en LevelDB, sin formato público. Se extraen las
/// rutas de ejecutables de los segmentos y se validan contra el disco.
pub fn scan(out: &mut Vec<Game>) {
    let mut seen = HashSet::new();
    for root in data_dirs() {
        let Ok(entries) = std::fs::read_dir(&root) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or_default();
            if !matches!(ext, "ldb" | "log") {
                continue;
            }
            let Ok(bytes) = std::fs::read(&path) else {
                continue;
            };
            for candidate in extract_windows_paths(&bytes) {
                if !seen.insert(candidate.clone()) || !candidate.is_file() {
                    continue;
                }
                let Some(install_dir) = candidate.parent().map(PathBuf::from) else {
                    continue;
                };
                let title = candidate
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .replace(['_', '-'], " ");
                let mut game = Game::new(
                    format!("hydra:{}", super::normalize(&candidate.to_string_lossy())),
                    title,
                    Source::Hydra,
                    install_dir,
                );
                game.platform = detect_platform(Some(&candidate));
                game.executable = Some(candidate);
                out.push(game);
            }
        }
    }
}

fn data_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(appdata) = std::env::var("APPDATA") {
        dirs.push(PathBuf::from(appdata).join("hydra/leveldb"));
    }
    dirs.push(home_dir().join(".config/hydra/leveldb"));
    dirs
}

/// Busca secuencias imprimibles que parezcan rutas absolutas a un ejecutable.
fn extract_windows_paths(bytes: &[u8]) -> Vec<PathBuf> {
    const MAX: usize = 300;
    let mut found = Vec::new();
    let mut current = String::new();
    for &byte in bytes {
        let printable = byte.is_ascii_graphic() || byte == b' ';
        if printable {
            current.push(byte as char);
            if current.len() > MAX {
                current.clear();
            }
            continue;
        }
        push_candidate(&current, &mut found);
        current.clear();
    }
    push_candidate(&current, &mut found);
    found
}

fn push_candidate(raw: &str, found: &mut Vec<PathBuf>) {
    let lower = raw.to_ascii_lowercase();
    for ext in [".exe", ".appimage"] {
        let Some(index) = lower.find(ext) else {
            continue;
        };
        let end = index + ext.len();
        let path = &raw[..end];
        let start = if cfg!(windows) {
            path.rfind(":\\").map(|i| i.saturating_sub(1)).unwrap_or(0)
        } else {
            0
        };
        let candidate = &path[start..];
        if candidate.len() > 4 && candidate.contains(['/', '\\']) {
            found.push(PathBuf::from(candidate));
        }
    }
}
