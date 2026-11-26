use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const SGDB_DEV_KEY: &str = "15564019b20ad8e03cfcc004128b3b6b";

pub fn data_dir() -> PathBuf {
    dirs::config_dir()
        .or_else(dirs::data_dir)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("auera")
}

pub fn art_dir() -> PathBuf {
    data_dir().join("art")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    Es,
    En,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub lang: Lang,
    pub steamgrid_key: String,
    pub steamgrid_enabled: bool,
    pub protondb_enabled: bool,
    pub gamepad_enabled: bool,
    pub scan_steam: bool,
    pub scan_epic: bool,
    pub scan_heroic: bool,
    pub scan_hydra: bool,
    pub scan_desktop: bool,
    pub watched_folders: Vec<PathBuf>,
    pub window_size: [f32; 2],
}

impl Default for Config {
    fn default() -> Self {
        Self {
            lang: Lang::Es,
            steamgrid_key: SGDB_DEV_KEY.to_string(),
            steamgrid_enabled: true,
            protondb_enabled: true,
            gamepad_enabled: true,
            scan_steam: true,
            scan_epic: true,
            scan_heroic: true,
            scan_hydra: true,
            scan_desktop: true,
            watched_folders: Vec::new(),
            window_size: [1280.0, 800.0],
        }
    }
}

impl Config {
    fn path() -> PathBuf {
        data_dir().join("config.json")
    }

    pub fn load() -> Self {
        std::fs::read_to_string(Self::path())
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let _ = std::fs::create_dir_all(data_dir());
        if let Ok(raw) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(Self::path(), raw);
        }
    }
}
