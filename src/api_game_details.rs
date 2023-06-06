use std::{time::{Instant}, sync::Arc};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{log};

use crate::{event_service::{EventService, ApiGameEvent}, api_season_service::{ApiGame, ApiSeasonService}, stats_service::{StatsService, ApiGameStats}, player_service::{Players, PlayerService}, game_report_service::GameStatus};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApiGameDetails {
    pub events: Vec<ApiGameEvent>,
    pub stats: Option<ApiGameStats>,
    pub game: ApiGame,
    pub players: Players,
}

#[derive(Clone)]
pub struct ApiGameDetailsService {
    api_season_service: Arc<RwLock<ApiSeasonService>>
}

impl ApiGameDetailsService {
    pub fn new(api_season_service: Arc<RwLock<ApiSeasonService>>) -> ApiGameDetailsService {
        ApiGameDetailsService { api_season_service }
    }
    pub async fn read(&self, game_uuid: &str) -> Option<ApiGameDetails> {
        let before = Instant::now();
        let game = self.api_season_service.read().await.read_game(game_uuid);
        if let Some(GameStatus::Coming) = game.as_ref().map(|e| e.status.clone()) {
            return Some(ApiGameDetails { game: game.unwrap(), events: vec!(), stats: None, players: Players::default() });
        }

        let game = game.as_ref()?;
        let (events, stats, players) = futures::join!(
            EventService::update(game_uuid, None),
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
