use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct VoteBody {
    pub game_uuid: String,
    pub user_id: String,
    pub team_code: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, Copy)]
pub struct VotePerGame {
    pub home_count: u16,
    pub away_count: u16,
}


#[derive(Serialize, Deserialize, Debug, Clone, Default, Copy)]
pub struct ApiVotePerGame {
    pub home_perc: u16,
    pub away_perc: u16,
}

impl From<VotePerGame> for ApiVotePerGame {
    fn from(value: VotePerGame) -> Self {
        let tot = f64::from(value.home_count + value.away_count);
        ApiVotePerGame { 
            home_perc: ((f64::from(value.home_count) / tot) * 100.0).round() as u16,
            away_perc: ((f64::from(value.away_count) / tot) * 100.0).round() as u16
        }
    }
}