use serde::{Serialize, Deserialize};

use super::report::GameStatus;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApiUpdateReport {
    pub game_uuid: String,

    pub gametime: String,
    pub status: GameStatus,

    pub home_team_result: i16,
    pub away_team_result: i16,

    pub overtime: Option<bool>,
    pub shootout: Option<bool>,
}
