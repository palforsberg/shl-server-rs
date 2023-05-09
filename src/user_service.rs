use serde::{Serialize, Deserialize};

use crate::{db::Db, event_service::ApiEventType, api::AddUser};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LiveActivityEntry {
    pub game_uuid: String,
    pub apn_token: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct User {
    pub id: String,
    pub teams: Vec<String>,
    pub apn_token: Option<String>,
    pub ios_version: Option<String>,
    pub app_version: Option<String>,    

    pub muted_games: Vec<String>,
    pub explicit_games: Vec<String>,
    pub live_activities: Vec<LiveActivityEntry>,

    pub event_types: Vec<ApiEventType>,
}

pub struct UserService;

impl UserService {

    pub fn handle(request: AddUser) {
        let db = UserService::get_db();
        // parallel stores?
        let user = db.read(&request.id);

        let updated_user = match user {
            Some(mut user) => {
                user.teams = request.teams;
                user.apn_token = request.apn_token;
                user.ios_version = request.ios_version;
                user.app_version = request.app_version;
                user
            },
            None => {
                User {
                    id: request.id,
                    teams: request.teams,
                    apn_token: request.apn_token,
                    ios_version: request.ios_version,
                    app_version: request.app_version,
                    ..Default::default()
                }
            }
        };

        _ = db.write(&updated_user.id, &updated_user);
    }

    pub fn stream_all() -> impl Iterator<Item = User> {
        UserService::get_db().stream_all()
    }

    pub fn remove_live_activity(user_id: &str, game_uuid: &str) {
        let db = UserService::get_db();
        if let Some(mut user) = db.read(&user_id.to_string()) {
            user.live_activities.retain(|e| e.game_uuid != game_uuid);
            _ = db.write(&user_id.to_string(), &user);
        }
    }

    pub fn remove_apn_token(user_id: &str) {
        let db = UserService::get_db();
        if let Some(mut user) = db.read(&user_id.to_string()) {
            user.apn_token = None;
            _ = db.write(&user_id.to_string(), &user);
        }
    }

    fn get_db() -> Db<String, User> {
        Db::new("v2_user")
    }
}