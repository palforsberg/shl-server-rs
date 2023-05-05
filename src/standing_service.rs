use std::{fmt::Display, collections::HashMap, time::Instant};

use serde::{Deserialize, Serialize};
use tracing::log;

use crate::{db::Db, models::{League, Season, GameType}, api_season_service::ApiGame};


#[derive(Serialize, Deserialize, Debug, Clone, Hash, PartialEq, Eq)]
pub struct StandingKey  (pub League, pub Season);
impl Display for StandingKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}/{}", self.0, self.1)
    }
}

type TeamCode = String;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Standing {
    team_code: TeamCode,
    rank: u8,
    
    games_played: u16,
    points: u16,
    goal_diff: i16,
    league: League,
}

impl Standing {
    fn add_game(&mut self, g: &ApiGame) {
        self.games_played += 1;
        self.points += g.get_points_for(&self.team_code);
        self.goal_diff += g.get_goal_diff_for(&self.team_code);
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
        
        let league_map = games.iter()
            .filter(|e| e.game_type == GameType::Season)
            .filter(|e| e.played)
            .fold(HashMap::<League, Vec<&ApiGame>>::new(), |mut map, game| {
                map
                    .entry(game.league.clone())
                    .or_insert_with(Vec::new)
                    .push(game);
                map
            });
        
        for (key, val) in league_map {
            let standing_key = StandingKey(key, season.clone());
            let standings = StandingService::get_standings(val);
            _ = db.write(&standing_key, &standings);
        }

        log::info!("[STANDING] Updated in {:.0?}", before.elapsed());
    }

    fn get_standings(games: Vec<&ApiGame>) -> Vec<Standing> {
        let mut team_map = HashMap::<TeamCode, Standing>::new();
        for g in games {
            {
                team_map
                    .entry(g.home_team_code.clone())
                    .or_insert_with(|| Standing { team_code: g.home_team_code.clone(), rank: 0, games_played: 0, points: 0, league: g.league.clone(), goal_diff: 0 })
                    .add_game(g);
            } 
            {
                team_map
                    .entry(g.away_team_code.clone())
                    .or_insert_with(|| Standing { team_code: g.away_team_code.clone(), rank: 0, games_played: 0, points: 0, league: g.league.clone(), goal_diff: 0 })
                    .add_game(g);
            }
        }

        let mut all_teams: Vec<Standing> = team_map.values().cloned().collect();

        all_teams.sort_by(|a, b| {
            if a.points == b.points {
                b.goal_diff.partial_cmp(&a.goal_diff).unwrap()
            } else {
                b.points.partial_cmp(&a.points).unwrap() 
            }
        });

        all_teams.into_iter().enumerate().map(|mut e| {
            e.1.rank = u8::try_from(e.0).unwrap() + 1;
            e.1
        }).collect()
    }


    pub fn read_raw(league: League, season: Season) -> String {
        StandingService::get_db().read_raw(&StandingKey(league, season))
    }

    fn get_db() -> Db<StandingKey, Vec<Standing>> {
        Db::new("v2_standings")
    }
}