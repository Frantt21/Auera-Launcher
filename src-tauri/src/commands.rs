use tauri::{AppHandle, Manager, State};

use crate::accent::Rgb;
use crate::config::Config;
use crate::model::Game;
use crate::prefix::Proton;
use crate::prefs::Prefs;
use crate::service::{self, Service};

#[derive(serde::Serialize)]
pub struct Bootstrap {
    pub cfg: Config,
    pub prefs: Prefs,
    pub games: Vec<Game>,
    pub protons: Vec<Proton>,
    pub data_dir: String,
    pub media_base: String,
    pub busy: service::Busy,
    pub log: Vec<String>,
}

#[tauri::command]
pub fn bootstrap(state: State<Service>) -> Bootstrap {
    Bootstrap {
        cfg: state.cfg(),
        prefs: state.prefs(),
        games: state.games(),
        protons: state.protons.clone(),
        data_dir: crate::config::data_dir().to_string_lossy().to_string(),
        media_base: crate::media::base().to_string(),
        busy: state.busy(),
        log: state.log.lock().unwrap().clone(),
    }
}

#[tauri::command]
pub fn scan(app: AppHandle) {
    service::start_scan(app);
}

#[tauri::command]
pub fn fetch_art(app: AppHandle, only: Option<String>) {
    service::start_art(app, only);
}

#[tauri::command]
pub fn fetch_tiers(app: AppHandle) {
    service::start_tiers(app);
}

#[tauri::command]
pub fn search_art(app: AppHandle, id: String, query: String) {
    service::search_art(app, id, query);
}

#[tauri::command]
pub fn pick_art(app: AppHandle, id: String, sgdb_id: u32) {
    service::pick_art(app, id, sgdb_id);
}

#[tauri::command]
pub fn compute_size(app: AppHandle, id: String) {
    service::compute_size(app, id);
}

#[tauri::command]
pub fn create_prefix(app: AppHandle, id: String, proton_label: String) {
    service::create_prefix(app, id, proton_label);
}

#[tauri::command]
pub fn detect_prefix(app: AppHandle, id: String) {
    service::detect_prefix(app, id);
}

#[tauri::command]
pub fn play(state: State<Service>, id: String) -> Result<(), String> {
    state.play(&id)
}

// Analizar carátulas y generar backdrops decodifica imágenes: si se ejecutara en
// el hilo principal la ventana se quedaría en "no responde".
#[tauri::command]
pub async fn accent(app: AppHandle, id: String, prefer_logo: bool) -> Rgb {
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<Service>().accent_for(&id, prefer_logo)
    })
    .await
    .unwrap_or(service::DEFAULT_ACCENT)
}

#[tauri::command]
pub async fn backdrop(app: AppHandle, id: String) -> Option<String> {
    tauri::async_runtime::spawn_blocking(move || app.state::<Service>().backdrop_for(&id))
        .await
        .ok()
        .flatten()
}

#[tauri::command]
pub async fn refresh_backdrop(app: AppHandle, id: String) -> Option<String> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::backdrop::invalidate(&id);
        app.state::<Service>().backdrop_for(&id)
    })
    .await
    .ok()
    .flatten()
}

#[tauri::command]
pub fn set_config(state: State<Service>, cfg: Config) {
    cfg.save();
    *state.cfg.lock().unwrap() = cfg;
}

#[tauri::command]
pub fn set_prefs(state: State<Service>, prefs: Prefs) {
    prefs.save();
    *state.prefs.lock().unwrap() = prefs;
}

#[tauri::command]
pub fn update_game(state: State<Service>, game: Game) {
    let mut lib = state.library.lock().unwrap();
    if let Some(stored) = lib.get_mut(&game.id) {
        stored.title = game.title;
        stored.executable = game.executable;
        stored.install_dir = game.install_dir;
        stored.platform = game.platform;
        stored.platform_locked = game.platform_locked;
        stored.blocked = game.blocked;
        stored.proton = game.proton;
        stored.prefix = game.prefix;
        stored.favorite = game.favorite;
        stored.tier = game.tier;
    }
    lib.save();
    drop(lib);
    state.emit_library();
}

#[tauri::command]
pub fn remove_game(state: State<Service>, id: String) {
    let mut lib = state.library.lock().unwrap();
    lib.remove(&id);
    drop(lib);
    state.emit_library();
}

#[tauri::command]
pub fn toggle_favorite(state: State<Service>, id: String) {
    let mut lib = state.library.lock().unwrap();
    if let Some(g) = lib.get_mut(&id) {
        g.favorite = !g.favorite;
    }
    lib.save();
    drop(lib);
    state.emit_library();
}
