use serde::{Deserialize};
use std::fs;

use crate::models::League;

#[derive(Debug, Deserialize, Default)]
pub struct Config {
    pub port: u16,

    pub ha_url: String,
    pub shl_url: String,

    pub sse_url: String,
}
impl Config {
    pub fn get_url(&self, league: &League) -> &str {
        match league {
            League::HA => self.ha_url.as_str(),
            League::SHL => self.shl_url.as_str(),
        }
    }
}

pub fn get_config() -> Config {
    let path = "./deployment/config.json";
    let data = fs::read_to_string(path)
        .expect("Unable to read file");
    serde_json::from_str(&data)
        .unwrap_or_else(|_| panic!("{}", &format!("Could not parse JSON at {path}!")))
}