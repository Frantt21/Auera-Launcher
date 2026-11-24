use std::path::PathBuf;

use serde_json::Value;

use super::{detect_platform, home_dir};
use crate::model::{Game, Source};

pub fn scan(out: &mut Vec<Game>) {
    for (store, file) in library_files() {
        let Ok(raw) = std::fs::read_to_string(&file) else {
            continue;
        };
        let Ok(entries) = serde_json::from_str::<Vec<Value>>(&raw) else {
            continue;
        };
        for entry in entries {
            if let Some(game) = read_entry(store, &entry) {
                out.push(game);
            }
        }
    }
}

fn library_files() -> Vec<(&'static str, PathBuf)> {
    let base = home_dir().join(".config/heroic");
    vec![
        ("gog", base.join("store/gog/library.json")),
        ("epic", base.join("store/epic/library.json")),
        ("amazon", base.join("store/amazon/library.json")),
        ("gog", base.join("gog_store/library.json")),
        ("epic", base.join("legendaryConfig/legendary/library.json")),
    ]
}

fn read_entry(store: &str, entry: &Value) -> Option<Game> {
    let title = string_at(entry, &["title", "app_name", "appName"])?;
    let app_id = string_at(entry, &["app_name", "appName", "id"]).unwrap_or_else(|| title.clone());
    let install = entry.get("install");

    let install_dir = path_at(entry, &["install_path", "installPath", "folder_name"])
        .or_else(|| install.and_then(|i| path_at(i, &["install_path", "installPath"])))?;
    if !install_dir.is_dir() {
        return None;
    }

    let executable = install
        .and_then(|i| string_at(i, &["executable"]))
        .filter(|exe| !exe.is_empty())
        .map(|exe| install_dir.join(exe))
        .filter(|path| path.is_file());

    let mut game = Game::new(
        format!("heroic:{store}:{app_id}"),
        title,
        Source::Heroic,
        install_dir.clone(),
    );
    game.platform = install
        .and_then(|i| string_at(i, &["platform"]))
        .map(|p| {
            if p.eq_ignore_ascii_case("windows") {
                crate::model::Platform::Windows
            } else {
                crate::model::Platform::Linux
            }
        })
        .unwrap_or_else(|| detect_platform(executable.as_ref()));
    game.executable = executable;
    game.playtime_min = entry
        .get("playtime")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    // Configuración de Wine específica del juego en GamesConfig/<app>.json.
    if let Some(config) = games_config(&app_id) {
        game.prefix = path_at(&config, &["winePrefix"]);
        game.proton = config
            .get("wineVersion")
            .and_then(|v| string_at(v, &["name"]));
    }
    Some(game)
}

fn games_config(app_id: &str) -> Option<Value> {
    let dir = home_dir().join(".config/heroic/GamesConfig");
    let raw = std::fs::read_to_string(dir.join(format!("{app_id}.json"))).ok()?;
    let value: Value = serde_json::from_str(&raw).ok()?;
    let map = value.as_object()?;
    // El archivo envuelve la configuración bajo el app_name.
    map.values().next().cloned()
}

fn string_at(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(|v| v.as_str()))
        .map(str::to_string)
        .filter(|s| !s.is_empty())
}

fn path_at(value: &Value, keys: &[&str]) -> Option<PathBuf> {
    string_at(value, keys)
        .map(PathBuf::from)
        .filter(|path| path.exists())
}
