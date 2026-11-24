use std::path::PathBuf;

use serde_json::Value;

use super::{detect_platform, find_executable, home_dir};
use crate::model::{Game, Platform, Source};

pub fn scan(out: &mut Vec<Game>) {
    for (store, file) in library_files() {
        let Ok(raw) = std::fs::read_to_string(&file) else {
            continue;
        };
        let Ok(value) = serde_json::from_str::<Value>(&raw) else {
            continue;
        };
        for entry in entries(&value) {
            if let Some(game) = read_entry(store, entry) {
                out.push(game);
            }
        }
    }
}

fn entries(value: &Value) -> Vec<&Value> {
    if let Some(list) = value.as_array() {
        return list.iter().collect();
    }
    for key in ["library", "games"] {
        if let Some(list) = value.get(key).and_then(Value::as_array) {
            return list.iter().collect();
        }
    }
    Vec::new()
}

/// Instalación nativa y Flatpak; `store/` (antiguo) y `store_cache/` (actual).
fn library_files() -> Vec<(&'static str, PathBuf)> {
    let mut bases = vec![home_dir().join(".config/heroic")];
    bases.push(home_dir().join(".var/app/com.heroicgameslauncher.hgl/config/heroic"));

    let mut files = Vec::new();
    for base in bases {
        files.push(("gog", base.join("store/gog/library.json")));
        files.push(("epic", base.join("store/epic/library.json")));
        files.push(("amazon", base.join("store/amazon/library.json")));
        files.push(("gog", base.join("store_cache/gog_library.json")));
        files.push(("epic", base.join("store_cache/legendary_library.json")));
        files.push(("amazon", base.join("store_cache/nile_library.json")));
        files.push(("sideload", base.join("sideload_apps/library.json")));
        files.push(("gog", base.join("gog_store/library.json")));
        files.push(("epic", base.join("legendaryConfig/legendary/library.json")));
    }
    files
}

fn read_entry(store: &str, entry: &Value) -> Option<Game> {
    if entry.get("is_installed").and_then(Value::as_bool) == Some(false) {
        return None;
    }
    let title = string_at(entry, &["title", "app_name", "appName"])?;
    let app_id = string_at(entry, &["app_name", "appName", "id"]).unwrap_or_else(|| title.clone());
    let install = entry.get("install");

    let install_dir = path_at(entry, &["install_path", "installPath"])
        .or_else(|| install.and_then(|i| path_at(i, &["install_path", "installPath"])))
        .or_else(|| path_at(entry, &["folder_name"]))
        .filter(|path| path.is_dir())?;

    let executable = install
        .and_then(|i| string_at(i, &["executable"]))
        .or_else(|| string_at(entry, &["executable"]))
        .filter(|exe| !exe.is_empty())
        .map(|exe| {
            let path = PathBuf::from(&exe);
            if path.is_absolute() {
                path
            } else {
                install_dir.join(path)
            }
        })
        .filter(|path| path.is_file())
        .or_else(|| find_executable(&install_dir, &[title.clone()]));

    let mut game = Game::new(
        format!("heroic:{store}:{app_id}"),
        title,
        Source::Heroic,
        install_dir,
    );
    game.platform = install
        .and_then(|i| string_at(i, &["platform"]))
        .map(|platform| {
            if platform.eq_ignore_ascii_case("windows") {
                Platform::Windows
            } else {
                Platform::Linux
            }
        })
        .or_else(|| {
            entry
                .get("is_linux_native")
                .and_then(Value::as_bool)
                .filter(|native| *native)
                .map(|_| Platform::Linux)
        })
        .unwrap_or_else(|| detect_platform(executable.as_ref()));
    game.executable = executable;
    game.playtime_min = entry.get("playtime").and_then(Value::as_u64).unwrap_or(0);

    // Prefijo de Heroic para este juego (nativo o Flatpak).
    if let Some((prefix, proton)) = crate::prefix::heroic_prefix(&app_id) {
        game.prefix = Some(prefix);
        game.proton = proton;
    }
    Some(game)
}

fn string_at(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(|v| v.as_str()))
        .map(str::to_string)
        .filter(|s| !s.is_empty())
}

fn path_at(value: &Value, keys: &[&str]) -> Option<PathBuf> {
    string_at(value, keys).map(PathBuf::from)
}

