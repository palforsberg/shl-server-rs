use std::{fmt::Display, collections::HashMap, time::Instant};

use serde::{Deserialize, Serialize};
use tracing::log;

use crate::{db::Db, models::{League, Season, GameType}, api_season_service::ApiGame};


#[derive(Serialize, Deserialize, Debug, Clone, Hash, PartialEq, Eq)]
pub struct StandingKey (pub Season);
impl Display for StandingKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

type TeamCode = String;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Standings {
    pub SHL: Vec<Standing>,
    pub HA: Vec<Standing>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Standing {
    team_code: TeamCode,
    rank: u8,
    
    gp: u16,
    points: u16,
    diff: i16,
    league: League,
}

impl Standing {
    fn add_game(&mut self, g: &ApiGame) {
        if g.played {
            self.gp += 1;
            self.points += g.get_points_for(&self.team_code);
            self.diff += g.get_goal_diff_for(&self.team_code);
        }
    }
}

impl ApiGame {
    fn did_team_win(&self, team_code: &str) -> bool {
        let winner = match self.home_team_result > self.away_team_result {
            true => &self.home_team_code,
            false => &self.away_team_code,
        };
        winner == team_code
    }
    fn get_points_for(&self, team_code: &str) -> u16 {
        debug_assert!(self.home_team_code == team_code || self.away_team_code == team_code);

        match (self.did_team_win(team_code), self.overtime || self.shootout) {
            (true, false) => 3,
            (true, true) => 2,
            (false, true) => 1,
            (false, false) => 0,
        }
    }
    fn get_goal_diff_for(&self, team_code: &str) -> i16 {
        debug_assert!(self.home_team_code == team_code || self.away_team_code == team_code);
        if team_code == self.home_team_code {
            self.home_team_result - self.away_team_result
        } else if team_code == self.away_team_code {
            self.away_team_result - self.home_team_result
        } else {
            panic!("unknown team");
        }
    }
}
pub struct StandingService;
impl StandingService {

    pub fn update(season: &Season, games: &[ApiGame]) {
        let db = StandingService::get_db();
        let before = Instant::now();
        
        let shl_games = games.iter()
            .filter(|e| e.game_type == GameType::Season)
            .filter(|e| e.league == League::SHL)
            .collect();
    
        let ha_games = games.iter()
            .filter(|e| e.game_type == GameType::Season)
            .filter(|e| e.league == League::HA)
            .collect();
        
        let standing_key = StandingKey(season.clone());
        let shl_standings = StandingService::get_standings(shl_games);
        let ha_standings = StandingService::get_standings(ha_games);
        _ = db.write(&standing_key, &Standings { SHL: shl_standings, HA: ha_standings });

        log::info!("[STANDING] Updated in {:.0?}", before.elapsed());
    }

    fn get_standings(games: Vec<&ApiGame>) -> Vec<Standing> {
        let mut team_map = HashMap::<TeamCode, Standing>::new();
        for g in games {
            {
                team_map
                    .entry(g.home_team_code.clone())
                    .or_insert_with(|| Standing { team_code: g.home_team_code.clone(), rank: 0, gp: 0, points: 0, league: g.league.clone(), diff: 0 })
                    .add_game(g);
            } 
            {
                team_map
                    .entry(g.away_team_code.clone())
                    .or_insert_with(|| Standing { team_code: g.away_team_code.clone(), rank: 0, gp: 0, points: 0, league: g.league.clone(), diff: 0 })
                    .add_game(g);
            }
        }

        let mut all_teams: Vec<Standing> = team_map.values().cloned().collect();

        all_teams.sort_by(|a, b| {
            if a.gp == 0 || b.gp == 0 {
                b.gp.partial_cmp(&a.gp).unwrap()
            } else if a.points == b.points {
                b.diff.partial_cmp(&a.diff).unwrap()
            } else {
                b.points.partial_cmp(&a.points).unwrap() 
            }
        });

        all_teams.into_iter().enumerate().map(|mut e| {
            e.1.rank = match e.1.gp {
                0 => 0,
                _ => u8::try_from(e.0).unwrap() + 1,
            };
            e.1
        }).collect()
    }


    pub fn read_raw(season: Season) -> String {
        StandingService::get_db().read_raw(&StandingKey(season))
    }

    pub fn read(season: Season) -> Option<Standings> {
        StandingService::get_db().read(&StandingKey(season))
    }

    fn get_db() -> Db<StandingKey, Standings> {
        Db::new("v2_standings")
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use tempdir::TempDir;

    use crate::api_season_service::ApiGame;

    use super::{StandingService};


    #[test]
    fn sunny_day() {
        std::env::set_var("DB_PATH", TempDir::new("test").expect("dir to be created").path().to_str().unwrap());
        
        let games = vec![
            get_played_game("game_uuid123", "LHF", "FHC"),
            get_coming_game("game_uuid124", "TIK", "MODO"),
        ];

        StandingService::update(&crate::models::Season::Season2023, &games);

        let standings = StandingService::read(crate::models::Season::Season2023).unwrap();
        assert_eq!(standings.SHL.len(), 4);
        assert_eq!(standings.HA.len(), 0);

        let lhf = standings.SHL.iter().find(|e| e.team_code == "LHF").unwrap();
        assert_eq!(lhf.gp, 1);
        assert_eq!(lhf.rank, 1);
        assert_eq!(lhf.points, 3);

        let fhc = standings.SHL.iter().find(|e| e.team_code == "FHC").unwrap();
        assert_eq!(fhc.gp, 1);
        assert_eq!(fhc.rank, 2);
        assert_eq!(fhc.points, 0);

        let tik = standings.SHL.iter().find(|e| e.team_code == "TIK").unwrap();
        assert_eq!(tik.gp, 0);
        assert_eq!(tik.rank, 0);
        assert_eq!(tik.points, 0);

        let modo = standings.SHL.iter().find(|e| e.team_code == "MODO").unwrap();
        assert_eq!(modo.gp, 0);
        assert_eq!(modo.rank, 0);
        assert_eq!(modo.points, 0);
    }

    pub fn get_played_game(game_uuid: &str, team1: &str, team2: &str) -> ApiGame {
        ApiGame {
            game_uuid: game_uuid.to_string(),
            home_team_code: team1.to_string(),
            away_team_code: team2.to_string(),
            home_team_result: 3,
            away_team_result: 0,
            start_date_time: Utc::now(),
            status: crate::game_report_service::GameStatus::Finished,
            shootout: false,
            overtime: false,
            played: true,
            game_type: crate::models::GameType::Season,
            league: crate::models::League::SHL,
            season: crate::models::Season::Season2022,
            gametime: None,
        }
    }
    pub fn get_coming_game(game_uuid: &str, team1: &str, team2: &str) -> ApiGame {
        ApiGame {
            game_uuid: game_uuid.to_string(),
            home_team_code: team1.to_string(),
            away_team_code: team2.to_string(),
            home_team_result: 3,
            away_team_result: 0,
            start_date_time: Utc::now(),
            status: crate::game_report_service::GameStatus::Coming,
            shootout: false,
            overtime: false,
            played: false,
            game_type: crate::models::GameType::Season,
            league: crate::models::League::SHL,
            season: crate::models::Season::Season2022,
            gametime: None,
        }
    }
}