use std::cmp::Ordering;


use crate::models_api::{report::{ApiGameReport, GameStatus}, event::{ApiGameEvent, ApiEventType, GameEndInfo}};

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
}

impl ReportStateMachine {
    pub fn process(report: &ApiGameReport, old_report: &ApiGameReport) -> Option<ApiGameEvent> {
        let last_status = old_report.status.clone();
        
        if last_status == GameStatus::Coming && report.status == GameStatus::Period1 {
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
        }
    }
}