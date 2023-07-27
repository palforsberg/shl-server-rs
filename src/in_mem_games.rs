use std::collections::HashMap;

use tracing::log;

use crate::models_api::game::ApiGame;

#[allow(dead_code)]
pub struct InMemGames {
    pub games: HashMap<String, ApiGame>,
}
impl InMemGames {
    #[allow(dead_code)]
    pub fn new(games: Vec<ApiGame>) -> InMemGames {
        let mut result = InMemGames { games: HashMap::new() };
        result.update(games);
        result
    }
    pub fn update(&mut self, games: Vec<ApiGame>) {
        for e in games {
            self.games.insert(e.game_uuid.clone(), e.clone());
        }

        log::info!("[INMEM] Updated to {} entries", self.games.len());
    }
}