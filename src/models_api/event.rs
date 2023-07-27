use serde::{Serialize, Deserialize};

use super::report::GameStatus;

#[derive(PartialEq)]
pub enum ApiEventTypeLevel {
    Low, // only websocket
    Medium, // live activity, show in UI
    High // alert
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "type")]
pub enum ApiEventType {
    Goal(GoalInfo),
    PeriodEnd,
    PeriodStart,
    GameEnd(GameEndInfo),
    GameStart,
    Penalty(PenaltyInfo),
    Shot(ShotInfo),
    Timeout,
    General,
}
impl ApiEventType {
    pub fn get_level(&self) -> ApiEventTypeLevel {
        match self {
            Self::Goal(_) => ApiEventTypeLevel::High,
            Self::GameStart => ApiEventTypeLevel::High,
            Self::GameEnd(_) => ApiEventTypeLevel::High,
            Self::Penalty(_) => ApiEventTypeLevel::Medium,
            Self::PeriodStart => ApiEventTypeLevel::Medium,
            Self::PeriodEnd => ApiEventTypeLevel::Medium,
            Self::Timeout => ApiEventTypeLevel::Medium,
            Self::Shot(_) => ApiEventTypeLevel::Low,
            Self::General => ApiEventTypeLevel::Low,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApiGameEvent {
    pub game_uuid: String,
    pub event_id: String,
    pub revision: u16,
    pub status: GameStatus,
    pub gametime: String,
    pub description: String,
    #[serde(flatten)]
    pub info: ApiEventType,
}


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ShotInfo {
    pub team: String,
    pub location: Location,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct GameEndInfo {
    pub winner: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]

pub struct PenaltyInfo {
    pub team: String,
    pub player: Option<Player>,
    pub reason: String,
    pub penalty: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Location {
    pub x: f32,
    pub y: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]

pub struct GoalInfo {
    pub team: String,
    pub player: Option<Player>,
    pub team_advantage: String,
    pub assist: Option<String>,
    pub home_team_result: i16,
    pub away_team_result: i16,
    pub location: Location,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Player {
    pub first_name: String,
    pub family_name: String,
    pub jersey: i32,
}