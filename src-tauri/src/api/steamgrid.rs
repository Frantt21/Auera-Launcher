use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::Value;

const BASE: &str = "https://www.steamgriddb.com/api/v2";

pub struct Client {
    http: reqwest::blocking::Client,
    key: String,
}

pub struct AssetUrls {
    pub hero: Option<String>,
    pub cover: Option<String>,
    pub logo: Option<String>,
    pub icon: Option<String>,
}

impl Client {
    pub fn new(key: &str) -> Option<Self> {
        Some(Self {
            http: super::client()?,
            key: key.trim().to_string(),
        })
    }

    fn get(&self, path: &str) -> Option<Value> {
        let response = self
            .http
            .get(format!("{BASE}/{path}"))
            .bearer_auth(&self.key)
            .send()
            .ok()?;
        if !response.status().is_success() {
            return None;
        }
        response.json().ok()
    }

    /// Coincidencia directa por appid de Steam (mucho más fiable que el nombre).
    pub fn game_by_steam(&self, app_id: u32) -> Option<u32> {
        let body = self.get(&format!("games/steam/{app_id}"))?;
        body.get("data")?.get("id")?.as_u64().map(|id| id as u32)
    }

    /// Autocompletado por nombre; devuelve (id, nombre).
    pub fn search_games(&self, term: &str) -> Vec<(u32, String)> {
        let encoded: String = urlencode(term);
        let Some(body) = self.get(&format!("search/autocomplete/{encoded}")) else {
            return Vec::new();
        };
        body.get("data")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| {
                        let id = item.get("id")?.as_u64()? as u32;
                        let name = item.get("name")?.as_str()?.to_string();
                        Some((id, name))
                    })
                    .take(8)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Primera URL disponible por tipo de recurso para un id de SteamGridDB.
    pub fn assets(&self, game_id: u32) -> AssetUrls {
        AssetUrls {
            hero: self.first_url(&format!("heroes/game/{game_id}")),
            cover: self.first_url(&format!(
                "grids/game/{game_id}?dimensions=600x900&types=static"
            )),
            logo: self.first_url(&format!("logos/game/{game_id}")),
            // Los iconos por defecto son .ico y el decoder no los soporta.
            icon: self.first_url(&format!("icons/game/{game_id}?mimes=image/png")),
        }
    }

    fn first_url(&self, path: &str) -> Option<String> {
        let body = self.get(path)?;
        body.get("data")?.as_array()?.iter().find_map(|asset| {
            asset
                .get("url")?
                .as_str()
                .filter(|url| !url.is_empty())
                .map(str::to_string)
        })
    }

    pub fn download(&self, url: &str, target: &Path) -> Option<()> {
        let bytes = self.http.get(url).send().ok()?.bytes().ok()?;
        std::fs::create_dir_all(target.parent()?).ok()?;
        std::fs::write(target, &bytes).ok()
    }
}

pub fn cache_dir(game_id: &str) -> PathBuf {
    crate::config::art_dir().join(sanitize(game_id))
}

fn sanitize(value: &str) -> String {
    value
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

fn urlencode(value: &str) -> String {
    value
        .chars()
        .flat_map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => vec![c],
            ' ' => vec!['%', '2', '0'],
            other => format!("%{:02X}", other as u32).chars().collect(),
        })
        .collect()
}

/// Descarga el arte que falte para un juego y devuelve las rutas locales
/// junto al id de SteamGridDB resuelto (para no repetir la búsqueda).
pub fn fetch_assets(
    client: &Client,
    game_id: &str,
    game: &crate::model::Game,
    force: bool,
) -> (crate::model::Art, Option<u32>) {
    let mut art = game.art.clone();
    if !force && !art.needs_download() {
        return (art, game.sgdb_id);
    }

    let sgdb_id = match game.sgdb_id {
        Some(id) => Some(id),
        None => game
            .app_id
            .and_then(|app_id| client.game_by_steam(app_id))
            .or_else(|| {
                client
                    .search_games(&game.title)
                    .into_iter()
                    .next()
                    .map(|(id, _)| id)
            }),
    };
    let Some(sgdb_id) = sgdb_id else {
        return (art, None);
    };
    // Límite público de la API: 2 peticiones por segundo.
    std::thread::sleep(Duration::from_millis(350));

    let urls = client.assets(sgdb_id);
    let dir = cache_dir(game_id);
    let store = |url: Option<String>, name: &str, current: &mut Option<PathBuf>| {
        let Some(url) = url else {
            return;
        };
        let downloaded = current.as_ref().map(|path| path.is_file()).unwrap_or(false);
        if !force && downloaded {
            return;
        }
        let extension = if url.contains(".png") { "png" } else { "jpg" };
        let target = dir.join(format!("{name}.{extension}"));
        if client.download(&url, &target).is_some() {
            *current = Some(target);
        }
    };

    store(urls.hero, "hero", &mut art.hero);
    store(urls.cover, "cover", &mut art.cover);
    store(urls.logo, "logo", &mut art.logo);
    store(urls.icon, "icon", &mut art.icon);
    (art, Some(sgdb_id))
}
