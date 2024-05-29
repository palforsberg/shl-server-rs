use std::time::Instant;

use serde::{Serialize, Deserialize};

use crate::{db::Db, models::{GameType, Season}, models_api::game::ApiGame, LogResult};
use tracing::log;
#[derive(Serialize, Deserialize, Clone)]
pub struct PlayoffEntry {
    pub team1: String,
    pub team2: String,
    pub score1: u8,
    pub score2: u8,
    pub eliminated: Option<String>,
    #[serde(default = "get_nr_games")]
    pub nr_games: u8,
}

fn get_nr_games() -> u8 {
    7
}

impl PlayoffEntry {
    fn clone_with(&self, score1: u8, score2: u8) -> PlayoffEntry {
        PlayoffEntry {
            team1: self.team1.clone(),
            team2: self.team2.clone(),
            score1,
            score2,
            eliminated: self.eliminated.clone(),
            nr_games: self.nr_games
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct PlayoffSeries {
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub eight: Vec<PlayoffEntry>,

    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub quarter: Vec<PlayoffEntry>,

    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub semi: Vec<PlayoffEntry>,

    #[serde(skip_serializing_if = "Option::is_none", default, rename="final")]
    pub final_: Option<PlayoffEntry>,

    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub demotion: Option<PlayoffEntry>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Playoffs {
    pub SHL: PlayoffSeries,
    pub HA: PlayoffSeries,
}

pub struct PlayoffService;
impl PlayoffService {
    pub fn update(season: &Season, games: &[ApiGame]) {
        let before = Instant::now();
        let db = PlayoffService::get_db();
        let applicable_games: Vec<ApiGame> = games.iter()
            .filter(|e| e.game_type != GameType::Season)
            .filter(|e| e.played)
            .cloned()
            .collect();
        if applicable_games.is_empty() {
            return
        }
        if let Some(play_offs) = db.read(season) {
            let updated = Playoffs {
                HA: PlayoffService::update_series(play_offs.HA, &applicable_games),
                SHL: PlayoffService::update_series(play_offs.SHL, &applicable_games),    
            };
            db.write_pretty(season, &updated, true)
                .ok_log("[PLAYOFF] failed to write");
            log::info!("[PLAYOFF] Updated in {:.0?}", before.elapsed());
        }
    }

    fn update_series(series: PlayoffSeries, applicable_games: &[ApiGame]) -> PlayoffSeries {
        PlayoffSeries {
            final_: series.final_.map(|e| {
                e.clone_with(
                    PlayoffService::get_score(&e.team1, &e.team2, applicable_games), 
                    PlayoffService::get_score(&e.team2, &e.team1, applicable_games))
            }),
            semi: series.semi.into_iter().map(|e| {
                e.clone_with(
                    PlayoffService::get_score(&e.team1, &e.team2, applicable_games), 
                    PlayoffService::get_score(&e.team2, &e.team1, applicable_games))
            }).collect(),
            quarter: series.quarter.into_iter().map(|e| {
                e.clone_with(
                    PlayoffService::get_score(&e.team1, &e.team2, applicable_games), 
                    PlayoffService::get_score(&e.team2, &e.team1, applicable_games))
            }).collect(),
            eight: series.eight.into_iter().map(|e| {
                e.clone_with(
                    PlayoffService::get_score(&e.team1, &e.team2, applicable_games), 
                    PlayoffService::get_score(&e.team2, &e.team1, applicable_games))
            }).collect(),
            demotion: series.demotion.map(|e| {
                e.clone_with(
                    PlayoffService::get_score(&e.team1, &e.team2, applicable_games), 
                    PlayoffService::get_score(&e.team2, &e.team1, applicable_games))
            }),
        }
    }

    fn get_score(team: &str, opponent: &str, games: &[ApiGame]) -> u8 {
        if team == "TBD" || opponent == "TBD" {
            return 0
        }
        games.iter()
            .filter(|e| e.home_team_code == team || e.home_team_code == opponent)
            .filter(|e| e.away_team_code == team || e.away_team_code == opponent)
            .fold(0, |e, agg| {
                e + match agg.did_team_win(team) {
                    true => 1,
                    false => 0,
                }
            })
    }
    
    pub fn get_db() -> Db<Season, Playoffs> {
        Db::new("v2_playoffs")
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use tempdir::TempDir;

    use crate::{models_api::{game::ApiGame, report::GameStatus}, playoff_service::{PlayoffEntry, PlayoffSeries, Playoffs}, LogResult};

    use super::PlayoffService;


    #[test]
    fn sunny_day() {
        std::env::set_var("DB_PATH", TempDir::new("test").expect("dir to be created").path().to_str().unwrap());
        let games = vec![
            get_played_game("game_uuid", "team1", "team2"),
            get_played_game("game_uuid", "team1", "team2"),
            get_played_game("game_uuid", "team2", "team1"),
            get_played_game("game_uuid", "team3", "team4"),
        ];
        let db = PlayoffService::get_db();
        db.write(&crate::models::Season::Season2023, &Playoffs {
            SHL: PlayoffSeries {
                quarter: vec![PlayoffEntry { 
                    team1: "team1".to_string(), 
                    team2: "team2".to_string(), 
                    score1: 4, 
                    score2: 6, 
                    eliminated: Some("team1".to_string()), 
                    nr_games: 7, 
                }],
                demotion: Some(PlayoffEntry {
                    team1: "team3".to_string(), 
                    team2: "team4".to_string(), 
                    score1: 4, 
                    score2: 6, 
                    eliminated: None, 
                    nr_games: 7, 
                }),
                ..Default::default()
            },
            HA: PlayoffSeries {
                ..Default::default()
            }
        }).ok_log("msg");

        PlayoffService::update(&crate::models::Season::Season2023, &games);
        
        let result = db.read(&crate::models::Season::Season2023);
        assert!(result.is_some());
        let play_off = result.unwrap();
        assert!(play_off.HA.final_.is_none());
        assert_eq!(play_off.SHL.quarter.len(), 1);
        let quarter = play_off.SHL.quarter.get(0).unwrap();
        assert_eq!(quarter.score1, 2);
        assert_eq!(quarter.score2, 1);
        assert_eq!(quarter.eliminated.as_ref().unwrap(), "team1");
        let demotion = play_off.SHL.demotion.unwrap();
        assert_eq!(demotion.score1, 1);
        assert_eq!(demotion.score2, 0);
        assert!(demotion.eliminated.is_none());
    }

    pub fn get_played_game(game_uuid: &str, team1: &str, team2: &str) -> ApiGame {
        ApiGame {
            game_uuid: game_uuid.to_string(),
            home_team_code: team1.to_string(),
            away_team_code: team2.to_string(),
            home_team_result: 3,
            away_team_result: 0,
            start_date_time: Utc::now(),
            status: GameStatus::Finished,
            shootout: false,
            overtime: false,
            played: true,
            game_type: crate::models::GameType::PlayOff,
            league: crate::models::League::SHL,
            season: crate::models::Season::Season2023,
            gametime: None,
            votes: None,
        }
    }
}