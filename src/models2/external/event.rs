use serde::{Serialize, Deserialize};

use crate::{models::StringOrNum, game_report_service::GameStatus};



#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameReport {
    pub gameUuid: String,

    pub gameTime: String,
    pub statusString: String,
    pub gameState: String,
    pub period: StringOrNum,

    pub homeTeamId: Option<String>,
    pub awayTeamId: Option<String>,
    pub homeTeamScore: StringOrNum,
    pub awayTeamScore: StringOrNum,
    pub revision: u16,
}
impl GameReport {
    pub fn get_status(&self) -> GameStatus {
        match self.gameState.as_str() {
            "NotStarted" => GameStatus::Coming,
            "GameEnded" => GameStatus::Finished,
            "Intermission" => GameStatus::Intermission,
            "PeriodBreak" => GameStatus::Intermission,
            "ShootOut" => GameStatus::Shootout,
            "OverTime" => GameStatus::Overtime,
            "Ongoing" => self.period.to_num().into(),
            _ => GameStatus::Coming,
        }
    }
}

impl From<i16> for GameStatus {
    fn from(value: i16) -> Self {
        match value {
            1 => GameStatus::Period1,
            2 => GameStatus::Period2,
            3 => GameStatus::Period3,
            4..=10 => GameStatus::Overtime,
            99 => GameStatus::Shootout,
            _ => GameStatus::Period1,
        }
    }
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct General {
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Location {
    pub x: f32,
    pub y: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Shot {
    pub team: String,
    pub location: Location,
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GoalExtra {

    pub scorerLong: String,
    pub teamAdvantage: String,
    pub homeAgainst: StringOrNum,
    pub homeForward: StringOrNum,
    pub assist: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Goal {
    pub team: String,
    pub location: Location,
    pub extra: GoalExtra,
}



#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PeriodExtra {
    pub gameStatus: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Period {
    pub extra: PeriodExtra,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Penalty {
    pub team: String,
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlayByPlay {
    pub eventId: i32,
    pub revision: u16,
    pub hash: String,
    pub period: StringOrNum,
    pub gametime: String,
    pub description: String,

    #[serde(flatten)]
    pub class: PlayByPlayType,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "class")]
pub enum PlayByPlayType {
    Period(Period),
    Goal(Goal),
    Penalty(Penalty),
    
    PenaltyShot(Shot),
    Shot(Shot),
    ShotBlocked(Shot),
    ShotIron(Shot),
    ShotWide(Shot),
    ShootoutPenaltyShot(Shot),

    General(General),
    Timeout(General),
    GoolkeeperEvent(General),
    #[serde(rename = "Livefeed_SHL")]
    Livefeed(General),
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Action {
    pub actions: Vec<PlayByPlay>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SseEvent {
    pub gameReport: Option<GameReport>,
    pub playByPlay: Option<Action>,
}