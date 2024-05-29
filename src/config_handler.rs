use serde::{Deserialize, Serialize};
use std::fs;

use crate::models::League;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    pub port: u16,

    pub ha_url: String,
    pub shl_url: String,

    pub sse_url: String,

    pub apn_host: String,
    pub apn_key_path: String,
    pub apn_key_id: String,
    pub apn_team_id: String,
    pub apn_topic: String,

    #[serde(default="default_db_path")]
    pub db_path: String,

    pub api_key: String,

    pub api_admin_key: String,

    #[serde(default="default_sse_sleep")]
    pub sse_sleep: u64,

    #[serde(default="default_true")]
    pub sse_file_append: bool,

    #[serde(default="default_false")]
    pub poll: bool
}

fn default_db_path() -> String {
    "./db".to_string()
}

fn default_sse_sleep() -> u64 {
    100
}

fn default_true() -> bool {
    true
}

fn default_false() -> bool {
    false
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
    let path = std::env::var("CONFIG_PATH").ok()
        .unwrap_or_else(|| "./deployment/config.json".to_string());
    let data = fs::read_to_string(path.clone())
        .expect("Unable to read file");
    let mut result: Config = serde_json::from_str(&data)
        .unwrap_or_else(|_| panic!("{}", &format!("Could not parse JSON at {path}!")));
    if let Ok(db_path) = std::env::var("DB_PATH") {
        result.db_path = db_path;
        println!("[CONFIG] DB_PATH {}", result.db_path);
    }
    println!("[CONFIG] {:?}", result);
    result
}