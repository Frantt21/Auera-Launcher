use std::collections::HashSet;
use std::path::PathBuf;

use serde_json::Value;

use super::{detect_platform, home_dir};
use crate::model::{Game, Source};

/// Hydra guarda su biblioteca en LevelDB (`hydra-db`), sin formato público.
/// Se leen los JSON de los segmentos y se validan las rutas contra el disco.
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
            for game in read_games(&bytes) {
                if seen.insert(game.id.clone()) {
                    out.push(game);
                }
            }
        }
    }
}

pub fn launcher_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![home_dir().join(".config/hydralauncher")];
    if let Ok(appdata) = std::env::var("APPDATA") {
        dirs.push(PathBuf::from(&appdata).join("hydra"));
        dirs.push(PathBuf::from(appdata).join("hydralauncher"));
    }
    dirs
}

fn data_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    for base in launcher_dirs() {
        dirs.push(base.join("hydra-db"));
        dirs.push(base.join("leveldb"));
    }
    dirs
}

fn read_games(bytes: &[u8]) -> Vec<Game> {
    let text = String::from_utf8_lossy(bytes);
    let mut games = Vec::new();
    for chunk in printable_chunks(&text) {
        if !chunk.contains("\"objectId\"") || !chunk.contains("\"executablePath\"") {
            continue;
        }
        for (index, _) in chunk.match_indices('{') {
            let mut stream = serde_json::Deserializer::from_str(&chunk[index..]).into_iter::<Value>();
            if let Some(Ok(value)) = stream.next() {
                if let Some(game) = to_game(&value) {
                    games.push(game);
                }
            }
        }
    }
    games
}

fn printable_chunks(text: &str) -> impl Iterator<Item = &str> {
    text.split(|c: char| !c.is_ascii_graphic() && c != ' ')
        .filter(|chunk| chunk.len() > 32)
}

fn to_game(value: &Value) -> Option<Game> {
    let title = value.get("title")?.as_str()?.to_string();
    let object_id = value.get("objectId")?.as_str()?.to_string();
    let shop = value.get("shop").and_then(Value::as_str).unwrap_or("hydra");

    let executable = PathBuf::from(value.get("executablePath")?.as_str()?);
    if !executable.is_file() {
        return None;
    }
    let install_dir = executable.parent()?.to_path_buf();

    let mut game = Game::new(
        format!("hydra:{shop}:{object_id}"),
        title,
        Source::Hydra,
        install_dir,
    );
    game.app_id = object_id.parse().ok();
    game.platform = detect_platform(Some(&executable));
    game.executable = Some(executable);
    game.playtime_min = value
        .get("playTimeInMilliseconds")
        .and_then(Value::as_f64)
        .map(|ms| (ms / 60_000.0) as u64)
        .unwrap_or(0);
    game.last_played = value
        .get("lastTimePlayed")
        .and_then(Value::as_str)
        .and_then(parse_timestamp);
    Some(game)
}

/// `2026-11-23T05:43:51.712Z` a segundos UNIX.
fn parse_timestamp(raw: &str) -> Option<i64> {
    let (date, time) = raw.split_once('T')?;
    let mut parts = date.split('-');
    let year: i64 = parts.next()?.parse().ok()?;
    let month: i64 = parts.next()?.parse().ok()?;
    let day: i64 = parts.next()?.parse().ok()?;

    let mut clock = time.split(':');
    let hour: i64 = clock.next()?.parse().ok()?;
    let minute: i64 = clock.next()?.parse().ok()?;
    let second: f64 = clock.next()?.trim_end_matches('Z').parse().ok()?;

    // Días desde 1970 con el algoritmo civil de Howard Hinnant.
    let adjusted_year = year - i64::from(month <= 2);
    let era = adjusted_year.div_euclid(400);
    let year_of_era = adjusted_year - era * 400;
    let shifted_month = (month + 9) % 12;
    let day_of_year = (153 * shifted_month + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let days = era * 146_097 + day_of_era - 719_468;

    Some(days * 86_400 + hour * 3_600 + minute * 60 + second as i64)
}
