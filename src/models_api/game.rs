use std::fmt::Display;

use chrono::{DateTime, Utc, Duration};
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

impl Display for ApiGame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} - {} {} :: {:#?}", self.home_team_code, self.home_team_result, self.away_team_result, self.away_team_code, self.status)
    }
}
impl ApiGame {
    pub fn is_potentially_live(&self) -> bool {
        let three_min_in_future = Utc::now() + Duration::minutes(3);
        self.status != GameStatus::Finished && (self.start_date_time < three_min_in_future)
    }
}