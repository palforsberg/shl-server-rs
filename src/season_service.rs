use std::time::Duration;

use futures::future::join_all;
use tracing::log;
use crate::rest_client::{self};
use crate::db::Db;
use crate::api_season_service::ApiSeasonService;
use crate::models::{GameType, League, SeasonKey, Season};
use crate::models2::external::season::{SeasonGame, SeasonRsp, SeasonTeam};

pub struct SeasonService {
}

impl Season {
    fn get_throttle(&self) -> Option<Duration> {
        if self == &Season::get_current() {
            Some(Duration::from_secs(60 * 60 * 10))
        } else {
            None
        }
    }
}
impl SeasonService {

    pub async fn update(&self, season: &Season) -> (Vec<(SeasonKey, SeasonRsp)>, bool) {
        let mut result = vec!();
        let mut updated = false;
        let db = Db::<SeasonKey, SeasonRsp>::new("v2_season");
        
        for league in League::get_all() {
            for game_type in GameType::get_all() {
                let key = SeasonKey(season.clone(), league.clone(), game_type.clone());

                if db.is_stale(&key, season.get_throttle()) {
                    if let Some(obj) = rest_client::get_season(&key).await {
                        let res = db.write(&key, &obj);
                        result.push((key.clone(), obj));
                        updated = true;
                    }
                } else if let Some(obj) = db.read(&key) {
                    result.push((key.clone(), obj));
                }
            }
        }

        (result, updated)
    }

    pub fn read_team(code: &str) -> Option<SeasonTeam> {
        Db::<SeasonKey, SeasonRsp>::new("v2_season")
            .read(&SeasonKey(Season::Season2022, League::SHL, GameType::Season))
            .map(|e: SeasonRsp| e.teamList)
            .unwrap_or(vec!())
            .into_iter()
            .find(|e| e.teamCode == code)
    }
    
    pub fn read_all_teams(season: &Season) -> Vec<SeasonTeam> {
        let db = Db::<SeasonKey, SeasonRsp>::new("v2_season");
        let mut shl_teams = db.read(&SeasonKey(season.clone(), League::SHL, GameType::Season))
            .map(|e: SeasonRsp| e.teamList)
            .unwrap_or_default();
        let ha_teams = db.read(&SeasonKey(season.clone(), League::HA, GameType::Season))
            .map(|e: SeasonRsp| e.teamList)
            .unwrap_or_default();
        shl_teams.extend(ha_teams);
        shl_teams

    }
}