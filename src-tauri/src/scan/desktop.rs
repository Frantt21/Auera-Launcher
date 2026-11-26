use std::collections::HashMap;
use std::path::PathBuf;

use super::{detect_platform, home_dir, normalize};
use crate::model::{Game, Source};

pub fn scan(out: &mut Vec<Game>) {
    let dirs = [
        home_dir().join(".local/share/applications"),
        PathBuf::from("/usr/share/applications"),
    ];
    for dir in dirs {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("desktop") {
                continue;
            }
            if let Some(game) = read_entry(&path) {
                out.push(game);
            }
        }
    }
}

fn read_entry(path: &std::path::Path) -> Option<Game> {
    let raw = std::fs::read_to_string(path).ok()?;
    let fields = parse_desktop(&raw);

    let categories = fields
        .get("Categories")
        .map(String::as_str)
        .unwrap_or_default();
    if !categories.split(';').any(|c| c == "Game") || fields.contains_key("NoDisplay") {
        return None;
    }

    let exec = fields.get("Exec")?.clone();
    let program = exec.split_whitespace().next()?.to_string();
    if !program.starts_with('/') {
        return None;
    }
    let executable = PathBuf::from(&program);
    if !executable.is_file() {
        return None;
    }

    let install_dir = executable.parent()?.to_path_buf();
    let title = fields.get("Name").cloned().unwrap_or_else(|| {
        executable
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into()
    });
    // Los propios launchers también publican accesos .desktop.
    if is_launcher(&title, &executable) {
        return None;
    }

    let mut game = Game::new(
        format!(
            "desktop:{}",
            normalize(&format!("{title}{}", install_dir.display()))
        ),
        title,
        Source::Desktop,
        install_dir,
    );
    game.platform = detect_platform(Some(&executable));
    game.executable = Some(executable);
    Some(game)
}

fn is_launcher(title: &str, executable: &std::path::Path) -> bool {
    const LAUNCHERS: [&str; 7] = [
        "steam", "heroic", "lutris", "bottles", "hydra", "playnite", "itgm",
    ];
    let name = title.to_lowercase();
    let binary = executable
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    LAUNCHERS
        .iter()
        .any(|launcher| name.contains(launcher) || binary.contains(launcher))
}

fn parse_desktop(raw: &str) -> HashMap<String, String> {
    let mut fields = HashMap::new();
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('[') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            fields
                .entry(key.trim().to_string())
                .or_insert_with(|| value.trim().to_string());
        }
    }
    fields
}
