use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config;
use crate::model::{Game, Platform, Source};
use crate::scan::{steam, steam_roots};

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
    /// Para instalaciones del sistema se usa la primera raíz disponible.
    pub fn steam_root(&self) -> Option<PathBuf> {
        let roots = steam_roots();
        roots
            .iter()
            .find(|root| self.dir.starts_with(root))
            .cloned()
            .or_else(|| roots.into_iter().next())
    }
}

pub fn detect_protons() -> Vec<Proton> {
    let mut found: Vec<Proton> = Vec::new();
    let mut dirs: Vec<PathBuf> = Vec::new();
    for root in steam_roots() {
        dirs.push(root.join("steamapps/common"));
        dirs.push(root.join("compatibilitytools.d"));
    }
    if let Some(home) = dirs::home_dir() {
        dirs.push(home.join(".local/share/Steam/compatibilitytools.d"));
    }
    // Instalaciones de Proton del sistema (paquetes de distro, CachyOS, etc.).
    dirs.push(PathBuf::from("/usr/share/steam/compatibilitytools.d"));
    dirs.push(PathBuf::from("/usr/local/share/steam/compatibilitytools.d"));

    for dir in dirs {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.join("proton").is_file() {
                continue;
            }
            let dir_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
            if !dir_name.to_lowercase().contains("proton") {
                continue;
            }
            if found.iter().any(|p| p.dir == path) {
                continue;
            }
            found.push(Proton {
                name: proton_name(&path, &dir_name),
                dir: path,
            });
        }
    }
    found.sort_by(|a, b| a.name.cmp(&b.name));
    found
}

/// Nombre visible de un Proton: `version` del propio tool si existe.
fn proton_name(dir: &Path, dir_name: &str) -> String {
    let Ok(text) = std::fs::read_to_string(dir.join("version")) else {
        return dir_name.to_string();
    };
    let line = text.lines().next().unwrap_or_default().trim();
    // Algunos paquetes anteponen un timestamp: "1792530497 cachyos-11.0-...".
    let label = line.split_whitespace().last().unwrap_or_default();
    if label.is_empty() || label.chars().all(|c| c.is_ascii_digit()) {
        dir_name.to_string()
    } else {
        label.to_string()
    }
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

/// Nombre de Proton instalado que corresponde a una etiqueta como `11.0-100`
/// o `CachyOS-11.0-100`, para lanzar con la capa correcta.
pub fn best_proton(protons: &[Proton], label: &str) -> Option<String> {
    let needle = label.to_lowercase();
    if let Some(proton) = protons
        .iter()
        .find(|proton| proton.name.to_lowercase() == needle)
    {
        return Some(proton.name.clone());
    }
    if let Some(proton) = protons.iter().find(|proton| {
        let name = proton.name.to_lowercase();
        name.contains(&needle) || needle.contains(&name)
    }) {
        return Some(proton.name.clone());
    }

    let vendor_key = vendor(&needle);
    let version = version_key(&needle);
    protons
        .iter()
        .find(|proton| {
            let name = proton.name.to_lowercase();
            vendor_key.is_some() && vendor(&name) == vendor_key && version_key(&name) == version
        })
        .or_else(|| {
            protons
                .iter()
                .find(|proton| version.is_some() && version_key(&proton.name.to_lowercase()) == version)
        })
        .map(|proton| proton.name.clone())
}

/// Parte alfabética inicial: `cachyos-11.0` -> `cachyos`.
fn vendor(name: &str) -> Option<String> {
    let vendor: String = name
        .chars()
        .take_while(|c| !c.is_ascii_digit())
        .filter(|c| c.is_ascii_alphanumeric())
        .collect();
    (!vendor.is_empty()).then_some(vendor)
}

/// Primera versión con forma `11.0` dentro del nombre.
fn version_key(name: &str) -> Option<String> {
    let chars: Vec<char> = name.chars().collect();
    for index in 0..chars.len() {
        if !chars[index].is_ascii_digit() {
            continue;
        }
        let token: String = chars[index..]
            .iter()
            .take_while(|c| c.is_ascii_digit() || **c == '.')
            .collect();
        if token.matches('.').count() == 1 && !token.ends_with('.') {
            return Some(token);
        }
    }
    None
}

/// Un prefijo Wine se reconoce por su unidad `drive_c` y sus registros.
pub fn looks_like_prefix(dir: &Path) -> bool {
    dir.join("drive_c").is_dir() && (dir.join("system.reg").is_file() || dir.join("user.reg").is_file())
}

/// Nombre de la capa de compatibilidad anotada por Steam/Proton en el compatdata.
pub fn proton_label(compat: &Path) -> Option<String> {
    for name in ["version", "config_info"] {
        let Ok(text) = std::fs::read_to_string(compat.join(name)) else {
            continue;
        };
        let line = text.lines().next().unwrap_or_default().trim();
        if !line.is_empty() && line.len() < 80 && !line.contains('/') {
            return Some(line.to_string());
        }
    }
    None
}

/// Busca el prefijo de un juego ya existente: compatdata de Steam o un prefijo
/// Wine junto a la instalación (Lutris, Heroic, copias manuales).
pub fn detect_for(game: &Game) -> Option<(PathBuf, Option<String>)> {
    if let Some(prefix) = game.prefix.as_ref().filter(|p| p.exists()) {
        return Some((prefix.clone(), game.proton.clone()));
    }

    if game.source == Source::Steam {
        if let Some(app_id) = game.app_id {
            for root in steam_roots() {
                for library in steam::libraries(&root) {
                    let compat = library.join("steamapps/compatdata").join(app_id.to_string());
                    let prefix = compat.join("pfx");
                    if prefix.is_dir() {
                        return Some((prefix, proton_label(&compat)));
                    }
                }
            }
        }
    }

    if game.platform != Platform::Windows {
        return None;
    }
    find_near(&game.install_dir).map(|prefix| {
        let compat = compat_dir(&prefix);
        (prefix, proton_label(&compat))
    })
}

/// Prefijos en la propia carpeta, en sus hijos o en la carpeta padre.
fn find_near(install_dir: &Path) -> Option<PathBuf> {
    const NAMES: [&str; 6] = ["pfx", "prefix", "wineprefix", "wine", "compatdata", ".wine"];

    if looks_like_prefix(install_dir) {
        return Some(install_dir.to_path_buf());
    }
    for name in NAMES {
        let candidate = install_dir.join(name);
        if looks_like_prefix(&candidate) {
            return Some(candidate);
        }
    }

    // Prefijos dentro de la carpeta del juego.
    if let Ok(entries) = std::fs::read_dir(install_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            if looks_like_prefix(&path) {
                return Some(path);
            }
            for name in NAMES {
                let candidate = path.join(name);
                if looks_like_prefix(&candidate) {
                    return Some(candidate);
                }
            }
        }
    }

    // Hermano con nombre emparentado, p. ej. GTA V/ y GTA V.pfx/.
    let (Some(parent), Some(own)) = (install_dir.parent(), install_dir.file_name()) else {
        return None;
    };
    let own = own.to_string_lossy().to_lowercase();
    let Ok(entries) = std::fs::read_dir(parent) else {
        return None;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() || path == install_dir {
            continue;
        }
        let name = path.file_name()?.to_string_lossy().to_lowercase();
        if !name.starts_with(&own) && !own.starts_with(&name) {
            continue;
        }
        if looks_like_prefix(&path) {
            return Some(path);
        }
        for candidate_name in NAMES {
            let candidate = path.join(candidate_name);
            if looks_like_prefix(&candidate) {
                return Some(candidate);
            }
        }
    }
    None
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
