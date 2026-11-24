use std::path::{Path, PathBuf};
use std::process::{Child, Command};

use crate::model::{Game, Platform};
use crate::prefix::{self, Proton};

pub struct Plan {
    program: PathBuf,
    args: Vec<String>,
    cwd: PathBuf,
    env: Vec<(String, String)>,
}

impl Plan {
    pub fn spawn(&self) -> Result<Child, String> {
        let mut command = Command::new(&self.program);
        command.args(&self.args).current_dir(&self.cwd);
        for (key, value) in &self.env {
            command.env(key, value);
        }
        command.spawn().map_err(|e| e.to_string())
    }
}

/// Construye el comando de arranque, usando Proton para juegos de Windows en Linux.
pub fn plan(game: &Game, protons: &[Proton]) -> Result<Plan, String> {
    let executable = game.executable.clone().ok_or("El juego no tiene ejecutable")?;
    let cwd = executable.parent().unwrap_or(&game.install_dir).to_path_buf();

    if cfg!(windows) || game.platform == Platform::Linux {
        return native(executable, cwd);
    }

    let proton = protons
        .iter()
        .find(|p| Some(&p.name) == game.proton.as_ref())
        .or_else(|| protons.first())
        .ok_or("No se encontró Proton instalado")?;

    let prefix = game
        .prefix
        .clone()
        .filter(|p| p.exists())
        .unwrap_or_else(|| prefix::default_prefix(game));
    let compat = prefix::compat_dir(&prefix);
    let steam_root = proton.steam_root().unwrap_or_default();

    Ok(Plan {
        program: proton.script(),
        args: vec![
            "run".into(),
            executable.to_string_lossy().to_string(),
        ],
        cwd,
        env: vec![
            ("STEAM_COMPAT_CLIENT_INSTALL_PATH".into(), steam_root.to_string_lossy().to_string()),
            ("STEAM_COMPAT_DATA_PATH".into(), compat.to_string_lossy().to_string()),
            ("WINEPREFIX".into(), prefix.to_string_lossy().to_string()),
            ("WINEDEBUG".into(), "-all".into()),
        ],
    })
}

fn native(executable: PathBuf, cwd: PathBuf) -> Result<Plan, String> {
    if !executable.exists() {
        return Err(format!("No existe {}", executable.display()));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if executable.extension().and_then(|e| e.to_str()) == Some("sh") {
            let meta = std::fs::metadata(&executable).map_err(|e| e.to_string())?;
            if meta.permissions().mode() & 0o111 == 0 {
                return Ok(Plan {
                    program: PathBuf::from("/bin/sh"),
                    args: vec![executable.to_string_lossy().to_string()],
                    cwd,
                    env: Vec::new(),
                });
            }
        }
    }
    Ok(Plan {
        program: executable,
        args: Vec::new(),
        cwd,
        env: Vec::new(),
    })
}

pub fn open_in_file_manager(path: &Path) {
    let program = if cfg!(windows) { "explorer" } else { "xdg-open" };
    let _ = Command::new(program).arg(path).spawn();
}
