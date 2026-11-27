use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::config;
use crate::model::{Platform, Source};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Sort {
    Name,
    #[default]
    Recent,
    Playtime,
    Size,
}

/// Estado de la biblioteca (filtros y orden) que sobrevive al cierre.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Prefs {
    pub sort: Sort,
    pub source: Option<Source>,
    pub platform: Option<Platform>,
    pub favorites_only: bool,
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            sort: Sort::Recent,
            source: None,
            platform: None,
            favorites_only: false,
        }
    }
}

impl Prefs {
    fn path() -> PathBuf {
        config::data_dir().join("prefs.json")
    }

    pub fn load() -> Self {
        std::fs::read_to_string(Self::path())
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let _ = std::fs::create_dir_all(config::data_dir());
        if let Ok(raw) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(Self::path(), raw);
        }
    }
}
