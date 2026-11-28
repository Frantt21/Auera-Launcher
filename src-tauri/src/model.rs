use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::i18n::{tr, Lang, Txt};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Platform {
    Windows,
    Linux,
}

impl Platform {
    pub fn current() -> Self {
        if cfg!(windows) {
            Self::Windows
        } else {
            Self::Linux
        }
    }

    pub fn label(self, lang: Lang) -> &'static str {
        match self {
            Self::Windows => tr(lang, Txt::Windows),
            Self::Linux => tr(lang, Txt::Linux),
        }
    }

    pub fn from_executable(path: &PathBuf) -> Self {
        match path.extension().and_then(|e| e.to_str()) {
            Some("exe") => Self::Windows,
            _ => Self::Linux,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    Steam,
    Epic,
    Heroic,
    Hydra,
    Desktop,
    Manual,
}

impl Source {
    pub fn label(self) -> &'static str {
        match self {
            Self::Steam => "Steam",
            Self::Epic => "Epic Games",
            Self::Heroic => "Heroic",
            Self::Hydra => "Hydra",
            Self::Desktop => "Desktop",
            Self::Manual => "Manual",
        }
    }

    pub const ALL: [Source; 6] = [
        Source::Steam,
        Source::Epic,
        Source::Heroic,
        Source::Hydra,
        Source::Desktop,
        Source::Manual,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tier {
    #[default]
    Unknown,
    Borked,
    Bronze,
    Silver,
    Gold,
    Platinum,
}

impl Tier {
    pub fn parse(raw: &str) -> Self {
        match raw.to_ascii_lowercase().as_str() {
            "borked" => Self::Borked,
            "bronze" => Self::Bronze,
            "silver" => Self::Silver,
            "gold" => Self::Gold,
            "platinum" => Self::Platinum,
            _ => Self::Unknown,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Unknown => "—",
            Self::Borked => "Borked",
            Self::Bronze => "Bronze",
            Self::Silver => "Silver",
            Self::Gold => "Gold",
            Self::Platinum => "Platinum",
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Art {
    pub hero: Option<PathBuf>,
    pub cover: Option<PathBuf>,
    pub logo: Option<PathBuf>,
    pub icon: Option<PathBuf>,
}

impl Art {
    pub fn is_empty(&self) -> bool {
        self.hero.is_none() && self.cover.is_none() && self.logo.is_none() && self.icon.is_none()
    }

    /// Arte pendiente cuando falta alguna pieza o su archivo ya no existe.
    pub fn needs_download(&self) -> bool {
        let missing = |path: &Option<PathBuf>| path.as_ref().map(|p| !p.is_file()).unwrap_or(true);
        missing(&self.cover) || missing(&self.hero) || missing(&self.logo) || missing(&self.icon)
    }

    pub fn paths(&self) -> Vec<&PathBuf> {
        [&self.hero, &self.cover, &self.logo, &self.icon]
            .into_iter()
            .flatten()
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub id: String,
    pub title: String,
    pub source: Source,
    #[serde(default)]
    pub app_id: Option<u32>,
    pub executable: Option<PathBuf>,
    /// Elegido a mano: los escaneos no lo sobrescriben.
    #[serde(default)]
    pub manual_exe: bool,
    pub install_dir: PathBuf,
    pub platform: Platform,
    #[serde(default)]
    pub platform_locked: bool,
    /// Bloqueo manual: el juego no se muestra como lanzable en este sistema.
    #[serde(default)]
    pub blocked: bool,
    #[serde(default)]
    pub prefix: Option<PathBuf>,
    #[serde(default)]
    pub proton: Option<String>,
    #[serde(default)]
    pub size_bytes: Option<u64>,
    #[serde(default)]
    pub playtime_min: u64,
    #[serde(default)]
    pub last_played: Option<i64>,
    #[serde(default)]
    pub favorite: bool,
    #[serde(default)]
    pub art: Art,
    #[serde(default)]
    pub sgdb_id: Option<u32>,
    #[serde(default)]
    pub tier: Tier,
}

impl Game {
    pub fn new(id: String, title: String, source: Source, install_dir: PathBuf) -> Self {
        Self {
            id,
            title,
            source,
            app_id: None,
            executable: None,
            manual_exe: false,
            install_dir,
            platform: Platform::current(),
            platform_locked: false,
            blocked: false,
            prefix: None,
            proton: None,
            size_bytes: None,
            playtime_min: 0,
            last_played: None,
            favorite: false,
            art: Art::default(),
            sgdb_id: None,
            tier: Tier::Unknown,
        }
    }

    /// Un juego de Windows es lanzable en Linux mediante Proton.
    pub fn compatible(&self) -> bool {
        if self.blocked {
            return false;
        }
        if cfg!(windows) {
            self.platform == Platform::Windows
        } else {
            true
        }
    }

    pub fn playtime_label(&self) -> String {
        match self.playtime_min {
            0 => "—".into(),
            m if m < 60 => format!("{m} min"),
            m => format!("{} h {} min", m / 60, m % 60),
        }
    }

    pub fn size_label(&self) -> String {
        match self.size_bytes {
            None => "—".into(),
            Some(bytes) => format_gb(bytes),
        }
    }
}

pub fn format_gb(bytes: u64) -> String {
    let gb = bytes as f64 / 1_073_741_824.0;
    if gb >= 0.1 {
        format!("{gb:.1} GB")
    } else {
        format!("{:.0} MB", bytes as f64 / 1_048_576.0)
    }
}

pub fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or_default()
}
