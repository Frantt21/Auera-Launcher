use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Child;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

use egui::Context;

use crate::api::{protondb, steamgrid};
use crate::config::Config;
use crate::gamepad::{Gamepad, Nav};
use crate::i18n::{tr, Txt};
use crate::library::Library;
use crate::launch;
use crate::model::{unix_now, Art, Game, Platform, Source, Tier};
use crate::prefix::{self, Proton};
use crate::scan;
use crate::theme;
use crate::ui::{self, Action, Textures};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum View {
    Library,
    Detail(String),
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sort {
    Name,
    Recent,
    Playtime,
    Size,
}

pub enum Event {
    Scanned(Vec<Game>),
    Art { id: String, art: Art, sgdb_id: Option<u32> },
    ArtDone,
    Tier { id: String, tier: Tier },
    TierDone,
    PrefixDetected { id: String, prefix: PathBuf, proton: Option<String>, force: bool },
    PrefixMissing(String),
    PrefixScanDone,
    Candidates { id: String, results: Vec<(u32, String)> },
    Size { id: String, bytes: u64 },
    Prefix { id: String, result: Result<std::path::PathBuf, String> },
}

struct Running {
    id: String,
    child: Child,
    started: Instant,
}

pub struct App {
    pub cfg: Config,
    pub library: Library,
    pub textures: Textures,
    pub view: View,
    pub selected: Option<String>,
    pub search: String,
    pub source_filter: Option<Source>,
    pub platform_filter: Option<Platform>,
    pub favorites_only: bool,
    pub sort: Sort,
    pub cursor: usize,
    /// Solo se resalta el cursor de la rejilla cuando se navega con mando o teclado.
    pub nav_mode: bool,
    pub columns: usize,
    pub scanning: bool,
    pub prefix_scanning: bool,
    pub art_fetching: bool,
    pub tier_fetching: bool,
    pub protons: Vec<Proton>,
    pub protondb_cache: HashMap<u32, Tier>,
    pub log: Vec<String>,
    pub toast: Option<(String, Instant)>,
    pub candidates: Vec<(u32, String)>,
    pub candidates_for: Option<String>,
    pub art_query: String,
    pub folder_input: String,
    pub confirm_delete: bool,
    running: Option<Running>,
    gamepad: Option<Gamepad>,
    tx: Sender<Event>,
    rx: Receiver<Event>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::apply(&cc.egui_ctx);
        let (tx, rx) = mpsc::channel();
        let cfg = Config::load();
        let mut app = Self {
            library: Library::load(),
            textures: Textures::new(),
            view: View::Library,
            selected: None,
            search: String::new(),
            source_filter: None,
            platform_filter: None,
            favorites_only: false,
            sort: Sort::Recent,
            cursor: 0,
            columns: 6,
            scanning: false,
            prefix_scanning: false,
            art_fetching: false,
            tier_fetching: false,
            protons: prefix::detect_protons(),
            protondb_cache: protondb::load_cache(),
            log: Vec::new(),
            toast: None,
            candidates: Vec::new(),
            candidates_for: None,
            art_query: String::new(),
            folder_input: String::new(),
            nav_mode: false,
            confirm_delete: false,
            running: None,
            gamepad: cfg.gamepad_enabled.then(Gamepad::new).flatten(),
            tx,
            rx,
            cfg,
        };
        for game in &mut app.library.games {
            if let Some(tier) = game.app_id.and_then(|id| app.protondb_cache.get(&id)) {
                game.tier = *tier;
            }
        }
        // Normaliza etiquetas de Proton guardadas con otro formato (p. ej. "11.0-100").
        let protons = app.protons.clone();
        for game in &mut app.library.games {
            if let Some(label) = game.proton.clone() {
                if let Some(matched) = prefix::best_proton(&protons, &label) {
                    game.proton = Some(matched);
                }
            }
        }

        app.start_scan();
        app.start_prefix_scan();
        app
    }

    pub fn t(&self, txt: Txt) -> &'static str {
        tr(self.cfg.lang, txt)
    }

    pub fn gamepad_name(&self) -> Option<&str> {
        self.gamepad.as_ref().map(|pad| pad.name())
    }

    pub fn filtered(&self) -> Vec<&Game> {
        let query = self.search.to_lowercase();
        let mut games: Vec<&Game> = self
            .library
            .games
            .iter()
            .filter(|game| {
                query.is_empty() || game.title.to_lowercase().contains(&query)
            })
            .filter(|game| self.source_filter.map(|s| game.source == s).unwrap_or(true))
            .filter(|game| {
                self.platform_filter
                    .map(|p| game.platform == p)
                    .unwrap_or(true)
            })
            .filter(|game| !self.favorites_only || game.favorite)
            .collect();
        match self.sort {
            Sort::Name => games.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase())),
            Sort::Recent => games.sort_by_key(|g| std::cmp::Reverse(g.last_played)),
            Sort::Playtime => games.sort_by_key(|g| std::cmp::Reverse(g.playtime_min)),
            Sort::Size => games.sort_by_key(|g| std::cmp::Reverse(g.size_bytes)),
        }
        games
    }

    // --- trabajos en segundo plano -------------------------------------

    pub fn start_scan(&mut self) {
        if self.scanning {
            return;
        }
        self.scanning = true;
        let cfg = self.cfg.clone();
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let games = scan::scan_all(&cfg);
            let _ = tx.send(Event::Scanned(games));
        });
    }

    /// Completa los prefijos de los juegos que ya estaban en la biblioteca.
    pub fn start_prefix_scan(&mut self) {
        if self.prefix_scanning {
            return;
        }
        let pending: Vec<Game> = self
            .library
            .games
            .iter()
            .filter(|game| !game.prefix.as_ref().map(|p| p.exists()).unwrap_or(false))
            .cloned()
            .collect();
        if pending.is_empty() {
            return;
        }
        self.prefix_scanning = true;
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            for game in pending {
                if let Some((prefix, proton)) = prefix::detect_for(&game) {
                    let _ = tx.send(Event::PrefixDetected {
                        id: game.id,
                        prefix,
                        proton,
                        force: false,
                    });
                }
            }
            let _ = tx.send(Event::PrefixScanDone);
        });
    }

    pub fn start_art_fetch(&mut self, only: Option<String>) {
        if self.art_fetching || !self.cfg.steamgrid_enabled {
            return;
        }
        let force = only.is_some();
        let games: Vec<Game> = match only {
            Some(id) => self.library.get(&id).cloned().into_iter().collect(),
            None => self
                .library
                .games
                .iter()
                .filter(|game| game.art.needs_download())
                .cloned()
                .collect(),
        };
        if games.is_empty() {
            return;
        }
        self.art_fetching = true;
        let key = self.cfg.steamgrid_key.clone();
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            if let Some(client) = steamgrid::Client::new(&key) {
                for game in games {
                    let (art, sgdb_id) = steamgrid::fetch_assets(&client, &game.id, &game, force);
                    if !art.is_empty() {
                        let _ = tx.send(Event::Art {
                            id: game.id,
                            art,
                            sgdb_id,
                        });
                    }
                }
            }
            let _ = tx.send(Event::ArtDone);
        });
    }

    pub fn start_tier_fetch(&mut self) {
        if self.tier_fetching || !self.cfg.protondb_enabled {
            return;
        }
        let pending: Vec<(String, u32)> = self
            .library
            .games
            .iter()
            .filter(|game| game.tier == Tier::Unknown)
            .filter_map(|game| {
                let app_id = game.app_id?;
                Some((game.id.clone(), app_id))
            })
            .filter(|(_, app_id)| !self.protondb_cache.contains_key(app_id))
            .collect();
        if pending.is_empty() {
            return;
        }
        self.tier_fetching = true;
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            for (id, app_id) in pending {
                let tier = protondb::fetch(app_id);
                let _ = tx.send(Event::Tier {
                    id,
                    tier: tier.unwrap_or(Tier::Unknown),
                });
                std::thread::sleep(Duration::from_millis(250));
            }
            let _ = tx.send(Event::TierDone);
        });
    }

    pub fn start_art_search(&mut self, id: String, query: String) {
        let key = self.cfg.steamgrid_key.clone();
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let results = steamgrid::Client::new(&key)
                .map(|client| client.search_games(&query))
                .unwrap_or_default();
            let _ = tx.send(Event::Candidates { id, results });
        });
    }

    pub fn apply_art_pick(&mut self, id: String, sgdb_id: u32) {
        self.candidates.clear();
        if let Some(game) = self.library.get_mut(&id) {
            game.sgdb_id = Some(sgdb_id);
            game.art = Art::default();
        }
        self.library.save();
        self.start_art_fetch(Some(id));
    }

    pub fn compute_size(&mut self, id: String) {
        let Some(game) = self.library.get(&id).cloned() else {
            return;
        };
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let bytes = scan::dir_size(&game.install_dir);
            let _ = tx.send(Event::Size { id, bytes });
        });
    }

    pub fn create_prefix(&mut self, id: String) {
        let Some(game) = self.library.get(&id).cloned() else {
            return;
        };
        let protons = self.protons.clone();
        let tx = self.tx.clone();
        self.log(format!("Creando prefijo para {}", game.title));
        std::thread::spawn(move || {
            let mut game = game;
            let result = prefix::ensure(&mut game, &protons);
            let _ = tx.send(Event::Prefix { id: game.id, result });
        });
    }

    /// Vuelve a detectar el prefijo pisando el valor actual, aunque exista.
    pub fn redetect_prefix(&mut self, id: String) {
        let Some(game) = self.library.get(&id).cloned() else {
            return;
        };
        self.log(format!("Buscando prefijo de {}", game.title));
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            match prefix::detect_for(&game) {
                Some((prefix, proton)) => {
                    let _ = tx.send(Event::PrefixDetected {
                        id: game.id,
                        prefix,
                        proton,
                        force: true,
                    });
                }
                None => {
                    let _ = tx.send(Event::PrefixMissing(game.id));
                }
            }
        });
    }

    pub fn play(&mut self, id: &str) {
        let Some(game) = self.library.get(id).cloned() else {
            return;
        };
        if !game.compatible() {
            self.toast(format!("{} no se puede lanzar en este sistema", game.title));
            return;
        }
        if self.running.is_some() {
            self.toast("Ya hay un juego en ejecución");
            return;
        }
        let plan = match launch::plan(&game, &self.protons) {
            Ok(plan) => plan,
            Err(err) => {
                self.toast(format!("{}: {err}", game.title));
                return;
            }
        };
        match plan.spawn() {
            Ok(child) => {
                self.log(format!("Iniciando {}", game.title));
                if let Some(game) = self.library.get_mut(id) {
                    game.last_played = Some(unix_now());
                }
                self.library.save();
                self.running = Some(Running {
                    id: id.to_string(),
                    child,
                    started: Instant::now(),
                });
            }
            Err(err) => self.toast(format!("No se pudo iniciar {}: {err}", game.title)),
        }
    }

    pub fn remove_game(&mut self, id: &str) {
        self.library.remove(id);
        if self.view == View::Detail(id.to_string()) {
            self.view = View::Library;
        }
        self.toast("Juego eliminado de la biblioteca");
    }

    pub fn toggle_favorite(&mut self, id: &str) {
        if let Some(game) = self.library.get_mut(id) {
            game.favorite = !game.favorite;
        }
        self.library.save();
    }

    pub fn log(&mut self, message: impl Into<String>) {
        self.log.push(message.into());
        if self.log.len() > 200 {
            self.log.remove(0);
        }
    }

    pub fn toast(&mut self, message: impl Into<String>) {
        self.toast = Some((message.into(), Instant::now()));
    }

    // --- ciclo principal -----------------------------------------------

    fn poll_events(&mut self) -> bool {
        let mut received = false;
        while let Ok(event) = self.rx.try_recv() {
            received = true;
            match event {
                Event::Scanned(games) => {
                    let found = games.len();
                    self.library.merge(games);
                    self.scanning = false;
                    self.log(format!("Escaneo completado: {found} juegos detectados"));
                    self.start_art_fetch(None);
                    self.start_tier_fetch();
                    self.start_prefix_scan();
                }
                Event::Art { id, art, sgdb_id } => {
                    if let Some(game) = self.library.get_mut(&id) {
                        for path in game.art.paths() {
                            self.textures.forget(path);
                        }
                        game.art = art;
                        if sgdb_id.is_some() {
                            game.sgdb_id = sgdb_id;
                        }
                    }
                    self.library.save();
                }
                Event::ArtDone => self.art_fetching = false,
                Event::Tier { id, tier } => {
                    if let Some(game) = self.library.get_mut(&id) {
                        game.tier = tier;
                        if let Some(app_id) = game.app_id {
                            self.protondb_cache.insert(app_id, tier);
                        }
                    }
                    self.library.save();
                }
                Event::TierDone => {
                    self.tier_fetching = false;
                    protondb::save_cache(&self.protondb_cache);
                }
                Event::Candidates { id, results } => {
                    self.candidates_for = Some(id);
                    self.candidates = results;
                }
                Event::PrefixMissing(id) => {
                    if let Some(game) = self.library.get(&id) {
                        let title = game.title.clone();
                        self.toast(format!("Sin prefijo detectado para {title}"));
                    }
                }
                Event::PrefixDetected { id, prefix, proton, force } => {
                    let proton = proton.map(|label| {
                        prefix::best_proton(&self.protons, &label).unwrap_or(label)
                    });
                    let mut updated = false;
                    if let Some(game) = self.library.get_mut(&id) {
                        let exists = game.prefix.as_ref().map(|p| p.exists()).unwrap_or(false);
                        if force || !exists {
                            game.prefix = Some(prefix);
                            if proton.is_some() {
                                game.proton = proton;
                            }
                            updated = true;
                        }
                    }
                    if updated {
                        self.library.save();
                    }
                }
                Event::PrefixScanDone => self.prefix_scanning = false,
                Event::Size { id, bytes } => {
                    if let Some(game) = self.library.get_mut(&id) {
                        game.size_bytes = Some(bytes);
                    }
                    self.library.save();
                }
                Event::Prefix { id, result } => match result {
                    Ok(prefix) => {
                        if let Some(game) = self.library.get_mut(&id) {
                            game.prefix = Some(prefix.clone());
                        }
                        self.library.save();
                        self.log(format!("Prefijo listo en {}", prefix.display()));
                    }
                    Err(err) => self.log(format!("Prefijo: {err}")),
                },
            }
        }
        received
    }

    fn tick(&mut self) {
        let finished = self.running.as_mut().and_then(|running| {
            match running.child.try_wait() {
                Ok(Some(_)) => Some((running.id.clone(), running.started.elapsed())),
                _ => None,
            }
        });
        if let Some((id, elapsed)) = finished {
            self.running = None;
            let minutes = (elapsed.as_secs() / 60).max(1) as u64;
            if let Some(game) = self.library.get_mut(&id) {
                game.playtime_min += minutes;
            }
            self.library.save();
            self.log(format!("Sesión finalizada (+{minutes} min)"));
        }
        if let Some((_, since)) = &self.toast {
            if since.elapsed() > Duration::from_secs(4) {
                self.toast = None;
            }
        }
    }

    fn draw_toast(&self, ctx: &Context) {
        let Some((message, _)) = &self.toast else {
            return;
        };
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::new("toast"),
        ));
        let font = egui::FontId::proportional(13.0);
        let galley = painter.layout_no_wrap(message.clone(), font, theme::TEXT);
        let size = galley.size() + egui::vec2(28.0, 18.0);
        let rect = egui::Rect::from_min_size(
            egui::pos2(
                ctx.content_rect().center().x - size.x / 2.0,
                ctx.content_rect().bottom() - size.y - 26.0,
            ),
            size,
        );
        painter.add(egui::Shape::convex_polygon(
            ui::widgets::rounded_points(rect, 12.0),
            theme::SURFACE_HIGH,
            egui::Stroke::new(1.0_f32, egui::Color32::from_white_alpha(24)),
        ));
        painter.galley(rect.min + egui::vec2(14.0, 9.0), galley, theme::TEXT);
    }

    fn handle_input(&mut self, ctx: &Context) {
        let navs: Vec<Nav> = if self.cfg.gamepad_enabled {
            self.gamepad.as_mut().map(|pad| pad.poll()).unwrap_or_default()
        } else {
            Vec::new()
        };
        for nav in navs {
            self.handle_nav(nav);
        }

        let (escape, enter, left, right, up, down) = ctx.input(|i| {
            (
                i.key_pressed(egui::Key::Escape),
                i.key_pressed(egui::Key::Enter),
                i.key_pressed(egui::Key::ArrowLeft),
                i.key_pressed(egui::Key::ArrowRight),
                i.key_pressed(egui::Key::ArrowUp),
                i.key_pressed(egui::Key::ArrowDown),
            )
        });
        if escape && self.view != View::Library {
            self.back();
        }
        if self.view == View::Library && !ctx.wants_keyboard_input() {
            for (pressed, nav) in [
                (left, Nav::Left),
                (right, Nav::Right),
                (up, Nav::Up),
                (down, Nav::Down),
                (enter, Nav::Confirm),
            ] {
                if pressed {
                    self.handle_nav(nav);
                }
            }
        }
    }

    fn back(&mut self) {
        self.view = View::Library;
    }

    fn handle_nav(&mut self, nav: Nav) {
        self.nav_mode = true;
        match nav {
            Nav::Menu => {
                self.view = match self.view {
                    View::Library => View::Settings,
                    View::Detail(_) | View::Settings => View::Library,
                };
            }
            Nav::Back => self.back(),
            Nav::Confirm => match self.view.clone() {
                View::Library => {
                    let selected = self.filtered().get(self.cursor).map(|g| g.id.clone());
                    if let Some(id) = selected {
                        self.selected = Some(id.clone());
                        self.view = View::Detail(id);
                    }
                }
                View::Detail(id) => self.play(&id),
                View::Settings => {}
            },
            Nav::Left | Nav::Right | Nav::Up | Nav::Down => match self.view.clone() {
                View::Library => {
                    let total = self.filtered().len();
                    if total == 0 {
                        return;
                    }
                    let columns = self.columns.max(1);
                    let next = match nav {
                        Nav::Left => self.cursor.saturating_sub(1),
                        Nav::Right => self.cursor + 1,
                        Nav::Up => self.cursor.saturating_sub(columns),
                        _ => self.cursor + columns,
                    };
                    self.cursor = next.min(total - 1);
                    if let Some(game) = self.filtered().get(self.cursor) {
                        self.selected = Some(game.id.clone());
                    }
                }
                _ => {}
            },
            Nav::PreviousSection | Nav::NextSection => {
                self.view = match self.view {
                    View::Settings => View::Library,
                    _ => View::Settings,
                };
            }
        }
    }

    fn apply_actions(&mut self, actions: Vec<Action>) {
        for action in actions {
            match action {
                Action::Open(id) => {
                    self.selected = Some(id.clone());
                    self.view = View::Detail(id);
                }
                Action::Play(id) => {
                    self.selected = Some(id.clone());
                    self.play(&id);
                }
                Action::Back => self.back(),
                Action::Scan => self.start_scan(),
                Action::FetchArt => {
                    self.start_art_fetch(None);
                    self.start_tier_fetch();
                }
                Action::RefreshArt(id) => self.start_art_fetch(Some(id)),
                Action::SearchArt { id, query } => self.start_art_search(id, query),
                Action::PickArt { id, sgdb_id } => self.apply_art_pick(id, sgdb_id),
                Action::ComputeSize(id) => self.compute_size(id),
                Action::CreatePrefix(id) => self.create_prefix(id),
                Action::DetectPrefix(id) => self.redetect_prefix(id),
                Action::Remove(id) => self.remove_game(&id),
                Action::Favorite(id) => self.toggle_favorite(&id),
                Action::OpenFolder(id) => {
                    if let Some(game) = self.library.get(&id) {
                        launch::open_in_file_manager(&game.install_dir);
                    }
                }
                Action::Select(id) => self.selected = Some(id),
            }
        }
    }

    fn paint_background(&self, ctx: &Context) {
        let screen = ctx.content_rect();
        let painter = ctx.layer_painter(egui::LayerId::background());
        let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
        let radius = if maximized { 0.0 } else { theme::RADIUS };
        painter.add(egui::Shape::convex_polygon(
            ui::widgets::rounded_points(screen, radius),
            theme::BG,
            egui::Stroke::NONE,
        ));
    }
}

