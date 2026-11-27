use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, Manager};

use crate::accent::{self, Rgb};
use crate::api;
use crate::backdrop;
use crate::config::Config;
use crate::gamepad::{Gamepad, Nav};
use crate::library::Library;
use crate::model::{Game, Platform, Tier, unix_now};
use crate::prefix::{self, Proton};
use crate::prefs::Prefs;
use crate::scan;

pub const DEFAULT_ACCENT: Rgb = Rgb { r: 196, g: 156, b: 255 };

#[derive(Clone, serde::Serialize)]
pub struct Busy {
    pub scanning: bool,
    pub art: bool,
    pub tiers: bool,
    pub prefix: bool,
}

#[derive(Clone, serde::Serialize)]
pub struct Candidates {
    pub id: String,
    pub results: Vec<(u32, String)>,
}

pub struct Service {
    pub app: AppHandle,
    pub cfg: Mutex<Config>,
    pub library: Mutex<Library>,
    pub prefs: Mutex<Prefs>,
    pub protons: Vec<Proton>,
    pub protondb_cache: Mutex<HashMap<u32, Tier>>,
    pub accents: Mutex<HashMap<PathBuf, Option<Rgb>>>,
    pub log: Mutex<Vec<String>>,
    pub scanning: AtomicBool,
    pub prefix_scanning: AtomicBool,
    pub art_fetching: AtomicBool,
    pub tier_fetching: AtomicBool,
    pub running: AtomicBool,
}

impl Service {
    pub fn new(app: AppHandle) -> Self {
        let cfg = Config::load();
        let prefs = Prefs::load();
        let mut library = Library::load();
        let protons = prefix::detect_protons();
        let cache = crate::api::protondb::load_cache();
        for game in &mut library.games {
            if let Some(tier) = game.app_id.and_then(|id| cache.get(&id)) {
                game.tier = *tier;
            }
            if let Some(label) = game.proton.as_ref() {
                if let Some(p) = prefix::best_proton(&protons, label) {
                    game.proton = Some(p);
                }
            }
        }
        Self {
            app,
            cfg: Mutex::new(cfg),
            library: Mutex::new(library),
            prefs: Mutex::new(prefs),
            protons,
            protondb_cache: Mutex::new(cache),
            accents: Mutex::new(HashMap::new()),
            log: Mutex::new(Vec::new()),
            scanning: AtomicBool::new(false),
            prefix_scanning: AtomicBool::new(false),
            art_fetching: AtomicBool::new(false),
            tier_fetching: AtomicBool::new(false),
            running: AtomicBool::new(false),
        }
    }

    pub fn cfg(&self) -> Config {
        self.cfg.lock().unwrap().clone()
    }

    pub fn prefs(&self) -> Prefs {
        self.prefs.lock().unwrap().clone()
    }

    pub fn games(&self) -> Vec<Game> {
        self.library.lock().unwrap().games.clone()
    }

    pub fn busy(&self) -> Busy {
        Busy {
            scanning: self.scanning.load(Ordering::SeqCst),
            art: self.art_fetching.load(Ordering::SeqCst),
            tiers: self.tier_fetching.load(Ordering::SeqCst),
            prefix: self.prefix_scanning.load(Ordering::SeqCst),
        }
    }

    pub fn emit_busy(&self) {
        let _ = self.app.emit("busy", self.busy());
    }

    pub fn emit_library(&self) {
        let games = self.games();
        let _ = self.app.emit("library", games);
    }

    pub fn log(&self, msg: String) {
        let mut log = self.log.lock().unwrap();
        log.push(msg);
        if log.len() > 400 {
            let drop = log.len() - 400;
            log.drain(0..drop);
        }
        let snapshot = log.clone();
        drop(log);
        let _ = self.app.emit("log", snapshot);
    }

    pub fn toast(&self, msg: String) {
        let _ = self.app.emit("toast", msg);
    }

    pub fn accent_for(&self, id: &str, prefer_logo: bool) -> Rgb {
        let Some(game) = self.library.lock().unwrap().get(id).cloned() else {
            return DEFAULT_ACCENT;
        };
        let mut paths: Vec<PathBuf> = Vec::new();
        if prefer_logo {
            paths.extend(game.art.logo.clone());
            paths.extend(game.art.cover.clone());
        } else {
            paths.extend(game.art.cover.clone());
            paths.extend(game.art.logo.clone());
        }
        for path in paths {
            let cached = self.accents.lock().unwrap().get(&path).copied();
            if let Some(cached) = cached {
                if let Some(color) = cached {
                    return color;
                }
                continue;
            }
            let color = accent::analyze(&path);
            self.accents.lock().unwrap().insert(path, color);
            if let Some(color) = color {
                return color;
            }
        }
        DEFAULT_ACCENT
    }

