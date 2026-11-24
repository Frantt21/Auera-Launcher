use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::config;
use crate::model::{Game, Source};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Library {
    pub games: Vec<Game>,
    #[serde(skip)]
    path: PathBuf,
}

impl Library {
    pub fn load() -> Self {
        let path = config::data_dir().join("library.json");
        let games: Vec<Game> = std::fs::read_to_string(&path)
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default();
        let mut library = Self { games, path };
        library
            .games
            .sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
        library
    }

    pub fn save(&self) {
        let _ = std::fs::create_dir_all(config::data_dir());
        if let Ok(raw) = serde_json::to_string_pretty(&self.games) {
            let _ = std::fs::write(&self.path, raw);
        }
    }

    pub fn get(&self, id: &str) -> Option<&Game> {
        self.games.iter().find(|g| g.id == id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut Game> {
        self.games.iter_mut().find(|g| g.id == id)
    }

    pub fn remove(&mut self, id: &str) {
        self.games.retain(|g| g.id != id);
        self.save();
    }

    /// Fusiona un escaneo con la biblioteca existente conservando los ajustes manuales.
    pub fn merge(&mut self, scanned: Vec<Game>) -> usize {
        let mut added = 0;
        for fresh in scanned {
            match self.games.iter_mut().find(|g| g.id == fresh.id) {
                Some(existing) => {
                    existing.title = fresh.title;
                    existing.executable = fresh.executable.or(existing.executable.clone());
                    existing.install_dir = fresh.install_dir;
                    if !existing.platform_locked {
                        existing.platform = fresh.platform;
                    }
                    if existing.prefix.is_none() {
                        existing.prefix = fresh.prefix;
                    }
                    if existing.proton.is_none() {
                        existing.proton = fresh.proton;
                    }
                    if fresh.size_bytes.is_some() {
                        existing.size_bytes = fresh.size_bytes;
                    }
                    if fresh.playtime_min > existing.playtime_min {
                        existing.playtime_min = fresh.playtime_min;
                    }
                    if fresh.last_played > existing.last_played {
                        existing.last_played = fresh.last_played;
                    }
                    if existing.app_id.is_none() {
                        existing.app_id = fresh.app_id;
                    }
                }
                None => {
                    self.games.push(fresh);
                    added += 1;
                }
            }
        }
        self.games.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
        self.save();
        added
    }

    pub fn count_by_source(&self, source: Source) -> usize {
        self.games.iter().filter(|g| g.source == source).count()
    }
}
