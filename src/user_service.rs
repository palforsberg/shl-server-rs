use serde::{Serialize, Deserialize};

use crate::db::Db;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct User {
    id: String,
    teams: Vec<String>,
    apn_token: Option<String>,
    ios_version: Option<String>,
    app_version: Option<String>,
}

pub struct UserService;

impl UserService {

    pub fn store(user: User) {
        let db = UserService::get_db();
        // parallel stores?
        let mut users = db.read(&"all".to_string()).unwrap_or_default();
        users.retain(|e| e.id != user.id);
        users.push(user);

        db.write(&"all".to_string(), &users);
    }

    pub fn get_users_for(team1: &str, team2: &str) -> Vec<User> {
        UserService::get_db()
            .read(&"all".to_string())
            .unwrap_or_default()
            .into_iter()
            .filter(|e| e.teams.contains(&team1.to_string()) || e.teams.contains(&team1.to_string()))
            .collect()
    }

    fn get_db() -> Db<String, Vec<User>> {
        Db::new("v2_users")
    }
}