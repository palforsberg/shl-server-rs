use std::{time::Instant};

use serde::{Serialize, Deserialize};
use tracing::log;

use crate::{db::Db, models2::external::season::{SeasonTeam}, models::League};


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApiTeam {
    code: String,
    display_code: String,
    name: String,
    shortname: String,
    golds: Vec<String>,
    league: League,
}

impl ApiTeam {
    fn from(team: SeasonTeam, league: League) -> Self {
        ApiTeam {
            code: team.teamCode,
            display_code: team.teamNames.code,
            name: team.teamNames.long,
            shortname: team.teamNames.short,
            golds: team.teamInfo.and_then(|a| a.golds).map(|a| ApiTeamsService::parse_golds(&a)).unwrap_or_default(),
            league,
        }
    }
}
pub struct ApiTeamsService;

impl ApiTeamsService {
    pub fn add(teams: &[SeasonTeam], league: League) -> Vec<ApiTeam> {
        let before = Instant::now();
        let db = ApiTeamsService::get_db();
        let mut existing = db.read(&"teams".to_string()).unwrap_or_default();
        for team in teams {
            let api_team = ApiTeam::from(team.clone(), league.clone());
            match existing.iter().position(|e| e.code == team.teamCode) {
                Some(pos) => existing[pos] = api_team,
                None => existing.push(api_team),
            }
        }

        _ = db.write(&"teams".to_string(), &existing);
        log::info!("[TEAMS] wrote {} {:.0?}", existing.len(), before.elapsed());
        existing
    }

    pub fn read() -> Vec<ApiTeam> {
        ApiTeamsService::get_db().read(&"teams".to_string()).unwrap_or_default()
    }

    pub fn read_raw() -> String {
        ApiTeamsService::get_db().read_raw(&"teams".to_string())
    }

    fn parse_golds(str: &str) -> Vec<String> {
        str.trim_start_matches('(')
            .trim_end_matches(')')
            .split(',')
            .map(|e| e.trim().to_string())
            .collect()
    }

    fn get_db() -> Db<String, Vec<ApiTeam>> {
        Db::new("v2_teams")
    }
}

pub struct TeamsMap {
    teams: Vec<ApiTeam>,
}
impl TeamsMap {
    pub fn new() -> TeamsMap {
        TeamsMap { teams: ApiTeamsService::read() }
    }

    pub fn get(&self, team_code: &str) -> Option<&ApiTeam> {
        self.teams.iter().find(|e| e.code == team_code)
    }

    pub fn get_display_code(&self, team_code: &str) -> String {
        match self.get(team_code) {
            Some(e) => e.display_code.to_string(),
            None => team_code.to_string(),
        }
    }

    pub fn get_shortname(&self, team_code: &str) -> String {
        match self.get(team_code) {
            Some(e) => e.shortname.to_string(),
            None => team_code.to_string(),
        }
    }
}