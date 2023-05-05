
use std::time::Duration;

use futures::future::{join_all, join};
use tracing::log;

use crate::{models2::external::season::SeasonGame, stats_service::StatsService, api_season_service::{ApiGame, ApiSeasonService}, player_service::PlayerService, event_service::EventService, db::Db};

pub struct FetchDetailsService;
impl FetchDetailsService {
    pub fn new() -> FetchDetailsService {
        FetchDetailsService { }
    }

    pub async fn update() {
        let db: Db<String, String> = Db::new("v2_fetch_details");
        if !db.is_stale(&"key".to_string(), Some(Duration::from_secs(60 * 60))) {
            return;
        }
        let all_games = ApiSeasonService::read_all();
        let mut applicable_games: Vec<&ApiGame> = all_games.iter()
            .filter(|e| e.played)
            .filter(|e| StatsService::is_stale(&e.league, &e.game_uuid) || PlayerService::is_stale(&e.league, &e.game_uuid))
            .collect();

        let nr_games_left = applicable_games.len();
        if applicable_games.is_empty() {
            log::info!("[FETCHDETAILS] Done");
        }
        applicable_games.truncate(10);
        for e in applicable_games {
            log::info!("[FETCHDETAILS] {}", e.game_uuid);
            futures::join!(
                StatsService::update(&e.league, &e.game_uuid, Some(Duration::from_secs(0))),
                PlayerService::update(&e.league, &e.game_uuid, Some(Duration::from_secs(0))),
                EventService::update(&e.game_uuid, Some(Duration::from_secs(0)))
            );
            
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        let info = format!("{} out of {} left", nr_games_left, all_games.len());
        log::info!("[FETCHDETAILS] {info}");
        db.write(&"key".to_string(), &info);
    }
}