pub mod protondb;
pub mod steamgrid;

use std::time::Duration;

use reqwest::blocking::Client;

pub fn client() -> Option<Client> {
    Client::builder()
        .user_agent("Auera/0.1")
        .timeout(Duration::from_secs(15))
        .build()
        .ok()
}
