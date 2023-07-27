use serde::{Serialize, Deserialize};

use crate::models::Season;

#[derive(Serialize, Deserialize, Clone)]
pub struct ApiAthlete {
    pub id: i32,
    pub first_name: String,
    pub family_name: String,
    pub jersey: i32,
    pub team_code: String,
    pub position: String,
    pub season: Season,
    #[serde(flatten)]
    pub stats: ApiAthleteStats,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(tag="type")]
pub enum ApiAthleteStats {
    Player(ApiPlayerStats),
    Goalkeeper(ApiGoalkeeperStats),
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct ApiPlayerStats {
    #[serde(rename = "+/-")]
    pub plus_minus: i32,
    pub a: i32,
    pub fol: i32,
    pub fow: i32,
    pub g: i32,
    pub hits: i32,
    pub pim: i32,
    pub sog: i32,
    pub sw: i32,
    pub toi_s: i32,
    pub gp: i32,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct ApiGoalkeeperStats {
    pub ga: i32,
    pub soga: i32,
    pub spga: i32,
    pub svs: i32,
    pub gp: i32,
}
