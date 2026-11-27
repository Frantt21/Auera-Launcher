use std::path::{Path, PathBuf};

use super::{detect_platform, find_executable, normalize};
use crate::model::{Game, Source};

/// Cada subcarpeta de una carpeta vigilada se trata como un juego candidato.
pub fn scan(folders: &[PathBuf], out: &mut Vec<Game>) {
    for folder in folders {
        let Ok(entries) = std::fs::read_dir(folder) else {
            continue;
        };
        for entry in entries.flatten() {
            let dir = entry.path();
            if !dir.is_dir() {
                continue;
            }
            let title = dir
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            if title.starts_with('.') {
                continue;
            }
            let executable = find_executable(&dir, &[title.clone()]);
            let Some(executable) = executable else {
                continue;
            };
            out.push(build(&dir, &title, executable));
        }

        // Juego suelto en la raíz de la carpeta vigilada.
        if let Some(executable) = find_executable_at_root(folder) {
            let title = folder
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            out.push(build(folder, &title, executable));
        }
    }
}

fn build(dir: &Path, title: &str, executable: PathBuf) -> Game {
    let mut game = Game::new(
        format!("folder:{}", normalize(&dir.to_string_lossy())),
        title.to_string(),
        Source::Manual,
        dir.to_path_buf(),
    );
    game.platform = detect_platform(Some(&executable));
    game.executable = Some(executable);
    game
}

fn find_executable_at_root(dir: &Path) -> Option<PathBuf> {
    std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .find(|path| {
            path.is_file()
                && path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| matches!(e, "exe" | "sh" | "AppImage"))
                    .unwrap_or(false)
        })
}
