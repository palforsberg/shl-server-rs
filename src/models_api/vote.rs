use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct VoteBody {
    pub game_uuid: String,
    pub user_id: String,
    pub team_code: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct VotePerGame {
    pub home_count: u16,
    pub away_count: u16,
}