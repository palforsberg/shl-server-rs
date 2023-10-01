use serde::{Serialize, Deserialize};
use tracing::log;

use crate::{db::Db, models_api::{user::AddUser, live_activity::StartLiveActivity}};

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

    #[serde(default)]
    pub muted_games: Vec<String>,
    #[serde(default)]
    pub explicit_games: Vec<String>,
    #[serde(default)]
    pub live_activities: Vec<LiveActivityEntry>,
}

pub struct UserService;

impl UserService {

    pub fn handle(request: AddUser) {
        let db = UserService::get_db();
        // parallel stores?
        let user = db.read(&request.id);

        let teams: Vec<String> = request.teams.into_iter().map(|e| {
            match e.as_str() {
                "HERR" => "NVIF".to_string(),
                _ => e.to_string(),
            }
        }).collect();
        let updated_user = match user {
            Some(mut user) => {
                user.teams = teams;
                user.apn_token = request.apn_token;
                user.ios_version = request.ios_version;
                user.app_version = request.app_version;
                user
            },
            None => {
                User {
                    id: request.id,
                    teams,
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

    pub fn start_live_activity(req: &StartLiveActivity) {
        let db = UserService::get_db();
        if let Some(mut user) = db.read(&req.user_id.to_string()) {
            let entry = LiveActivityEntry { game_uuid: req.game_uuid.clone(), apn_token: req.token.clone() };
            user.live_activities.retain(|e| e.game_uuid != req.game_uuid);
            user.live_activities.push(entry);
            _ = db.write(&req.user_id.to_string(), &user);
        }
    }

    pub fn end_live_activity(user_id: &str, game_uuid: &str) {
        let db = UserService::get_db();
        if let Some(mut user) = db.read(&user_id.to_string()) {
            user.live_activities.retain(|e| e.game_uuid != game_uuid);
            log::info!("[USER] Remove live activity {user_id} {game_uuid}");
            _ = db.write(&user_id.to_string(), &user);
        }
    }

    pub fn remove_references_to(game_uuid: &str) {
        let db = UserService::get_db();
        let all_users = db.read_all();

        for mut user in all_users {
            user.live_activities.retain(|e| e.game_uuid != game_uuid);
            user.muted_games.retain(|e| e != game_uuid);
            user.explicit_games.retain(|e| e != game_uuid);

            _ = db.write(&user.id, &user);
        }
    }

    pub fn remove_apn_token(user_id: &str) {
        log::info!("[USER] Remove apn_token {user_id}");
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