use std::time::Duration;

use crate::rest_client::{self};
use crate::db::Db;
use crate::models::{GameType, League, SeasonKey, Season};
use crate::models_external::season::SeasonRsp;

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
        let db = Db::<String, SeasonRsp>::new("rest");
        
        for league in League::get_all() {
            for game_type in GameType::get_all() {
                let key = SeasonKey(season.clone(), league.clone(), game_type.clone());
                let url = rest_client::get_season_url(&key);
                if db.is_stale(&url, season.get_throttle()) {
                    if let Some(obj) = rest_client::throttle_call(&url, season.get_throttle()).await {
                        result.push((key.clone(), obj));
                        updated = true;
                    }
                } else if let Some(obj) = db.read(&url) {
                    result.push((key.clone(), obj));
                }
            }
        }

        (result, updated)
    }
}