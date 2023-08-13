use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

use crate::models::{GameType, League, Season};

use super::{report::GameStatus, vote::ApiVotePerGame};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApiGame {
    pub game_uuid: String,
    pub home_team_code: String,
    pub away_team_code: String,
    pub home_team_result: i16,
    pub away_team_result: i16,
    pub start_date_time: DateTime<Utc>,
    pub status: GameStatus,
    pub shootout: bool,
    pub overtime: bool,
    pub played: bool,
    pub game_type: GameType,
    pub league: League,
    pub season: Season,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub gametime: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub votes: Option<ApiVotePerGame>,
}