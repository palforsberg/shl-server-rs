use serde::Serialize;

use crate::models_api::athlete::{ApiAthleteStats, ApiAthlete};

#[derive(Serialize, Clone)]
pub struct LegacyPlayerStats {
    pub player: i32,
    pub team: String,
    pub firstName: String,
    pub familyName: String,
    pub position: String,
    pub jersey: i32,
    pub gp: Option<i32>,
    pub rank: Option<i32>,

    pub toi: Option<String>,
    pub g: Option<i32>,
    pub a: Option<i32>,
    pub sog: Option<i32>,
    pub pim: Option<i32>,
    pub toiSeconds: Option<i32>,
    pub pop: Option<i32>,
    pub nep: Option<i32>,
    
    // GK stats
    pub tot_svs: Option<i32>,
    pub tot_ga: Option<i32>,
    pub tot_soga: Option<i32>,
}


impl From<ApiAthlete> for LegacyPlayerStats {
    fn from(e: ApiAthlete) -> Self {
        let player_stats = match &e.stats { ApiAthleteStats::Player(e) => Some(e), _ => None };
        let gk_stats = match &e.stats { ApiAthleteStats::Goalkeeper(e) => Some(e), _ => None };
        LegacyPlayerStats { 
            player: e.id,
            team: e.team_code.clone(),
            firstName: e.first_name.clone(),
            familyName: e.family_name.clone(),
            position: e.position.clone(),
            jersey: e.jersey,
            gp: Some(player_stats.as_ref().map(|e| e.gp).unwrap_or_else(|| gk_stats.map(|e| e.gp).unwrap_or_default())),
            rank: None,
            toi: None,
            g: player_stats.as_ref().map(|e| e.g),
            a: player_stats.as_ref().map(|e| e.a),
            sog: player_stats.as_ref().map(|e| e.sog),
            pim: player_stats.as_ref().map(|e| e.pim),
            toiSeconds: player_stats.as_ref().map(|e| e.toi_s),
            pop: player_stats.as_ref().map(|e| e.plus_minus),
            nep: None,
            tot_svs: gk_stats.as_ref().map(|e| e.svs),
            tot_ga: gk_stats.as_ref().map(|e| e.ga),
            tot_soga: gk_stats.as_ref().map(|e| e.soga), 
        }
    }
}