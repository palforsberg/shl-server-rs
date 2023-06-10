use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

use crate::{game_report_service::GameStatus, models::{Season, GameType}, api_season_service::ApiGame};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LegacyGame {
    pub game_uuid: String,
    pub game_id: u8,
    pub home_team_code: String,
    pub away_team_code: String,
    pub home_team_result: i16,
    pub away_team_result: i16,
    pub start_date_time: DateTime<Utc>,
    pub status: GameStatus,
    pub penalty_shots: bool,
    pub overtime: bool,
    pub played: bool,
    pub game_type: String,
    pub season: Season,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub gametime: Option<String>,
}

fn get_legacy_game_type(e: GameType) -> String {
    match e {
        GameType::Season => "Regular season game".to_string(),
        GameType::PlayOff => "Playoff game".to_string(),
        GameType::Demotion => "Kvalmatch nedflyttning".to_string(),
    }
}
impl From<ApiGame> for LegacyGame {
    fn from(e: ApiGame) -> Self {
        LegacyGame { 
            game_uuid: e.game_uuid,
            game_id: 123,
            home_team_code: e.home_team_code,
            away_team_code: e.away_team_code,
            home_team_result: e.home_team_result,
            away_team_result: e.away_team_result,
            start_date_time: e.start_date_time,
            status: e.status,
            penalty_shots: e.shootout,
            overtime: e.overtime,
            played: e.played,
            game_type: get_legacy_game_type(e.game_type),
            season: e.season,
            gametime: e.gametime,
        }
    }
}