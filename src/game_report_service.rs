use std::{fmt::Display, time::Duration};

use serde::{Serialize, Deserialize};

use crate::{db::Db, models_external, models_api::report::{ApiGameReport, GameStatus}, msg_bus::UpdateReport, models::League, rest_client};

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

impl From<models_external::event::GameReport> for ApiGameReport {
    fn from(value: models_external::event::GameReport) -> Self {
        ApiGameReport {
            game_uuid: value.gameUuid.clone(),
            gametime: value.gameTime.clone(),
            status: value.get_status(),
            home_team_code: value.homeTeamId.unwrap_or("TBD".to_string()),
            away_team_code: value.awayTeamId.unwrap_or("TBD".to_string()),
            home_team_result: value.homeTeamScore.to_num(),
            away_team_result: value.awayTeamScore.to_num(),
            overtime: None,
            shootout: None,
        }
    }
}

impl From<models_external::event::GameReport> for UpdateReport {
    fn from(value: models_external::event::GameReport) -> Self {
        UpdateReport {
            gametime: Some(value.gameTime.clone()),
            status: Some(value.get_status()),
            home_team_result: Some(value.homeTeamScore.to_num()),
            away_team_result: Some(value.awayTeamScore.to_num()),
            ..Default::default()
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
struct GameOverview {
    pub gameUuid: String,
    pub homeGoals: i16,
    pub awayGoals: i16,
    pub state: String,
    pub time: GameTime,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct GameTime {
    period: i16,
    periodTime: String,
}

impl From<GameOverview> for UpdateReport {
    fn from(v: GameOverview) -> Self {
        let status = GameStatus::get_from(&v.state, v.time.period);
        UpdateReport {
            gametime: Some(v.time.periodTime),
            status: Some(status.clone()),
            home_team_result: Some(v.homeGoals),
            away_team_result: Some(v.awayGoals),
            ..Default::default()
        }
    }
}

pub struct GameReportService;
impl GameReportService {
    pub async fn fetch_update(league: &League, game_uuid: &str, throttle_s: Option<Duration>) -> Option<UpdateReport> {
        let url = rest_client::get_report_url(league, game_uuid);
        let rsp: Option<GameOverview> = rest_client::throttle_call(&url, throttle_s).await;
        rsp.map(|e| e.into())
    }

    pub fn store(game_uuid: &str, report: &ApiGameReport) {
        let db = GameReportService::get_db();
        _ = db.write(&game_uuid.to_string(), report);
    }
    pub fn read(game_uuid: &str) -> Option<ApiGameReport> {
        let db = GameReportService::get_db();
        db.read(&game_uuid.to_string())
    }

    fn get_db() -> Db<String, ApiGameReport> {
        Db::<String, ApiGameReport>::new("v2_report")
    }
}
