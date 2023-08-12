use std::{time::Instant, cmp::Ordering};

use tracing::log;

use crate::{game_report_service::GameReportService, models_api::{report::{ApiGameReport, GameStatus}, event::{ApiGameEvent, ApiEventType, GameEndInfo}}};

impl ApiGameReport {
    fn get_winner(&self) -> Option<String> {
        match self.home_team_result.cmp(&self.away_team_result) {
            Ordering::Greater => Some(self.home_team_code.clone()),
            Ordering::Less => Some(self.away_team_code.clone()),
            Ordering::Equal => None,
        }
    }    
}

pub struct ReportStateMachine {
    last: Option<GameStatus>,
}

impl ReportStateMachine {
    pub fn new() -> ReportStateMachine {
        ReportStateMachine {
            last: None,
        }
    }

    pub fn process(&mut self, report: &ApiGameReport) -> Option<ApiGameEvent> {
        let last_status = self.last
            .clone()
            .unwrap_or_else(|| ReportStateMachine::get_initial_status(&report.game_uuid).unwrap_or(GameStatus::Coming));

        let result = if last_status == GameStatus::Coming && report.status == GameStatus::Period1 {
            Some(ApiGameEvent { 
                game_uuid: report.game_uuid.clone(),
                event_id: "GameStarted".to_string(),
                revision: 1,
                status: GameStatus::Period1,
                gametime: "00:00".to_string(),
                description: "Nedsläpp".to_string(),
                info: ApiEventType::GameStart,
            })
        } else if last_status != GameStatus::Finished && report.status == GameStatus::Finished {
            Some(ApiGameEvent { 
                game_uuid: report.game_uuid.clone(),
                event_id: "GameEnded".to_string(), 
                revision: 1,
                status: GameStatus::Finished,
                gametime: report.gametime.clone(),
                description: "Matchen slutade".to_string(),
                info: ApiEventType::GameEnd(GameEndInfo { winner: report.get_winner() }),
            })
        } else {
            None
        };
        self.last = Some(report.status.clone());
        result
    }

    fn get_initial_status(game_uuid: &str) -> Option<GameStatus> {
        let before = Instant::now();
        let res = GameReportService::read(game_uuid)
            .map(|e| e.status);
        log::info!("[RSM] Get initial status {:?} {game_uuid} {:.2?}", res, before.elapsed());
        res
    } 
}