impl eframe::App for App {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        // Los resultados de los hilos de trabajo llegan por canal y deben repintar.
        if self.poll_events() | self.textures.poll(ctx) {
            ctx.request_repaint();
        }
        if ctx.input(|i| i.pointer.delta() != egui::Vec2::ZERO || i.pointer.any_click()) {
            self.nav_mode = false;
        }
        self.tick();
        self.handle_input(ctx);
        self.paint_background(ctx);

        ui::titlebar::show(self, ctx);
        ui::rail::show(self, ctx);

        let mut actions = Vec::new();
        let view = self.view.clone();
        // El contenido vive a la derecha del rail y bajo la titlebar propia.
        let frame = egui::Frame::NONE.inner_margin(egui::Margin {
            left: (theme::RAIL_WIDTH + 20.0) as i8,
            right: 20,
            top: (theme::TITLEBAR_HEIGHT + 14.0) as i8,
            bottom: 14,
        });
        egui::CentralPanel::default().frame(frame).show(ctx, |ui| match view {
            View::Library => ui::library::show(self, ui, &mut actions),
            View::Detail(id) => ui::detail::show(self, ui, &mut actions, &id),
            View::Settings => ui::settings::show(self, ui, &mut actions),
        });

        self.apply_actions(actions);
        self.draw_toast(ctx);

        if self.scanning || self.art_fetching || self.tier_fetching || self.running.is_some() {
            ctx.request_repaint_after(Duration::from_millis(400));
        }
        if self.textures.is_busy() {
            ctx.request_repaint_after(Duration::from_millis(80));
        }
    }
}
