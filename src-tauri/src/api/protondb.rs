use std::collections::HashMap;

use serde_json::Value;

use crate::config;
use crate::model::Tier;

const SUMMARY_URL: &str = "https://www.protondb.com/api/v1/reports/summaries";

type Cache = HashMap<u32, Tier>;

fn cache_path() -> std::path::PathBuf {
    config::data_dir().join("protondb.json")
}

pub fn load_cache() -> Cache {
    std::fs::read_to_string(cache_path())
        .ok()
        .and_then(|raw| serde_json::from_str::<HashMap<String, String>>(&raw).ok())
        .map(|map| {
            map.into_iter()
                .filter_map(|(key, tier)| Some((key.parse().ok()?, Tier::parse(&tier))))
                .collect()
        })
        .unwrap_or_default()
}

pub fn save_cache(cache: &Cache) {
    let serializable: HashMap<String, &str> = cache
        .iter()
        .map(|(id, tier)| (id.to_string(), tier.label()))
        .collect();
    let _ = std::fs::create_dir_all(config::data_dir());
    if let Ok(raw) = serde_json::to_string(&serializable) {
        let _ = std::fs::write(cache_path(), raw);
    }
}

/// Devuelve el tier de un appid de Steam consultando la API pública.
pub fn fetch(app_id: u32) -> Option<Tier> {
    let response = super::client()?
        .get(format!("{SUMMARY_URL}/{app_id}.json"))
        .header("Referer", "https://www.protondb.com/")
        .send()
        .ok()?;
    if !response.status().is_success() {
        return None;
    }
    let body: Value = response.json().ok()?;
    let tier = body
        .get("trendingTier")
        .or_else(|| body.get("tier"))
        .and_then(Value::as_str)?;
    Some(Tier::parse(tier))
}
