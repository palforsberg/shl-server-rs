use serde::{Serialize, Deserialize};

use super::{game::ApiGame, event::ApiGameEvent, stats::ApiGameStats, athlete::ApiAthlete, vote::ApiVotePerGame};

#[derive(Serialize, Deserialize, Clone)]
pub struct ApiGameDetails {
    pub events: Vec<ApiGameEvent>,
    pub stats: Option<ApiGameStats>,
    pub game: ApiGame,
    pub players: Vec<ApiAthlete>,
    pub votes: Option<ApiVotePerGame>,
}
