use serde::{Deserialize, Serialize};
use shl_server_rs::models_api::game::ApiGame;
use shl_server_rs::models_api::report::GameStatus;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ApnAlert {
    pub title: String,
    pub body: String,
    pub subtitle: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LiveActivityContentState {
    pub report: LiveActivityReport,
    pub event: Option<LiveActivityEvent>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LiveActivityReport {
    pub home_score: i16,
    pub away_score: i16,
    pub status: Option<GameStatus>,
    pub gametime: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LiveActivityEvent {
    pub title: String,
    pub body: Option<String>,
    pub team_code: Option<String>,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
#[serde(rename_all="kebab-case")]
pub struct ApnAps<T: Serialize> {
    pub alert: Option<ApnAlert>,
    pub mutable_content: Option<u8>,
    pub content_available: Option<u8>,
    pub sound: Option<String>,
    pub badge: Option<u8>,

    pub event: Option<String>,
    pub relevance_score: Option<u8>,
    pub stale_date: Option<i64>,
    pub timestamp: Option<i64>,
    pub content_state: T,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all="kebab-case")]
pub struct ApnBody {
    pub aps: ApnAps<Option<LiveActivityContentState>>,
    #[serde(flatten)]
    pub data: ApiGame,

    #[serde(rename="localAttachements")]
    pub local_attachements: Vec<String>,
}
