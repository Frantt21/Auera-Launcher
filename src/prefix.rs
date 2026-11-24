use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config;
use crate::model::Game;
use crate::scan::steam_roots;

#[derive(Debug, Clone)]
pub struct Proton {
    pub name: String,
    pub dir: PathBuf,
}

impl Proton {
    pub fn script(&self) -> PathBuf {
        self.dir.join("proton")
    }

    /// Raíz de Steam que contiene este Proton (necesaria para lanzar).
    pub fn steam_root(&self) -> Option<PathBuf> {
        steam_roots()
            .into_iter()
            .find(|root| self.dir.starts_with(root))
    }
}

pub fn detect() -> Vec<Proton> {
    let mut found: Vec<Proton> = Vec::new();
    let mut dirs: Vec<PathBuf> = Vec::new();
    for root in steam_roots() {
        dirs.push(root.join("steamapps/common"));
        dirs.push(root.join("compatibilitytools.d"));
    }
    if let Some(home) = dirs::home_dir() {
        dirs.push(home.join(".local/share/Steam/compatibilitytools.d"));
    }

    for dir in dirs {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.join("proton").is_file() {
                continue;
            }
            let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
            if !name.to_lowercase().contains("proton") {
                continue;
            }
            if found.iter().any(|p| p.dir == path) {
                continue;
            }
            found.push(Proton { name, dir: path });
        }
    }
    found.sort_by(|a, b| a.name.cmp(&b.name));
    found
}

/// Prefijo por defecto cuando el juego no viene de un launcher con compatdata.
pub fn default_prefix(game: &Game) -> PathBuf {
    let slug: String = game
        .id
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    config::data_dir().join("prefixes").join(slug).join("pfx")
}

/// Devuelve el compatdata (padre de pfx) que usa Proton.
pub fn compat_dir(prefix: &Path) -> PathBuf {
    if prefix.file_name().map(|n| n == "pfx").unwrap_or(false) {
        prefix.parent().map(Path::to_path_buf).unwrap_or_else(|| prefix.to_path_buf())
    } else {
        prefix.to_path_buf()
    }
}

/// Crea el prefijo si no existe, usando Proton o Wine según lo disponible.
pub fn ensure(game: &mut Game, protons: &[Proton]) -> Result<PathBuf, String> {
    if cfg!(windows) {
        return Err("Los prefijos solo aplican a Linux".into());
    }
    if let Some(prefix) = game.prefix.as_ref().filter(|p| p.is_dir()) {
        return Ok(prefix.clone());
    }

    let prefix = game.prefix.clone().unwrap_or_else(|| default_prefix(game));
    let compat = compat_dir(&prefix);
    std::fs::create_dir_all(&prefix).map_err(|e| e.to_string())?;

    let proton = protons
        .iter()
        .find(|p| Some(&p.name) == game.proton.as_ref())
        .or_else(|| protons.first());

    let status = match proton {
        Some(proton) => {
            let root = proton.steam_root().unwrap_or_else(|| PathBuf::from(""));
            Command::new(proton.script())
                .args(["run", "wineboot", "-u"])
                .env("STEAM_COMPAT_CLIENT_INSTALL_PATH", &root)
                .env("STEAM_COMPAT_DATA_PATH", &compat)
                .env("WINEPREFIX", &prefix)
                .env("WINEDEBUG", "-all")
                .status()
        }
        None => Command::new("wineboot")
            .arg("-u")
            .env("WINEPREFIX", &prefix)
            .env("WINEDEBUG", "-all")
            .status(),
    };

    match status {
        Ok(status) if status.success() => {
            game.prefix = Some(prefix.clone());
            if let Some(proton) = proton {
                game.proton = Some(proton.name.clone());
            }
            Ok(prefix)
        }
        Ok(status) => Err(format!("El prefijo terminó con estado {status}")),
        Err(err) => Err(format!("No se pudo crear el prefijo: {err}")),
    }
}
