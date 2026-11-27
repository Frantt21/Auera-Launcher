pub mod desktop;
pub mod epic;
pub mod folders;
pub mod heroic;
pub mod hydra;
pub mod steam;

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::model::{Game, Platform};

const SKIP_DIRS: [&str; 13] = [
    "redist",
    "_commonredist",
    "redistributables",
    "vcredist",
    "directx",
    "dotnet",
    "support",
    "tools",
    "installer",
    "installers",
    "crashreport",
    "crashreports",
    "engine",
];

const SKIP_EXES: [&str; 12] = [
    "unins",
    "dxsetup",
    "dxwebsetup",
    "vcredist",
    "ue4prereqsetup",
    "ue5prereqsetup",
    "crashreport",
    "unitycrashhandler",
    "dotnetfx",
    "oalinst",
    "setup",
    "install",
];

pub fn scan_all(cfg: &Config) -> Vec<Game> {
    let mut games = Vec::new();
    if cfg.scan_steam {
        steam::scan(&mut games);
    }
    if cfg.scan_epic {
        epic::scan(&mut games);
    }
    if cfg.scan_heroic {
        heroic::scan(&mut games);
    }
    if cfg.scan_hydra {
        hydra::scan(&mut games);
    }
    if cfg.scan_desktop {
        desktop::scan(&mut games);
    }
    folders::scan(&cfg.watched_folders, &mut games);

    let mut seen = HashSet::new();
    games.retain(|g| seen.insert(g.install_dir.clone()));
    games.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
    games
}

pub fn home_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"))
}

pub fn steam_roots() -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();
    if let Ok(path) = std::env::var("STEAM_COMPAT_CLIENT_INSTALL_PATH") {
        roots.push(PathBuf::from(path));
    }
    let home = home_dir();
    roots.push(home.join(".steam/steam"));
    roots.push(home.join(".steam/root"));
    roots.push(home.join(".local/share/Steam"));
    roots.push(home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam"));
    if let Ok(pf) = std::env::var("ProgramFiles(x86)") {
        roots.push(PathBuf::from(pf).join("Steam"));
    }
    if let Ok(pf) = std::env::var("ProgramFiles") {
        roots.push(PathBuf::from(pf).join("Steam"));
    }
    roots.retain(|p| p.join("steamapps").is_dir());
    roots.dedup();
    roots
}

/// Busca el ejecutable principal de un juego descartando redistribuibles.
pub fn find_executable(dir: &Path, hints: &[String]) -> Option<PathBuf> {
    let mut candidates = Vec::new();
    collect_executables(dir, 0, &mut candidates);

    let mut best: Option<(i32, u64, PathBuf)> = None;
    for path in candidates {
        let stem = normalize(&path.file_stem().unwrap_or_default().to_string_lossy());
        let name = normalize(&path.file_name().unwrap_or_default().to_string_lossy());
        if SKIP_EXES
            .iter()
            .any(|skip| stem.starts_with(skip) || name.contains(skip))
        {
            continue;
        }
        let mut score = 0i32;
        for hint in hints {
            let hint = normalize(hint);
            if hint.is_empty() {
                continue;
            }
            if stem == hint {
                score += 1000;
            } else if stem.contains(&hint) || hint.contains(&stem) {
                score += 300;
            } else if hint
                .split_whitespace()
                .any(|word| word.len() > 3 && stem.contains(word))
            {
                score += 120;
            }
        }
        let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        let rank = score * 1_000_000 + (size.min(999_999) as i32);
        if best.as_ref().map(|(r, _, _)| rank > *r).unwrap_or(true) {
            best = Some((rank, size, path));
        }
    }
    best.map(|(_, _, path)| path)
}

fn collect_executables(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    if depth > 3 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_lowercase();
        if path.is_dir() {
            if !SKIP_DIRS.contains(&name.as_str()) && !name.starts_with('.') {
                collect_executables(&path, depth + 1, out);
            }
            continue;
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default();
        if matches!(ext, "exe" | "sh" | "appimage" | "AppImage") && is_runnable(&path, ext) {
            out.push(path);
        }
    }
}

fn is_runnable(path: &Path, ext: &str) -> bool {
    if ext.eq_ignore_ascii_case("exe") {
        return true;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = std::fs::metadata(path) {
            return meta.permissions().mode() & 0o111 != 0;
        }
    }
    let _ = ext;
    false
}

pub fn detect_platform(executable: Option<&PathBuf>) -> Platform {
    match executable {
        Some(path) => Platform::from_executable(path),
        None => Platform::current(),
    }
}

pub fn normalize(value: &str) -> String {
    value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

pub fn dir_size(path: &Path) -> u64 {
    let Ok(entries) = std::fs::read_dir(path) else {
        return 0;
    };
    entries
        .flatten()
        .map(|entry| {
            let path = entry.path();
            let meta = match entry.metadata() {
                Ok(meta) => meta,
                Err(_) => return 0,
            };
            if meta.is_dir() {
                dir_size(&path)
            } else {
                meta.len()
            }
        })
        .sum()
}
