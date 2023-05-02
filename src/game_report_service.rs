use std::fmt::Display;

use serde::{Serialize, Deserialize};

use crate::{db::Db, models2::external};

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

impl Display for ApiGameReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} - {} {} :: {:?} • {}",
            self.home_team_code,
            self.home_team_result,
            self.away_team_result,
            self.away_team_code,
            self.status,
            self.gametime
        )
    }
}

impl From<external::event::GameReport> for ApiGameReport {
    fn from(value: external::event::GameReport) -> Self {
        ApiGameReport {
            game_uuid: value.gameUuid.clone(),
            gametime: value.gameTime.clone(),
            status: value.get_status(),
            home_team_code: value.homeTeamId.unwrap_or("TBD".to_string()),
            away_team_code: value.awayTeamId.unwrap_or("TBD".to_string()),
            home_team_result: value.homeTeamScore.to_num(),
            away_team_result: value.awayTeamScore.to_num()
        }
    }
}

pub struct GameReportService;
impl GameReportService {
    pub fn store(game_uuid: &str, report: &ApiGameReport) {
        let db = GameReportService::get_db();
        db.write(&game_uuid.to_string(), report);
    }
    pub fn read(game_uuid: &str) -> Option<ApiGameReport> {
        let db = GameReportService::get_db();
        db.read(&game_uuid.to_string())
    }

    fn get_db() -> Db<String, ApiGameReport> {
        Db::<String, ApiGameReport>::new("v2_report")
    }
}
