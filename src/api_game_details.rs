use std::{time::Instant, sync::Arc};

use tokio::sync::RwLock;
use tracing::log;

use crate::{event_service::EventService, api_season_service::ApiSeasonService, stats_service::StatsService, player_service::PlayerService, models_api::{game_details::ApiGameDetails, report::GameStatus}};


#[derive(Clone)]
pub struct ApiGameDetailsService {
    api_season_service: Arc<RwLock<ApiSeasonService>>,
}

impl ApiGameDetailsService {
    pub fn new(
        api_season_service: Arc<RwLock<ApiSeasonService>>,
    ) -> ApiGameDetailsService {
        ApiGameDetailsService { api_season_service }
    }
    pub async fn read(&self, game_uuid: &str) -> Option<ApiGameDetails> {
        let before = Instant::now();
        let game = self.api_season_service.read().await.read_game(game_uuid);

        if let Some(GameStatus::Coming) = game.as_ref().map(|e| e.status.clone()) {
            return Some(ApiGameDetails { game: game.unwrap(), events: vec!(), stats: None, players: vec![] });
        }

        let game = game.as_ref()?;
        let (events, stats, players) = futures::join!(
            EventService::update(&game.season, game_uuid, None),
            StatsService::update(&game.league, game_uuid, None),
            PlayerService::update(&game.league, game_uuid, None),
        );

        
        let res = Some(ApiGameDetails {
            game: game.clone(),
            events: events.unwrap_or_default().into_iter().rev().collect(),
            stats,
            players,
        });

        log::debug!("[API.DETAILS] read {:.2?}", before.elapsed());
        res
    }
}
