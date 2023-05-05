use std::{collections::HashMap, sync::Arc};

use serde::{Serialize, Deserialize};
use tokio::sync::RwLock;

use crate::db::Db;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Vote {
    pub user_id: String,
    pub game_uuid: String,
    pub team_code: String,
    pub is_home_winner: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct VotePerGame {
    home_count: u16,
    away_count: u16,
}

pub struct VoteService {
    db: Db<String, Vec<Vote>>,
    in_mem_per_game: HashMap<String, VotePerGame>,
}
pub type SafeVoteService = Arc<RwLock<VoteService>>;
impl VoteService {
    pub fn new() -> SafeVoteService {
        let db = Db::new("v2_votes"); 
        let in_mem_per_game = VoteService::get_per_game(&db.read(&"all".to_string()).unwrap_or_default());
        Arc::new(RwLock::new(VoteService { db, in_mem_per_game, }))
    }

    pub fn vote(&mut self, vote: Vote) -> VotePerGame {
        let mut all_votes = self.db.read(&"all".to_string()).unwrap_or_default();

        all_votes.retain(|e| !(e.game_uuid == vote.game_uuid && e.user_id == vote.user_id));
        all_votes.push(vote.clone());

        _ = self.db.write(&"all".to_string(), &all_votes);
        
        self.in_mem_per_game = VoteService::get_per_game(&all_votes);

        VoteService::get_aggregate(&all_votes.into_iter().filter(|e| e.game_uuid == vote.game_uuid).collect::<Vec<Vote>>())
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

    fn get_per_game(all_votes: &Vec<Vote>) -> HashMap<String, VotePerGame> {
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