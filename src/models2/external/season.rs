use chrono::{DateTime, Utc, Duration};
use serde::{Deserialize, Serialize};

use crate::models::{StringOrNum, League};

fn default_TBD() -> String {
    "TBD".to_string()
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameTeamInfo {
    #[serde(default = "default_TBD")]
    pub code: String,
    pub score: StringOrNum,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SeriesInfo {
    pub code: League
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SeasonGame {
    pub uuid: String,
    pub awayTeamInfo: GameTeamInfo,
    pub homeTeamInfo: GameTeamInfo,
    pub startDateTime: DateTime<Utc>,
    pub state: String,
    pub shootout: bool,
    pub overtime: bool,

    pub seriesInfo: SeriesInfo
}

impl SeasonGame {
    pub fn is_potentially_live(&self) -> bool {
        let fifteen_min_in_future = Utc::now() + Duration::minutes(15);
        self.state == "pre-game" && (self.startDateTime < fifteen_min_in_future)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SeasonTeam {
    pub teamCode: String,
    pub teamInfo: Option<TeamInfo>,
    pub teamNames: TeamNames,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TeamInfo {
    pub golds: Option<String>,
    pub retiredNumbers: Option<String>,
    pub founded: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TeamNames {
    pub code: String,
    pub long: String,
    pub short: String,

}
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct SeasonRsp {
    pub gameInfo: Vec<SeasonGame>,
    pub teamList: Vec<SeasonTeam>,
}
