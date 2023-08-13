use std::{collections::HashMap, sync::Arc, time::Instant};

use serde::{Serialize, Deserialize};
use tokio::sync::{RwLock, mpsc::Sender};
use tracing::log;

use crate::{db::Db, models_api::vote::VotePerGame};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Vote {
    pub user_id: String,
    pub game_uuid: String,
    pub team_code: String,
    pub is_home_winner: bool, // is home team picked as winner
}



pub struct VoteService {
    db: Db<String, Vec<Vote>>,
    in_mem_per_game: HashMap<String, VotePerGame>,
    on_vote: Sender<(String, VotePerGame)>,
}
pub type SafeVoteService = Arc<RwLock<VoteService>>;
impl VoteService {
    pub fn new(
        on_vote: Sender<(String, VotePerGame)>,
    ) -> SafeVoteService {
        let db = Db::new("v2_votes"); 
        let in_mem_per_game = VoteService::generate_per_game(&db.read(&"all".to_string()).unwrap_or_default());
        Arc::new(RwLock::new(VoteService { db, in_mem_per_game, on_vote }))
    }

    pub async fn vote(&mut self, vote: Vote) -> VotePerGame {
        let before = Instant::now();
        let mut all_votes = self.db.read(&"all".to_string()).unwrap_or_default();

        all_votes.retain(|e| !(e.game_uuid == vote.game_uuid && e.user_id == vote.user_id));
        all_votes.push(vote.clone());

        _ = self.db.write(&"all".to_string(), &all_votes);
        
        self.in_mem_per_game = VoteService::generate_per_game(&all_votes);

        let result = VoteService::get_aggregate(&all_votes.into_iter().filter(|e| e.game_uuid == vote.game_uuid).collect::<Vec<Vote>>());
        log::info!("[VOTE] Vote in {:.2?}", before.elapsed());

        if let Some(vote_per_game) = self.in_mem_per_game.get(&vote.game_uuid) {
            _  = self.on_vote.send((vote.game_uuid, *vote_per_game)).await;
        }
        result
    }

    pub fn get(&self, game_uuid: &str) -> Option<VotePerGame> {
        self.in_mem_per_game.get(game_uuid).copied()
    }

    pub fn get_all(&self) -> HashMap<String, VotePerGame> {
        self.in_mem_per_game.clone()
    }

    fn get_aggregate(entry: &[Vote]) -> VotePerGame {
        entry.iter().fold(VotePerGame { home_count: 0, away_count: 0 }, |mut a, b| {
            if b.is_home_winner {
                a.home_count += 1;
            } else {
                a.away_count += 1;
            }
            a
        })
    }

    fn generate_per_game(all_votes: &Vec<Vote>) -> HashMap<String, VotePerGame> {
        let mut votes_per_game = HashMap::new();
        for k in all_votes {
            votes_per_game.entry(k.game_uuid.clone()).or_insert_with(Vec::new).push(k.clone());
        }

        let mut result = HashMap::with_capacity(votes_per_game.len());
        for (key, value) in votes_per_game {
            result.insert(key, VoteService::get_aggregate(&value));
        }
        result
    }
}

#[cfg(test)]
mod tests{
    use tempdir::TempDir;

    use crate::vote_service::Vote;

    use super::VoteService;

    fn before() {
        std::env::set_var("DB_PATH", TempDir::new("test").expect("dir to be created").path().to_str().unwrap());
    }

    #[tokio::test]
    async fn sunny_day() {
        // Given
        before();
        let (sender, _) = tokio::sync::mpsc::channel(1);
        let service = VoteService::new(sender);
        let vote = Vote { user_id: "user_id".to_string(), game_uuid: "game_uuid".to_string(), team_code: "team_code".to_string(), is_home_winner: true };
        let vote2 = Vote { user_id: "user_id2".to_string(), game_uuid: "game_uuid".to_string(), team_code: "team_code".to_string(), is_home_winner: true };

        // When
        service.write().await.vote(vote.clone()).await;
        service.write().await.vote(vote2.clone()).await;

        service.write().await.vote(vote.clone()).await;
        service.write().await.vote(vote2.clone()).await;

        service.write().await.vote(vote.clone()).await;
        service.write().await.vote(vote2.clone()).await;

        // Then
        let votes = service.read().await.db.read(&"all".to_string()).unwrap_or_default();
        assert_eq!(votes.len(), 2);
        assert!(votes.iter().any(|e| { e.game_uuid == vote.game_uuid && e.user_id == vote.user_id }));
        assert!(votes.iter().any(|e| { e.game_uuid == vote2.game_uuid && e.user_id == vote2.user_id }));
    }
}