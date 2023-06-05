use serde::Serialize;

use crate::{api_player_stats_service::{ApiPlayerSeasonStats, SeasonStats}};



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


impl From<ApiPlayerSeasonStats> for LegacyPlayerStats {
    fn from(e: ApiPlayerSeasonStats) -> Self {
        LegacyPlayerStats { 
            player: e.player_id,
            team: e.team.clone(),
            firstName: e.first_name.clone(),
            familyName: e.family_name.clone(),
            position: e.position.clone(),
            jersey: e.jersey,
            gp: Some(match e.stats { SeasonStats::Player(e) => e.gp, SeasonStats::Goalkeeper(e) => e.gp }),
            rank: None,
            toi: get_toi_string(e.stats),
            g: match e.stats { SeasonStats::Player(e) => Some(e.g), _ => None },
            a: match e.stats { SeasonStats::Player(e) => Some(e.a), _ => None },
            sog: match e.stats { SeasonStats::Player(e) => Some(e.sog), _ => None },
            pim: match e.stats { SeasonStats::Player(e) => Some(e.pim), _ => None },
            toiSeconds: match e.stats { SeasonStats::Player(e) => Some(e.toi_s), _ => None },
            pop: match e.stats { SeasonStats::Player(e) => Some(e.plus_minus), _ => None },
            nep: None,
            tot_svs: match e.stats { SeasonStats::Goalkeeper(e) => Some(e.svs), _ => None },
            tot_ga: match e.stats { SeasonStats::Goalkeeper(e) => Some(e.ga), _ => None },
            tot_soga: match e.stats { SeasonStats::Goalkeeper(e) => Some(e.soga), _ => None }, 
        }
    }
}

fn get_toi_string(e: SeasonStats) -> Option<String> {
    match e { 
        SeasonStats::Player(_) => Some("13.27".to_string()),
         _ => None
    }
    
}