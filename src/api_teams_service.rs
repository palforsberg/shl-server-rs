use std::{time::Instant};

use serde::{Serialize, Deserialize};
use tracing::log;

use crate::{db::Db, models2::external::season::{SeasonTeam}};


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApiTeam {
    code: String,
    display_code: String,
    name: String,
    shortname: String,
    golds: Vec<String>,
}

impl From<SeasonTeam> for ApiTeam {
    fn from(team: SeasonTeam) -> Self {
        ApiTeam {
            code: team.teamCode,
            display_code: team.teamNames.code,
            name: team.teamNames.long,
            shortname: team.teamNames.short,
            golds: team.teamInfo.and_then(|a| a.golds).map(|a| ApiTeamsService::parse_golds(&a)).unwrap_or_default(),
        }
    }
}
pub struct ApiTeamsService;

impl ApiTeamsService {
    pub fn add(teams: &[SeasonTeam]) -> Vec<ApiTeam> {
        let before = Instant::now();
        let db = ApiTeamsService::get_db();
        let mut existing = db.read(&"teams".to_string()).unwrap_or_default();
        for team in teams {
            match existing.iter().position(|e| e.code == team.teamCode) {
                Some(pos) => existing[pos] = team.clone().into(),
                None => existing.push(team.clone().into()),
            }
        }

        _ = db.write(&"teams".to_string(), &existing);
        log::info!("[TEAMS] wrote {} {:.0?}", existing.len(), before.elapsed());
        existing
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