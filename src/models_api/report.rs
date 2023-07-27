use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum GameStatus {
    Coming,
    Finished,
    Period1,
    Period2,
    Period3,
    Overtime,
    Shootout,
    Intermission,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApiGameReport {
    pub game_uuid: String,

    pub gametime: String,
    pub status: GameStatus,

    pub home_team_code: String,
    pub away_team_code: String,
    pub home_team_result: i16,
    pub away_team_result: i16,
}