    pub fn backdrop_for(&self, id: &str) -> Option<String> {
        let game = self.library.lock().unwrap().get(id).cloned()?;
        let source = game.art.hero.clone().or(game.art.cover.clone())?;
        let path = backdrop::generate(&game.id, &source)?;
        Some(crate::media::url(&path, 0))
    }

    pub fn play(&self, id: &str) -> Result<(), String> {
        let mut game = self.library.lock().unwrap().get(id).cloned().ok_or("Juego no encontrado")?;
        if !game.compatible() {
            return Err(format!("{} no se puede lanzar en este sistema", game.title));
        }
        if self.running.swap(true, Ordering::SeqCst) {
            return Err("Ya hay un juego en ejecución".into());
        }

        if cfg!(unix) && game.platform == Platform::Windows && game.proton.is_none() {
            if let Some(prefix) = game.prefix.clone() {
                let compat = prefix::compat_dir(&prefix::expand(&prefix.to_string_lossy()));
                if let Some(name) = prefix::proton_label(&compat)
                    .and_then(|label| prefix::best_proton(&self.protons, &label))
                {
                    game.proton = Some(name.clone());
                    {
                        let mut lib = self.library.lock().unwrap();
                        if let Some(stored) = lib.get_mut(&game.id) {
                            stored.proton = Some(name);
                        }
                        lib.save();
                    }
                }
            }
        }

        if cfg!(unix) && game.platform == Platform::Windows && game.prefix.is_none() {
            if let Some((prefix, proton)) = prefix::detect_for(&game) {
                game.prefix = Some(prefix);
                if let Some(label) = proton {
                    game.proton = Some(prefix::best_proton(&self.protons, &label).unwrap_or(label));
                }
                {
                    let mut lib = self.library.lock().unwrap();
                    if let Some(stored) = lib.get_mut(&game.id) {
                        stored.prefix = game.prefix.clone();
                        stored.proton = game.proton.clone();
                    }
                    lib.save();
                }
            }
        }

        if cfg!(unix) && game.platform == Platform::Windows && game.proton.is_some() {
            if game.prefix.is_none() || game.prefix.as_ref().map(|p| !p.exists()).unwrap_or(true) {
                match prefix::ensure(&mut game, &self.protons) {
                    Ok(p) => {
                        game.prefix = Some(p);
                        {
                            let mut lib = self.library.lock().unwrap();
                            if let Some(stored) = lib.get_mut(&game.id) {
                                stored.prefix = game.prefix.clone();
                                stored.proton = game.proton.clone();
                            }
                            lib.save();
                        }
                    }
                    Err(e) => {
                        self.running.store(false, Ordering::SeqCst);
                        return Err(format!("{}: {e}", game.title));
                    }
                }
            }
        }

        let plan = match crate::launch::plan(&game, &self.protons) {
            Ok(p) => p,
            Err(e) => {
                self.running.store(false, Ordering::SeqCst);
                return Err(format!("{}: {e}", game.title));
            }
        };
        let mut child = match plan.spawn() {
            Ok(c) => c,
            Err(e) => {
                self.running.store(false, Ordering::SeqCst);
                return Err(format!("{}: {e}", game.title));
            }
        };

        self.log(format!("Lanzando {}", game.title));
        let app = self.app.clone();
        let id_owned = id.to_string();
        let started = Instant::now();
        std::thread::spawn(move || {
            let _ = child.wait();
            let minutes = (started.elapsed().as_secs() / 60).max(1) as u64;
            let service = app.state::<Service>();
            {
                let mut lib = service.library.lock().unwrap();
                if let Some(g) = lib.get_mut(&id_owned) {
                    g.playtime_min += minutes;
                    g.last_played = Some(unix_now());
                }
                lib.save();
            }
            service.running.store(false, Ordering::SeqCst);
            service.log(format!("Sesión finalizada (+{minutes} min)"));
            service.emit_library();
        });
        Ok(())
    }
}

// --- funciones libres que lanzan trabajo en segundo plano ---

pub fn start_background(app: AppHandle) {
    start_prefix_scan(app.clone());
    start_gamepad(app);
}

pub fn start_scan(app: AppHandle) {
    let service = app.state::<Service>();
    if service.scanning.swap(true, Ordering::SeqCst) {
        return;
    }
    service.emit_busy();
    let cfg = service.cfg();
    let app2 = app.clone();
    std::thread::spawn(move || {
        let found = scan::scan_all(&cfg);
        let count = found.len();
        let service = app2.state::<Service>();
        {
            let mut lib = service.library.lock().unwrap();
            lib.merge(found);
        }
        service.log(format!("Escaneo completado: {count} juegos detectados"));
        service.scanning.store(false, Ordering::SeqCst);
        service.emit_library();
        service.emit_busy();
        start_art(app2.clone(), None);
        start_tiers(app2.clone());
        start_prefix_scan(app2.clone());
    });
}

