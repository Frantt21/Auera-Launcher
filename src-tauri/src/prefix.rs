use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;

use crate::config;
use crate::model::{Game, Platform, Source};
use crate::scan::{hydra, steam, steam_roots};

#[derive(Debug, Clone, Serialize)]
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
            let dir_name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
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
            protons.iter().find(|proton| {
                version.is_some() && version_key(&proton.name.to_lowercase()) == version
            })
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

fn heroic_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(home) = dirs::home_dir() {
        roots.push(home.join(".config/heroic"));
        roots.push(home.join(".var/app/com.heroicgameslauncher.hgl/config/heroic"));
        roots.push(home.join("Games/Heroic"));
    }
    roots
}

/// Prefijo configurado en Heroic para un app id (Epic, GOG, Amazon o sideload).
pub fn heroic_prefix(app_id: &str) -> Option<(PathBuf, Option<String>)> {
    let needle = app_id.to_lowercase();
    for base in heroic_roots() {
        let configs = base.join("GamesConfig");
        let mut files = Vec::new();
        let exact = configs.join(format!("{app_id}.json"));
        if exact.is_file() {
            files.push(exact);
        }
        // El nombre del archivo puede no coincidir en mayúsculas con el app id.
        if let Ok(entries) = std::fs::read_dir(&configs) {
            for entry in entries.flatten() {
                let path = entry.path();
                let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                    continue;
                };
                if stem.to_lowercase() == needle && !files.contains(&path) {
                    files.push(path);
                }
            }
        }
        for file in files {
            if let Some(found) = heroic_config(&file) {
                return Some(found);
            }
        }
        // Prefijos por defecto que Heroic crea por app id.
        for prefix in [
            base.join("Prefixes").join(app_id),
            base.join("Prefixes").join(&needle),
            base.join("prefixes").join(app_id),
        ] {
            if looks_like_prefix(&prefix) {
                return Some((prefix.clone(), proton_label(&prefix)));
            }
        }
    }
    None
}

fn heroic_config(file: &Path) -> Option<(PathBuf, Option<String>)> {
    let raw = std::fs::read_to_string(file).ok()?;
    let value = serde_json::from_str::<serde_json::Value>(&raw).ok()?;
    let config = value
        .as_object()?
        .values()
        .find_map(|entry| entry.as_object())?;
    let prefix = config
        .get("winePrefix")
        .and_then(|v| v.as_str())
        .map(PathBuf::from)
        .filter(|path| looks_like_prefix(path))?;
    let proton = config
        .get("wineVersion")
        .and_then(|v| v.get("name"))
        .and_then(|v| v.as_str())
        .map(str::to_string);
    Some((prefix, proton))
}

fn hydra_entry(object_id: &str) -> Option<hydra::HydraGame> {
    hydra::installed()
        .into_iter()
        .find(|game| game.object_id == object_id)
}

fn hydra_proton(entry: &hydra::HydraGame, prefix: &Path) -> Option<String> {
    entry
        .proton_path
        .as_ref()
        .and_then(|path| path.file_name())
        .map(|name| name.to_string_lossy().to_string())
        .or_else(|| proton_label(&compat_dir(prefix)))
}

/// Prefijo de un juego de Hydra: el que anota su base (puede ser un compatdata
/// de Steam), el de su atajo de Steam o el `wine-prefixes/<objectId>` clásico.
pub fn hydra_prefix(object_id: &str) -> Option<(PathBuf, Option<String>)> {
    match hydra_entry(object_id) {
        Some(entry) => hydra_prefix_of(&entry),
        None => hydra_convention_prefix(object_id),
    }
}

fn hydra_prefix_of(entry: &hydra::HydraGame) -> Option<(PathBuf, Option<String>)> {
    if let Some(prefix) = entry.wine_prefix.as_ref().filter(|p| looks_like_prefix(p)) {
        return Some((prefix.clone(), hydra_proton(entry, prefix)));
    }
    if let Some(shortcut) = entry.steam_shortcut {
        if let Some(found) = steam_prefix(shortcut) {
            return Some(found);
        }
    }
    hydra_convention_prefix(&entry.object_id)
}

fn hydra_convention_prefix(object_id: &str) -> Option<(PathBuf, Option<String>)> {
    for base in hydra::launcher_dirs() {
        for folder in ["wine-prefixes", "prefixes"] {
            let dir = base.join(folder).join(object_id);
            for candidate in [dir.clone(), dir.join("pfx")] {
                if looks_like_prefix(&candidate) {
                    return Some((candidate.clone(), proton_label(&compat_dir(&candidate))));
                }
            }
        }
    }
    None
}

