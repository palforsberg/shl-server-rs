use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct AddUser {
    pub id: String,
    pub teams: Vec<String>,
    pub apn_token: Option<String>,
    pub ios_version: Option<String>,
    pub app_version: Option<String>,
}