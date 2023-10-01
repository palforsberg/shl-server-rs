use std::cmp::Ordering;

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

    pub overtime: Option<bool>,
    pub shootout: Option<bool>,
}


impl GameStatus {
    fn get_valid_steps(&self) -> Vec<GameStatus> {
        match self {
            Self::Coming => vec![GameStatus::Period1],
            Self::Period1 => vec![GameStatus::Intermission, GameStatus::Period2],
            Self::Period2 => vec![GameStatus::Intermission, GameStatus::Period3],
            Self::Period3 => vec![GameStatus::Intermission, GameStatus::Finished, GameStatus::Overtime],
            Self::Overtime => vec![GameStatus::Intermission, GameStatus::Finished, GameStatus::Shootout],
            Self::Shootout => vec![GameStatus::Intermission, GameStatus::Finished],
            Self::Intermission => vec![GameStatus::Period1, GameStatus::Period2, GameStatus::Period3, GameStatus::Overtime, GameStatus::Shootout, GameStatus::Finished],
            Self::Finished => vec![],
        }
    }
}
impl ApiGameReport {
    pub fn is_valid_update(&self, older: &Option<ApiGameReport>) -> bool {
        if let Some(older) = older {
            if older.status == self.status {
                self.gametime.cmp(&older.gametime) != Ordering::Equal ||
                self.home_team_result > older.home_team_result || 
                self.away_team_result > older.away_team_result
            } else {
                older.status.get_valid_steps().contains(&self.status)
            }            
        } else {
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::models_api::report::GameStatus;

    use super::ApiGameReport;

    #[test]
    fn test_valid_steps() {
        assert!(GameStatus::Coming.get_valid_steps().contains(&GameStatus::Period1));
        assert!(!GameStatus::Coming.get_valid_steps().contains(&GameStatus::Period2));

        assert!(GameStatus::Period1.get_valid_steps().contains(&GameStatus::Intermission));
        assert!(GameStatus::Period1.get_valid_steps().contains(&GameStatus::Period2));
        assert!(!GameStatus::Period1.get_valid_steps().contains(&GameStatus::Period3));
    }

    #[test]
    fn test_is_valid_update() {
        assert!(report("00:00", GameStatus::Coming, 0, 0).is_valid_update(&None));
        assert!(report("00:00", GameStatus::Period1, 0, 0).is_valid_update(&Some(report("00:00", GameStatus::Coming, 0, 0))));

        assert!(report("00:15", GameStatus::Period1, 0, 0).is_valid_update(&Some(report("00:00", GameStatus::Period1, 0, 0))));

        assert!(report("00:15", GameStatus::Period1, 1, 0).is_valid_update(&Some(report("00:15", GameStatus::Period1, 0, 0))));
        assert!(report("00:40", GameStatus::Period1, 1, 0).is_valid_update(&Some(report("00:15", GameStatus::Period1, 1, 0))));
        assert!(report("01:00", GameStatus::Period1, 1, 0).is_valid_update(&Some(report("00:40", GameStatus::Period1, 1, 0))));
        assert!(report("01:00", GameStatus::Period1, 2, 0).is_valid_update(&Some(report("01:00", GameStatus::Period1, 1, 0))));

        assert!(report("00:00", GameStatus::Intermission, 1, 0).is_valid_update(&Some(report("00:00", GameStatus::Period1, 1, 0))));

        assert!(report("20:00", GameStatus::Period2, 1, 0).is_valid_update(&Some(report("00:00", GameStatus::Intermission, 1, 0))));

        assert!(report("00:00", GameStatus::Intermission, 1, 0).is_valid_update(&Some(report("00:00", GameStatus::Period2, 1, 0))));

        assert!(report("00:00", GameStatus::Finished, 1, 0).is_valid_update(&Some(report("00:00", GameStatus::Period3, 1, 0))));
    }


    #[test]
    fn test_is_invalid_update() {
        assert!(!report("00:00", GameStatus::Coming, 0, 0).is_valid_update(&Some(report("00:00", GameStatus::Period1, 0, 0))));

        // assert!(!report("00:00", GameStatus::Period1, 0, 0).is_valid_update(&Some(report("00:15", GameStatus::Period1, 0, 0))));

        assert!(!report("00:15", GameStatus::Period1, 0, 0).is_valid_update(&Some(report("00:15", GameStatus::Period1, 1, 0))));
        assert!(!report("00:15", GameStatus::Period1, 1, 0).is_valid_update(&Some(report("00:15", GameStatus::Period1, 1, 0))));
        // assert!(!report("00:15", GameStatus::Period1, 1, 0).is_valid_update(&Some(report("00:40", GameStatus::Period1, 1, 0))));
        // assert!(!report("00:40", GameStatus::Period1, 1, 0).is_valid_update(&Some(report("01:00", GameStatus::Period1, 1, 0))));
        assert!(!report("01:00", GameStatus::Period1, 0, 0).is_valid_update(&Some(report("01:00", GameStatus::Period1, 1, 0))));

        assert!(!report("00:00", GameStatus::Period3, 1, 0).is_valid_update(&Some(report("00:00", GameStatus::Finished, 1, 0))));
    }

    fn report(gametime: &str, status: GameStatus, home_team_result: i16, away_team_result: i16) -> ApiGameReport {
        ApiGameReport { game_uuid: "uuid".to_string(),
            gametime: gametime.to_string(),
            status,
            home_team_code: "SAIK".to_string(),
            away_team_code: "MODO".to_string(),
            home_team_result,
            away_team_result,
            overtime: None, shootout: None,
        }
    }
}