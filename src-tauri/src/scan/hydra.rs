use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use serde_json::Value;

use super::{detect_platform, home_dir};
use crate::model::{Game, Source};

/// Juego instalado según la base de Hydra. Los prefijos se resuelven aparte,
/// porque Hydra puede apuntar a un compatdata de Steam o a un prefijo propio.
#[derive(Debug, Clone)]
pub struct HydraGame {
    pub object_id: String,
    pub shop: String,
    pub title: String,
    pub executable: PathBuf,
    pub wine_prefix: Option<PathBuf>,
    pub proton_path: Option<PathBuf>,
    pub steam_shortcut: Option<u32>,
    pub playtime_min: u64,
    pub last_played: Option<i64>,
}

/// Hydra guarda su biblioteca en LevelDB (`hydra-db`), sin formato público.
/// Se leen los JSON de los segmentos y se validan las rutas contra el disco.
pub fn scan(out: &mut Vec<Game>) {
    for entry in installed() {
        out.push(to_game(entry));
    }
}

/// Entradas instaladas, con caché por huella de la base: la detección de
/// prefijos las consulta juego a juego.
pub fn installed() -> Vec<HydraGame> {
    static CACHE: OnceLock<Mutex<(u64, Vec<HydraGame>)>> = OnceLock::new();
    let stamp = fingerprint();
    let cache = CACHE.get_or_init(|| Mutex::new((0, Vec::new())));
    let mut guard = cache.lock().unwrap();
    if guard.0 == stamp && !guard.1.is_empty() {
        return guard.1.clone();
    }
    let games = read_all();
    *guard = (stamp, games.clone());
    games
}

pub fn launcher_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![home_dir().join(".config/hydralauncher")];
    if let Some(data) = dirs::data_dir() {
        dirs.push(data.join("hydralauncher"));
    }
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

fn db_files() -> Vec<PathBuf> {
    let mut files = Vec::new();
    for root in data_dirs() {
        let Ok(entries) = std::fs::read_dir(&root) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or_default();
            if matches!(ext, "ldb" | "log") {
                files.push(path);
            }
        }
    }
    files
}

/// Suma de tamaño y fecha de los segmentos: si Hydra instala algo, cambia.
fn fingerprint() -> u64 {
    db_files()
        .iter()
        .filter_map(|path| std::fs::metadata(path).ok())
        .map(|meta| {
            let modified = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            meta.len() ^ modified
        })
        .fold(0, u64::wrapping_add)
}

fn read_all() -> Vec<HydraGame> {
    let mut found = Vec::new();
    let mut seen = HashSet::new();
    for path in db_files() {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        for game in read_games(&bytes) {
            if seen.insert(game.object_id.clone()) {
                found.push(game);
            }
        }
    }
    found
}

/// Los juegos están serializados empezando por `canonicalId`, así que se parsea
/// desde esa marca. Trocear los segmentos por bytes no ASCII descartaba
/// cualquier título con acentos o símbolos.
fn read_games(bytes: &[u8]) -> Vec<HydraGame> {
    const MARKER: &str = "{\"canonicalId\"";
    const MAX_OBJECT: usize = 16 * 1024;

    let text = String::from_utf8_lossy(bytes);
    let mut games = Vec::new();
    let mut search = 0;
    while let Some(found) = text[search..].find(MARKER) {
        let start = search + found;
        search = start + MARKER.len();
        let end = (start + MAX_OBJECT).min(text.len());
        let slice = text.get(start..end).unwrap_or(&text[start..]);
        let mut stream = serde_json::Deserializer::from_str(slice).into_iter::<Value>();
        if let Some(Ok(value)) = stream.next() {
            if let Some(game) = to_entry(&value) {
                games.push(game);
            }
        }
    }
    games
}

fn to_entry(value: &Value) -> Option<HydraGame> {
    let title = value.get("title")?.as_str()?.to_string();
    let object_id = value.get("objectId")?.as_str()?.to_string();
    let shop = value.get("shop").and_then(Value::as_str).unwrap_or("hydra");

    let executable = PathBuf::from(value.get("executablePath")?.as_str()?);
    if !executable.is_file() {
        return None;
    }

    Some(HydraGame {
        wine_prefix: path_field(value, "winePrefixPath").filter(|p| p.is_dir()),
        proton_path: path_field(value, "protonPath"),
        steam_shortcut: value
            .get("steamShortcutAppId")
            .and_then(Value::as_u64)
            .and_then(|id| u32::try_from(id).ok()),
        object_id,
        shop: shop.to_string(),
        title,
        executable,
        playtime_min: value
            .get("playTimeInMilliseconds")
            .and_then(Value::as_f64)
            .map(|ms| (ms / 60_000.0) as u64)
            .unwrap_or(0),
        last_played: value
            .get("lastTimePlayed")
            .and_then(Value::as_str)
            .and_then(parse_timestamp),
    })
}

fn path_field(value: &Value, key: &str) -> Option<PathBuf> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(PathBuf::from)
}

fn to_game(entry: HydraGame) -> Game {
    let install_dir = entry
        .executable
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default();

    let mut game = Game::new(
        format!("hydra:{}:{}", entry.shop, entry.object_id),
        entry.title,
        Source::Hydra,
        install_dir,
    );
    game.app_id = entry.object_id.parse().ok();
    game.platform = detect_platform(Some(&entry.executable));
    game.executable = Some(entry.executable);
    game.playtime_min = entry.playtime_min;
    game.last_played = entry.last_played;
    game
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