pub fn start_art(app: AppHandle, only: Option<String>) {
    let service = app.state::<Service>();
    if service.art_fetching.swap(true, Ordering::SeqCst) {
        return;
    }
    service.emit_busy();
    let cfg = service.cfg();
    let app2 = app.clone();
    std::thread::spawn(move || {
        if !cfg.steamgrid_enabled {
            let service = app2.state::<Service>();
            service.log("SteamGridDB desactivado".into());
            service.art_fetching.store(false, Ordering::SeqCst);
            service.emit_busy();
            return;
        }
        let client = if cfg.steamgrid_key.trim().is_empty() {
            None
        } else {
            api::steamgrid::Client::new(cfg.steamgrid_key.trim())
        };
        let Some(client) = client else {
            let service = app2.state::<Service>();
            service.log("SteamGridDB no configurado".into());
            service.art_fetching.store(false, Ordering::SeqCst);
            service.emit_busy();
            return;
        };
        let service = app2.state::<Service>();
        let force = only.is_some();
        let all: Vec<Game> = service.games();
        let games: Vec<Game> = if let Some(id) = &only {
            all.into_iter().filter(|g| g.id == *id).collect()
        } else {
            all.into_iter().filter(|g| g.art.needs_download()).collect()
        };
        if games.is_empty() {
            let service = app2.state::<Service>();
            service.art_fetching.store(false, Ordering::SeqCst);
            service.emit_busy();
            return;
        }
        for game in games {
            if game.id.is_empty() { continue; }
            let (assets, sgdb_id) = api::steamgrid::fetch_assets(&client, &game.id, &game, force);
            let service = app2.state::<Service>();
            {
                let mut lib = service.library.lock().unwrap();
                if let Some(stored) = lib.get_mut(&game.id) {
                    stored.art = assets;
                    if let Some(id) = sgdb_id {
                        stored.sgdb_id = Some(id);
                    }
                    backdrop::invalidate(&stored.id);
                }
                lib.save();
            }
            service.emit_library();
            let _ = app2.emit("art-updated", &game.id);
        }
        let service = app2.state::<Service>();
        service.art_fetching.store(false, Ordering::SeqCst);
        service.emit_busy();
        service.log("Arte sincronizado".into());
    });
}

pub fn start_tiers(app: AppHandle) {
    let service = app.state::<Service>();
    if service.tier_fetching.swap(true, Ordering::SeqCst) {
        return;
    }
    service.emit_busy();
    let app2 = app.clone();
    std::thread::spawn(move || {
        let service = app2.state::<Service>();
        let games: Vec<Game> = service.games();
        let mut cache = service.protondb_cache.lock().unwrap().clone();
        for mut game in games {
            let Some(app_id) = game.app_id else { continue };
            let Some(tier) = crate::api::protondb::fetch(app_id) else { continue };
            game.tier = tier;
            cache.insert(app_id, tier);
            let service = app2.state::<Service>();
            {
                let mut lib = service.library.lock().unwrap();
                if let Some(stored) = lib.get_mut(&game.id) {
                    stored.tier = tier;
                }
                lib.save();
            }
            service.emit_library();
        }
        let service = app2.state::<Service>();
        *service.protondb_cache.lock().unwrap() = cache;
        crate::api::protondb::save_cache(&service.protondb_cache.lock().unwrap());
        service.tier_fetching.store(false, Ordering::SeqCst);
        service.emit_busy();
        service.log("Tiers sincronizados".into());
    });
}

pub fn start_prefix_scan(app: AppHandle) {
    let service = app.state::<Service>();
    let pending: Vec<Game> = service
        .games()
        .into_iter()
        .filter(|g| {
            g.platform == Platform::Windows
                && (g.prefix.is_none() || g.prefix.as_ref().map(|p| !p.exists()).unwrap_or(true))
        })
        .collect();
    if pending.is_empty() {
        return;
    }
    if service.prefix_scanning.swap(true, Ordering::SeqCst) {
        return;
    }
    service.emit_busy();
    let app2 = app.clone();
    std::thread::spawn(move || {
        for game in pending {
            if let Some((prefix, proton)) = prefix::detect_for(&game) {
                let service = app2.state::<Service>();
                let mut lib = service.library.lock().unwrap();
                if let Some(stored) = lib.get_mut(&game.id) {
                    let exists = stored.prefix.as_ref().map(|p| p.exists()).unwrap_or(false);
                    if !exists {
                        stored.prefix = Some(prefix);
                        if let Some(label) = proton {
                            if let Some(p) = prefix::best_proton(&service.protons, &label) {
                                stored.proton = Some(p);
                            } else {
                                stored.proton = Some(label);
                            }
                        }
                        lib.save();
                    }
                }
            }
        }
        let service = app2.state::<Service>();
        service.prefix_scanning.store(false, Ordering::SeqCst);
        service.emit_library();
        service.emit_busy();
        service.log("Prefijos detectados".into());
    });
}

