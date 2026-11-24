pub mod detail;
pub mod library;
pub mod rail;
pub mod settings;
pub mod textures;
pub mod titlebar;
pub mod widgets;

pub use textures::Textures;

pub enum Action {
    Open(String),
    Play(String),
    Back,
    Scan,
    FetchArt,
    RefreshArt(String),
    SearchArt { id: String, query: String },
    PickArt { id: String, sgdb_id: u32 },
    ComputeSize(String),
    CreatePrefix(String),
    Remove(String),
    Favorite(String),
    OpenFolder(String),
    Select(String),
}
