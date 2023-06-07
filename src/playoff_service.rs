use serde::{Serialize, Deserialize};

use crate::{models::Season, db::Db};


#[derive(Serialize, Deserialize, Clone)]
pub struct PlayoffEntry {
    pub team1: String,
    pub team2: String,
    pub score1: u8,
    pub score2: u8,
    pub eliminiated: Option<String>,
    pub nr_games: u8,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PlayoffSeries {
    #[serde(default)]
    pub eight: Vec<PlayoffEntry>,
    #[serde(default)]
    pub quarter: Vec<PlayoffEntry>,
    #[serde(default)]
    pub semi: Vec<PlayoffEntry>,
    #[serde(default, rename="final")]
    pub final_: Option<PlayoffEntry>,
    #[serde(default)]
    pub demotion: Option<PlayoffEntry>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Playoffs {
    pub SHL: PlayoffSeries,
    pub HA: PlayoffSeries,
}

pub struct PlayoffService;
impl PlayoffService {
    pub fn get_db() -> Db<Season, Playoffs> {
        Db::new("v2_playoffs")
    }
}