pub fn search_art(app: AppHandle, id: String, query: String) {
    let cfg = app.state::<Service>().cfg();
    let key = cfg.steamgrid_key.trim();
    if key.is_empty() {
        app.state::<Service>().toast("Configura SteamGridDB".into());
        return;
    };
            let client = api::steamgrid::Client::new(key);
        let app2 = app.clone();
        std::thread::spawn(move || {
            let results = client
                .map(|c| c.search_games(&query))
                .unwrap_or_default();
            let _ = app2.emit("candidates", Candidates { id, results });
        });
}

pub fn pick_art(app: AppHandle, id: String, sgdb_id: u32) {
    let service = app.state::<Service>();
    {
        let mut lib = service.library.lock().unwrap();
        if let Some(stored) = lib.get_mut(&id) {
            stored.art = crate::model::Art::default();
            stored.sgdb_id = Some(sgdb_id);
        }
        lib.save();
    }
    start_art(app.clone(), Some(id));
}

pub fn compute_size(app: AppHandle, id: String) {
    let app2 = app.clone();
    std::thread::spawn(move || {
        let service = app2.state::<Service>();
        let path = service.games().into_iter().find(|g| g.id == id).map(|g| g.install_dir.clone());
        let Some(path) = path else { return; };
        let size = scan::dir_size(&path);
        let service = app2.state::<Service>();
        {
            let mut lib = service.library.lock().unwrap();
            if let Some(g) = lib.get_mut(&id) {
                g.size_bytes = Some(size);
            }
            lib.save();
        }
        service.emit_library();
    });
}

pub fn create_prefix(app: AppHandle, id: String, proton_label: String) {
    let app2 = app.clone();
    std::thread::spawn(move || {
        let service = app2.state::<Service>();
        let game = service.games().into_iter().find(|g| g.id == id);
        let Some(mut game) = game else { return; };
        if let Some(p) = prefix::best_proton(&service.protons, &proton_label) {
            game.proton = Some(p);
        } else {
            game.proton = Some(proton_label.clone());
        }
        let proton = game.proton.clone().unwrap_or_default();
        match prefix::ensure(&mut game, &service.protons) {
            Ok(p) => {
                let service = app2.state::<Service>();
                {
                    let mut lib = service.library.lock().unwrap();
                    if let Some(stored) = lib.get_mut(&id) {
                        stored.prefix = Some(p);
                        stored.proton = Some(proton);
                    }
                    lib.save();
                }
                service.emit_library();
                service.log(format!("Prefijo creado para {}", game.title));
                service.toast("Prefijo creado".into());
            }
            Err(e) => {
                app2.state::<Service>().toast(format!("Error: {e}"));
            }
        }
    });
}

pub fn detect_prefix(app: AppHandle, id: String) {
    let app2 = app.clone();
    std::thread::spawn(move || {
        let service = app2.state::<Service>();
        let game = service.games().into_iter().find(|g| g.id == id);
        let Some(game) = game else { return; };
        if let Some((prefix, proton)) = prefix::detect_for(&game) {
            let service = app2.state::<Service>();
            {
                let mut lib = service.library.lock().unwrap();
                if let Some(stored) = lib.get_mut(&id) {
                    stored.prefix = Some(prefix);
                    if let Some(label) = proton {
                        if let Some(p) = prefix::best_proton(&service.protons, &label) {
                            stored.proton = Some(p);
                        } else {
                            stored.proton = Some(label);
                        }
                    }
                }
                lib.save();
            }
            service.emit_library();
            service.log("Prefijo detectado".into());
        } else {
            app2.state::<Service>().toast("No se encontró prefijo".into());
        }
    });
}

pub fn start_gamepad(app: AppHandle) {
    std::thread::spawn(move || {
        let Some(mut pad) = Gamepad::new() else { return; };
        let name = pad.name().to_string();
        let _ = app.emit("gamepad-name", name);
        loop {
            let enabled = app
                .state::<Service>()
                .cfg
                .lock()
                .unwrap()
                .gamepad_enabled;
            if enabled {
                for nav in pad.poll() {
                    let _ = app.emit("gamepad", nav);
                }
            }
            std::thread::sleep(Duration::from_millis(16));
        }
    });
}

// Evita warning por import no usado en caso de no llamar.
#[allow(dead_code)]
fn _touch_nav(_n: Nav) {}
