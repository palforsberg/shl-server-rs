use serde::Serialize;

use crate::{event_service::ApiGameEvent, api_game_details::ApiGameDetails, game_report_service::GameStatus};

#[derive(Serialize, Clone)]
pub struct LegacyPeriod {
    pub periodNumber: i32,
    pub homeG: i32,
    pub awayG: i32,
    pub homeHits: i32,
    pub homeSOG: i32,
    pub homePIM: i32,
    pub homeFOW: i32,
    pub awayHits: i32,
    pub awaySOG: i32,
    pub awayPIM: i32,
    pub awayFOW: i32,
}

#[derive(Serialize, Clone)]
pub struct LegacyRecaps {
    pub gameRecap: LegacyPeriod,
}

#[derive(Serialize, Clone)]
pub struct LegacyGameDetails {
    pub recaps: LegacyRecaps,
    pub gameState: String,
    pub events: Vec<ApiGameEvent>,
    pub status: GameStatus,
}

impl From<ApiGameDetails> for LegacyGameDetails {
    fn from(value: ApiGameDetails) -> Self {
        let gameRecap = LegacyPeriod { periodNumber: 0,
            homeG: value.stats.as_ref().map(|e| e.home.g).unwrap_or_default(),
            awayG: value.stats.as_ref().map(|e| e.away.g).unwrap_or_default(),
            homeHits: 0,
            homeSOG: value.stats.as_ref().map(|e| e.home.sog).unwrap_or_default(),
            homePIM: value.stats.as_ref().map(|e| e.home.pim).unwrap_or_default(),
            homeFOW: value.stats.as_ref().map(|e| e.home.fow).unwrap_or_default(),
            awayHits: 0,
            awaySOG: value.stats.as_ref().map(|e| e.away.sog).unwrap_or_default(), 
            awayPIM: value.stats.as_ref().map(|e| e.away.pim).unwrap_or_default(), 
            awayFOW: value.stats.as_ref().map(|e| e.away.fow).unwrap_or_default()
        };
        LegacyGameDetails { 
            recaps: LegacyRecaps { gameRecap }, 
            gameState: "Finished".to_string(), 
            events: vec![], //value.events,
            status: value.game.status,
        }
    }
}