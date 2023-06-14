use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::{event_service::{ApiGameEvent, ApiEventType}, api_game_details::ApiGameDetails, game_report_service::GameStatus};

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
    pub events: Vec<LegacyGameEvent>,
    pub status: GameStatus,
    pub report: Option<LegacyGameReport>,
}

#[derive(Serialize, Clone, Default)]
pub struct LegacyGameReport {
    pub gametime: String,
    pub timePeriod: u8,
    pub period: u8,
    pub gameState: String,
}

#[derive(Serialize, Clone)]
pub struct LegacyGameEvent {
    #[serde(rename = "type")]
    event_type: String,
    info: LegacyGameEventInfo,
    timestamp: DateTime<Utc>,
    id: String,
    gametime: String,
}


#[derive(Serialize, Clone)]
pub struct LegacyGameEventInfo {
    homeTeamId: String,
    awayTeamId: String,
    homeResult: i16,
    awayResult: i16,

    team: Option<String>,
    player: Option<LegacyPlayer>,
    
    teamAdvantage: Option<String>,
    
    periodNumber: i16,
    
    penalty: Option<i16>,
    penaltyLong: Option<String>,
    reason: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct LegacyPlayer {
    firstName: String,
    familyName: String,
    jersey: i32,
}

impl ApiEventType {
    fn to_str(&self) -> &str {
        match self {
            ApiEventType::Goal(_) => "Goal",
            ApiEventType::PeriodStart => "PeriodStart",
            ApiEventType::PeriodEnd => "PeriodEnd",
            ApiEventType::Penalty(_) => "Penalty",
            ApiEventType::Shot(_) => "Shot",
            ApiEventType::GameStart => "GameStart",
            ApiEventType::GameEnd(_) => "GameEnd",
            ApiEventType::Timeout => "Timeout",
            ApiEventType::General => "General",
        }
    }
}
impl From<(ApiGameEvent, ApiGameDetails)> for LegacyGameEvent {
    fn from((event, details): (ApiGameEvent, ApiGameDetails)) -> Self {
        LegacyGameEvent { 
            event_type: event.info.to_str().to_string(),
            info: LegacyGameEventInfo { 
                homeTeamId: details.game.home_team_code, 
                awayTeamId: details.game.away_team_code,
                homeResult: match event.info.clone() { ApiEventType::Goal(a) => a.home_team_result, _ => details.game.home_team_result },
                awayResult: match event.info.clone() { ApiEventType::Goal(a) => a.away_team_result, _ => details.game.away_team_result },
                team: match event.info.clone() {
                    ApiEventType::Goal(a) => Some(a.team),
                    ApiEventType::Penalty(a) => Some(a.team),
                    _ => None,
                },
                player: match event.info.clone() {
                    ApiEventType::Goal(a) => a.player,
                    ApiEventType::Penalty(a) => a.player,
                    _ => None,
                }.map(|e| LegacyPlayer { firstName: e.first_name, familyName: e.family_name, jersey: e.jersey }),
                teamAdvantage:  match event.info.clone() {
                    ApiEventType::Goal(a) => Some(a.team_advantage),
                    _ => None,
                },
                periodNumber: match event.status {
                    GameStatus::Period1 => 1,
                    GameStatus::Period2 => 2,
                    GameStatus::Period3 => 3,
                    GameStatus::Overtime => 4,
                    GameStatus::Shootout => 99,
                    _ => 1,
                },
                penalty: None,
                penaltyLong: match event.info.clone() {
                    ApiEventType::Penalty(a) => a.penalty,
                    _ => None,
                },
                reason: match event.info {
                    ApiEventType::Penalty(a) => Some(a.reason),
                    _ => None,
                },
            }, 
            timestamp: Utc::now(),
            id: event.event_id,
            gametime: event.gametime 
        }
    }
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
            gameState: "".to_string(),
            events: value.events.clone().into_iter().map(|e| (e, value.clone()).into()).collect(),
            status: value.game.status,
            report: value.game.gametime.map(|e| LegacyGameReport { gametime: e, ..Default::default() }),
        }
    }
}