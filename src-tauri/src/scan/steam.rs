use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::{detect_platform, find_executable, steam_roots};
use crate::model::{Game, Source};
use crate::vdf;

pub fn scan(out: &mut Vec<Game>) {
    for root in steam_roots() {
        let playtimes = playtime_map(&root);
        for library in libraries(&root) {
            let steamapps = library.join("steamapps");
            let Ok(entries) = std::fs::read_dir(&steamapps) else {
                continue;
            };
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_lowercase();
                if !(name.starts_with("appmanifest_") && name.ends_with(".acf")) {
                    continue;
                }
                if let Some(game) = read_manifest(&entry.path(), &steamapps, &playtimes) {
                    out.push(game);
                }
            }
        }
    }
}

fn read_manifest(path: &Path, steamapps: &Path, playtimes: &HashMap<u32, u64>) -> Option<Game> {
    let raw = std::fs::read_to_string(path).ok()?;
    let node = vdf::parse(&raw)?;
    let state = node.map("AppState")?;

    let app_id: u32 = state.str("appid")?.parse().ok().or_else(|| {
        path.file_stem()
            .and_then(|s| s.to_str())
            .and_then(|s| s.rsplit('_').next())
            .and_then(|s| s.parse().ok())
    })?;
    let title = state.str("name")?.to_string();
    if is_tool(&title) {
        return None;
    }
    let installdir = state.str("installdir")?;
    let install_path = steamapps.join("common").join(installdir);
    if !install_path.is_dir() {
        return Some(Game::new(
            format!("steam:{app_id}"),
            title,
            Source::Steam,
            install_path,
        ));
    }

    let hints = vec![title.clone(), installdir.to_string()];
    let executable = find_executable(&install_path, &hints);
    let mut game = Game::new(
        format!("steam:{app_id}"),
        title,
        Source::Steam,
        install_path.clone(),
    );
    game.app_id = Some(app_id);
    game.platform = detect_platform(executable.as_ref());
    game.platform_locked = false;
    game.executable = executable;
    game.size_bytes = state.u64("SizeOnDisk").filter(|v| *v > 0);
    game.last_played = state.u64("LastPlayed").filter(|v| *v > 0).map(|v| v as i64);
    game.playtime_min = playtimes.get(&app_id).copied().unwrap_or(0);

    let compatdata = steamapps.join("compatdata").join(app_id.to_string());
    let prefix = compatdata.join("pfx");
    if prefix.is_dir() {
        game.prefix = Some(prefix.clone());
        game.proton = prefix
            .parent()
            .map(|dir| dir.join("version"))
            .and_then(|file| std::fs::read_to_string(file).ok())
            .map(|v| v.trim().trim_start_matches("proton_").to_string())
            .filter(|v| !v.is_empty());
    }
    Some(game)
}

/// Proton, runtimes y redistribuibles aparecen como apps pero no son juegos.
fn is_tool(title: &str) -> bool {
    let lower = title.to_lowercase();
    lower.starts_with("proton")
        || lower.starts_with("steamvr")
        || lower.contains("steam linux runtime")
        || lower.contains("steamworks common redistributable")
}

/// Todas las bibliotecas declaradas en libraryfolders.vdf más la raíz.
pub fn libraries(root: &Path) -> Vec<PathBuf> {
    let mut libraries = vec![root.to_path_buf()];
    let vdf_path = root.join("steamapps/libraryfolders.vdf");
    if let Ok(raw) = std::fs::read_to_string(vdf_path) {
        if let Some(node) = vdf::parse(&raw) {
            if let Some(folders) = node.map("libraryfolders") {
                for (key, value) in folders.iter() {
                    if key == "contentstatsid" {
                        continue;
                    }
                    // Formato nuevo (mapa con "path") y antiguo (clave -> ruta).
                    if let Some(path) = value.str("path") {
                        libraries.push(PathBuf::from(path));
                    } else if let crate::vdf::Node::Value(path) = value {
                        libraries.push(PathBuf::from(path));
                    }
                }
            }
        }
    }
    libraries.retain(|p| p.join("steamapps").is_dir());
    libraries.dedup();
    libraries
}

/// Tiempo jugado por appid leído de userdata/<id>/config/localconfig.vdf.
fn playtime_map(root: &Path) -> HashMap<u32, u64> {
    let mut map = HashMap::new();
    let Ok(users) = std::fs::read_dir(root.join("userdata")) else {
        return map;
    };
    for user in users.flatten() {
        let file = user.path().join("config/localconfig.vdf");
        let Ok(raw) = std::fs::read_to_string(file) else {
            continue;
        };
        let Some(node) = vdf::parse(&raw) else {
            continue;
        };
        let Some(store) = node.map("UserLocalConfigStore") else {
            continue;
        };
        let Some(apps) = store
            .map("Software")
            .and_then(|s| s.map("Valve"))
            .and_then(|v| v.map("Steam"))
            .and_then(|s| s.map("apps"))
        else {
            continue;
        };
        for (app_id, app) in apps.iter() {
            let crate::vdf::Node::Map(app) = app else {
                continue;
            };
            if let (Ok(id), Some(minutes)) = (app_id.parse::<u32>(), app.u64("Playtime")) {
                map.insert(id, minutes);
            }
        }
    }
    map
}