/// Prefijo de Hydra para un juego que se escaneó como carpeta suelta: se
/// compara la instalación con el ejecutable que Hydra tiene apuntado.
fn hydra_prefix_for_dir(install_dir: &Path) -> Option<(PathBuf, Option<String>)> {
    let target = install_dir.to_string_lossy().to_lowercase();
    for entry in hydra::installed() {
        let Some(parent) = entry.executable.parent() else {
            continue;
        };
        let exe_dir = parent.to_string_lossy().to_lowercase();
        if !related_paths(&exe_dir, &target) {
            continue;
        }
        if let Some(found) = hydra_prefix_of(&entry) {
            return Some(found);
        }
    }
    None
}

/// Una de las rutas contiene a la otra como carpeta, no como prefijo de texto.
fn related_paths(a: &str, b: &str) -> bool {
    let under = |root: &str, path: &str| {
        path.strip_prefix(root)
            .map(|rest| rest.starts_with('/'))
            .unwrap_or(false)
    };
    under(a, b) || under(b, a)
}

/// Acepta el prefijo o su carpeta contenedora (`compatdata/<id>`, `wine-prefixes/<id>`)
/// y devuelve la carpeta Wine real.
pub fn normalize(root: &Path) -> Option<PathBuf> {
    if looks_like_prefix(root) {
        return Some(root.to_path_buf());
    }
    let pfx = root.join("pfx");
    looks_like_prefix(&pfx).then_some(pfx)
}

/// Compatdata de una app de Steam.
pub fn steam_prefix(app_id: u32) -> Option<(PathBuf, Option<String>)> {
    for root in steam_roots() {
        for library in steam::libraries(&root) {
            let compat = library
                .join("steamapps/compatdata")
                .join(app_id.to_string());
            let prefix = compat.join("pfx");
            if prefix.is_dir() {
                return Some((prefix, proton_label(&compat)));
            }
        }
    }
    None
}

/// Nombre de la carpeta cuando la instalación cuelga de un `steamapps/common`.
fn steam_dir_name(install_dir: &Path) -> Option<String> {
    let common = install_dir.parent()?;
    if common.file_name()? != "common" || common.parent()?.file_name()? != "steamapps" {
        return None;
    }
    Some(install_dir.file_name()?.to_string_lossy().to_string())
}

/// Apps de Steam instaladas en una biblioteca, por `installdir` del manifiesto.
fn steam_app_for_dir(install_dir: &Path) -> Option<u32> {
    let folder = steam_dir_name(install_dir)?.to_lowercase();
    for root in steam_roots() {
        for library in steam::libraries(&root) {
            let steamapps = library.join("steamapps");
            let Ok(entries) = std::fs::read_dir(&steamapps) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_lowercase();
                if !(name.starts_with("appmanifest_") && name.ends_with(".acf")) {
                    continue;
                }
                let Ok(raw) = std::fs::read_to_string(&path) else {
                    continue;
                };
                let Some(node) = crate::vdf::parse(&raw) else {
                    continue;
                };
                let Some(state) = node.map("AppState") else {
                    continue;
                };
                let Some(installdir) = state.str("installdir") else {
                    continue;
                };
                if installdir.to_lowercase() != folder {
                    continue;
                }
                return state
                    .str("appid")
                    .and_then(|id| id.parse().ok())
                    .or_else(|| {
                        name.trim_start_matches("appmanifest_")
                            .trim_end_matches(".acf")
                            .parse()
                            .ok()
                    });
            }
        }
    }
    None
}

fn lutris_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(home) = dirs::home_dir() {
        roots.push(home.join(".config/lutris/games"));
        roots.push(home.join(".var/app/net.lutris.Lutris/config/lutris/games"));
    }
    if let Some(data) = dirs::data_dir() {
        roots.push(data.join("lutris/games"));
    }
    roots
}

/// Prefijo de Lutris cuyo `exe` cae dentro de la instalación del juego.
fn lutris_prefix(install_dir: &Path) -> Option<(PathBuf, Option<String>)> {
    let target = install_dir.to_string_lossy().to_lowercase();
    for root in lutris_roots() {
        let Ok(entries) = std::fs::read_dir(&root) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !matches!(
                path.extension().and_then(|e| e.to_str()),
                Some("yml") | Some("yaml")
            ) {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            let Some(exe) = yml_field(&text, "exe") else {
                continue;
            };
            let Some(parent) = expand(&exe).parent().map(|p| p.to_path_buf()) else {
                continue;
            };
            if !related_paths(&parent.to_string_lossy().to_lowercase(), &target) {
                continue;
            }
            let Some(raw) = yml_field(&text, "prefix") else {
                continue;
            };
            let prefix = expand(&raw);
            if looks_like_prefix(&prefix) {
                return Some((prefix.clone(), proton_label(&compat_dir(&prefix))));
            }
        }
    }
    None
}

fn yml_field(text: &str, key: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let value = line.trim().strip_prefix(key)?.strip_prefix(':')?.trim();
        (!value.is_empty()).then(|| value.trim_matches(&['"', '\''][..]).to_string())
    })
}

