
use std::time::Duration;

use tracing::log;

use crate::{stats_service::StatsService, api_season_service::ApiSeasonService, player_service::PlayerService, event_service::EventService, db::Db, models_api::game::ApiGame, models::Season};


impl From<Season> for Duration {
    fn from(value: Season) -> Self {
        if value.is_current() {
            Duration::from_secs(60 * 60 * 24 * 10)
        } else {
            Duration::from_secs(0)
        }
    }
}

pub struct FetchDetailsService;
impl FetchDetailsService {
    pub async fn update() {
        let db: Db<String, String> = Db::new("v2_fetch_details");
        if !db.is_stale(&"key".to_string(), Some(Duration::from_secs(60 * 60))) {
            return;
        }
        let all_games = ApiSeasonService::read_all();
        let mut applicable_games: Vec<&ApiGame> = all_games.iter()
            .filter(|e| e.played)
            .filter(|e| 
                StatsService::is_stale(&e.league, &e.game_uuid, Some(e.season.clone().into())) || 
                PlayerService::is_stale(&e.league, &e.game_uuid, Some(e.season.clone().into())))
            .collect();

        let nr_games_left = applicable_games.len();
        if applicable_games.is_empty() {
            log::info!("[FETCHDETAILS] Done");
        }
        applicable_games.truncate(10);
        for e in applicable_games {
            log::info!("[FETCHDETAILS] {}", e.game_uuid);
            futures::join!(
                StatsService::update(&e.league, &e.season, &e.game_uuid, Some(e.season.clone().into())),
                PlayerService::update(&e.league, &e.game_uuid, Some(e.season.clone().into())),
                EventService::update(&e.season, &e.game_uuid, Some(e.season.clone().into()))
            );
            
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        let info = format!("{} out of {} left", nr_games_left, all_games.len());
        log::info!("[FETCHDETAILS] {info}");
        _ = db.write(&"key".to_string(), &info);
    }
}