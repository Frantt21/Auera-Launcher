use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::{detect_platform, find_executable, home_dir};
use crate::model::{Game, Source};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Manifest {
    #[serde(default)]
    display_name: String,
    #[serde(default)]
    install_location: String,
    #[serde(default)]
    launch_executable: String,
    #[serde(default)]
    app_name: String,
    #[serde(default)]
    install_size: u64,
}

pub fn scan(out: &mut Vec<Game>) {
    for dir in manifest_dirs() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            if entry.path().extension().and_then(|e| e.to_str()) != Some("item") {
                continue;
            }
            if let Some(game) = read_manifest(&entry.path()) {
                out.push(game);
            }
        }
    }
}

fn manifest_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(program_data) = std::env::var("ProgramData") {
        dirs.push(PathBuf::from(program_data).join("Epic/EpicGamesLauncher/Data/Manifests"));
    }
    dirs.push(home_dir().join(".local/share/Epic/EpicGamesLauncher/Data/Manifests"));
    dirs
}

fn read_manifest(path: &Path) -> Option<Game> {
    let raw = std::fs::read_to_string(path).ok()?;
    let manifest: Manifest = serde_json::from_str(&raw).ok()?;
    let install_dir = PathBuf::from(&manifest.install_location);
    if manifest.display_name.is_empty() || manifest.app_name.is_empty() {
        return None;
    }

    let executable = if manifest.launch_executable.is_empty() {
        find_executable(&install_dir, &[manifest.display_name.clone()])
    } else {
        let path = install_dir.join(&manifest.launch_executable);
        if path.is_file() {
            Some(path)
        } else {
            find_executable(&install_dir, &[manifest.display_name.clone()])
        }
    };

    let mut game = Game::new(
        format!("epic:{}", manifest.app_name),
        manifest.display_name,
        Source::Epic,
        install_dir,
    );
    game.platform = detect_platform(executable.as_ref());
    game.executable = executable;
    game.size_bytes = (manifest.install_size > 0).then_some(manifest.install_size);
    Some(game)
}