/// Un prefijo Wine se reconoce por su unidad `drive_c` y sus registros.
/// Acepta rutas escritas a mano con `~` y comillas.
pub fn expand(text: &str) -> PathBuf {
    let trimmed = text.trim().trim_matches('"');
    if trimmed == "~" {
        return dirs::home_dir().unwrap_or_else(|| PathBuf::from(trimmed));
    }
    match trimmed.strip_prefix("~/") {
        Some(rest) => dirs::home_dir()
            .map(|home| home.join(rest))
            .unwrap_or_else(|| PathBuf::from(trimmed)),
        None => PathBuf::from(trimmed),
    }
}

pub fn looks_like_prefix(dir: &Path) -> bool {
    dir.join("drive_c").is_dir()
        && (dir.join("system.reg").is_file() || dir.join("user.reg").is_file())
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

/// Busca el prefijo de un juego ya existente; los nativos no usan Wine.
/// Primero por el id del lanzador, luego comparando la instalación con sus
/// bases (Hydra, Steam, Lutris) y al final por prefijos junto al juego.
pub fn detect_for(game: &Game) -> Option<(PathBuf, Option<String>)> {
    if let Some(prefix) = game.prefix.as_ref().filter(|p| p.exists()) {
        return Some((prefix.clone(), game.proton.clone()));
    }
    if game.platform != Platform::Windows {
        return None;
    }

    if let Some(found) = by_launcher_id(game) {
        return Some(found);
    }
    if let Some(found) = by_install_dir(&game.install_dir) {
        return Some(found);
    }

    find_near(&game.install_dir).map(|prefix| {
        let compat = compat_dir(&prefix);
        (prefix, proton_label(&compat))
    })
}

fn by_launcher_id(game: &Game) -> Option<(PathBuf, Option<String>)> {
    match game.source {
        Source::Steam => game.app_id.and_then(steam_prefix),
        Source::Hydra => hydra_object_id(&game.id).and_then(hydra_prefix),
        Source::Heroic => heroic_app_id(&game.id).and_then(heroic_prefix),
        Source::Epic => game.id.strip_prefix("epic:").and_then(heroic_prefix),
        _ => game.app_id.and_then(steam_prefix),
    }
}

fn by_install_dir(install_dir: &Path) -> Option<(PathBuf, Option<String>)> {
    hydra_prefix_for_dir(install_dir)
        .or_else(|| steam_app_for_dir(install_dir).and_then(steam_prefix))
        .or_else(|| lutris_prefix(install_dir))
}

/// `heroic:epic:<app>` o `heroic:gog:<app>` -> `<app>`.
fn heroic_app_id(id: &str) -> Option<&str> {
    id.strip_prefix("heroic:")?.split_once(':').map(|(_, app)| app)
}

/// `hydra:<shop>:<objectId>` -> `<objectId>`.
fn hydra_object_id(id: &str) -> Option<&str> {
    id.strip_prefix("hydra:")?.split_once(':').map(|(_, object)| object)
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

    // El ejecutable puede vivir dentro del propio prefijo (Hydra, compatdata).
    for ancestor in install_dir.ancestors().skip(1).take(4) {
        if looks_like_prefix(ancestor) {
            return Some(ancestor.to_path_buf());
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
        prefix
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| prefix.to_path_buf())
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

#[cfg(test)]
mod tests {
    #[test]
    fn lee_las_rutas_de_lutris() {
        let text = "game:\n  exe: /juegos/pfx/drive_c/Game.exe\n  prefix: '~/prefixes/game'\n";
        assert_eq!(
            super::yml_field(text, "exe").as_deref(),
            Some("/juegos/pfx/drive_c/Game.exe")
        );
        assert_eq!(
            super::yml_field(text, "prefix").as_deref(),
            Some("~/prefixes/game")
        );
        assert_eq!(super::yml_field(text, "enabled"), None);
    }

    #[test]
    fn compara_carpetas_no_texto() {
        assert!(super::related_paths("/juegos/gta", "/juegos/gta/drive_c"));
        assert!(!super::related_paths("/juegos/gta", "/juegos/gta2"));
    }

    #[test]
    fn extrae_el_app_id_de_los_ids() {
        assert_eq!(super::heroic_app_id("heroic:epic:Rocket"), Some("Rocket"));
        assert_eq!(super::hydra_object_id("hydra:steam:239140"), Some("239140"));
        assert_eq!(super::heroic_app_id("steam:1"), None);
        assert_eq!(super::hydra_object_id("folder:x"), None);
    }

    #[test]
    fn expande_la_tilde() {
        let home = dirs::home_dir().expect("home");
        assert_eq!(super::expand("~/juegos"), home.join("juegos"));
        assert_eq!(super::expand("  \"~/pfx\" "), home.join("pfx"));
        assert_eq!(
            super::expand("/ruta/absoluta"),
            std::path::PathBuf::from("/ruta/absoluta")
        );
    }
}



