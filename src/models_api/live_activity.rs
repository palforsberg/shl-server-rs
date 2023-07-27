use serde::{Deserialize, Serialize};


#[derive(Serialize, Deserialize)]
pub struct StartLiveActivity {
    pub user_id: String,
    pub token: String,
    pub game_uuid: String,
}


#[derive(Serialize, Deserialize)]
pub struct EndLiveActivity {
    pub user_id: String,
    pub game_uuid: String,
}