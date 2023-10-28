use std::fmt::Display;

use tokio::sync::broadcast::{Sender, Receiver, self};
use crate::{models_api::{event::ApiGameEvent, report::{ApiGameReport, GameStatus}}, LogResult, models_external::event::{LiveEvent, EventType}};

#[derive(Clone, Default)]
pub struct UpdateReport {
    pub gametime: Option<String>,
    pub status: Option<GameStatus>,

    pub home_team_result: Option<i16>,
    pub away_team_result: Option<i16>,

    pub overtime: Option<bool>,
    pub shootout: Option<bool>,
}
impl Display for UpdateReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "UpdateReport {:?} • {:?} :: {:?} - {:?}", 
        self.status, 
        self.gametime, 
        self.home_team_result, 
        self.away_team_result
    )
    }
}

impl UpdateReport {
    pub fn to_report(&self, old: &ApiGameReport) -> ApiGameReport {
        let overtime = match self.status {
            Some(GameStatus::Overtime) => Some(true),
            _ => match self.overtime {
                Some(a) => Some(a),
                _ => old.overtime,
            },
        };
        let shootout = match self.status {
            Some(GameStatus::Shootout) => Some(true),
            _ => match self.shootout {
                Some(a) => Some(a),
                _ => old.shootout,
            },
        };
        ApiGameReport { 
            game_uuid: old.game_uuid.clone(),
            gametime: self.gametime.clone().unwrap_or_else(|| old.gametime.clone()),
            status: self.status.clone().unwrap_or_else(|| old.status.clone()),
            home_team_code: old.home_team_code.clone(),
            away_team_code: old.away_team_code.clone(),
            home_team_result: self.home_team_result.unwrap_or(old.home_team_result),
            away_team_result: self.away_team_result.unwrap_or(old.away_team_result),
            overtime,
            shootout,
        }
    }

    pub fn from(value: &LiveEvent) -> UpdateReport {
        match &value.get_event_type() {
            EventType::Goal(e) => UpdateReport {
                gametime: Some(e.time.clone()),
                status: Some(GameStatus::get_from(&e.gameState, value.period.to_num())),
                home_team_result: Some(e.homeTeam.score.to_num()),
                away_team_result: Some(e.awayTeam.score.to_num()),
                ..Default::default()
            },
            EventType::Shot(e) => UpdateReport {
                gametime: Some(e.time.clone()),
                status: Some(GameStatus::get_from(&e.gameState, value.period.to_num())),
                ..Default::default()
            },
            EventType::Penalty(e) => UpdateReport {
                gametime: Some(e.time.clone()),
                status: Some(GameStatus::get_from(&e.gameState, value.period.to_num())),
                ..Default::default()
            },
            EventType::Period(a) => {
                let status = match (a.started, a.finished) {
                    (true, true) => (GameStatus::Intermission, "20:00"),
                    (_, _) => (GameStatus::get_from("Ongoing", value.period.to_num()), "00:00"),
                };
                UpdateReport {
                    gametime: Some(status.1.to_string()),
                    status: Some(status.0),
                    ..Default::default()
                }
            },
            EventType::Goalkeeper(a) => UpdateReport {
                status: Some(GameStatus::get_from(&a.gameState, value.period.to_num())),
                ..Default::default()
            },
            EventType::Unknown => UpdateReport { ..Default::default() },
        }
    }
}

#[derive(Clone)]
pub enum Msg {
    AddEvent { event: ApiGameEvent, game_uuid: String },
    UpdateReport { report: UpdateReport, game_uuid: String, forced: bool },
    SseClosed { game_uuid: String },
    ReportUpdated { report: ApiGameReport, game_uuid: String },
    EventUpdated { event: ApiGameEvent, game_uuid: String },
}

impl Msg {
    pub fn get_game_uuid(&self) -> &String {
        match self {
            Msg::SseClosed { game_uuid } => game_uuid,
            Msg::AddEvent { event:_, game_uuid } => game_uuid,
            Msg::UpdateReport { report:_, game_uuid, forced: _ } => game_uuid,
            Msg::ReportUpdated { report:_, game_uuid } => game_uuid,
            Msg::EventUpdated { event:_, game_uuid } => game_uuid,
         }
    }
}

pub struct MsgBus {
    sender: Sender<Msg>,
}

impl MsgBus {
    pub fn new() -> MsgBus {
        let (sender, _) = broadcast::channel(1000);
        MsgBus { sender }
    }

    pub fn subscribe(&self) -> Receiver<Msg> {
        self.sender.subscribe()
    }

    pub fn send(&self, msg: Msg) {
        self.sender.send(msg)
            .ok_log("[MSGBUS] Error sending");
    }
}

#[cfg(test)]
mod tests {
    use crate::models_api::report::ApiGameReport;

    use super::UpdateReport;

    #[test]
    fn test_shootout_overtime_to_report() -> Result<(), ()> {

        // Given
        let mut update = UpdateReport {
            ..Default::default()
        };
        let mut old = ApiGameReport {
            game_uuid: "game_uuid123".to_string(),
            gametime: "00:00".to_string(),
            status: crate::models_api::report::GameStatus::Period3,
            home_team_code: "LHF".to_string(),
            away_team_code: "SAIK".to_string(),
            home_team_result: 1,
            away_team_result: 0,
            overtime: None,
            shootout: None,
        };

        // When
        let updated = update.to_report(&old);

        // Then
        assert_eq!(updated.shootout, None);
        assert_eq!(updated.overtime, None);

        // Given
        old.overtime = Some(false);

        // When
        let updated = update.to_report(&old);

        // Then
        assert_eq!(updated.overtime, Some(false));

        // Given
        old.overtime = Some(true);

        // When
        let updated = update.to_report(&old);

        // Then
        assert_eq!(updated.overtime, Some(true));

        // Given
        old.overtime = None;
        update.status = Some(crate::models_api::report::GameStatus::Overtime);

        // When
        let updated = update.to_report(&old);

        // Then
        assert_eq!(updated.overtime, Some(true));

        // Given
        old.overtime = None;
        update.status = None;
        update.overtime = Some(true);

        // When
        let updated = update.to_report(&old);

        // Then
        assert_eq!(updated.overtime, Some(true));
        Ok(())
    }
}