use serde::{Serialize, Deserialize};

use crate::{db::Db, models::League};


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApiTeam {
    pub code: String,
    pub display_code: String,
    pub name: String,
    pub shortname: String,
    pub golds: Vec<String>,
    pub league: Option<League>,
    pub founded: Option<String>,
    pub retired_numbers: Vec<String>,
}

pub struct ApiTeamsService;

impl ApiTeamsService {
    pub fn read() -> Vec<ApiTeam> {
        ApiTeamsService::get_db().read(&"teams".to_string()).unwrap_or_default()
    }

    pub fn read_raw() -> String {
        ApiTeamsService::get_db().read_raw(&"teams".to_string())
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