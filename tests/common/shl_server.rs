use std::process::{Command, Child};

use assert_cmd::prelude::CommandCargoExt;
use predicates::{function::FnPredicate, Predicate};
use reqwest::Response;
use shl_server_rs::{models::Season, models_api::{game::ApiGame, user::AddUser, report::GameStatus, game_details::ApiGameDetails, standings::Standings, live_activity::StartLiveActivity, vote::VoteBody}, config_handler::Config};

pub struct ShlServer {
    port: u16,
    child_process: Option<Child>,
}

impl Drop for ShlServer {
    fn drop(&mut self) {
        if self.child_process.is_some() {
            self.child_process.as_mut().unwrap().kill()
                .expect("Should kill");
        }
    }
}

impl ShlServer {
    pub fn new(port: u16) -> ShlServer {
        ShlServer { port, child_process: None }
    }

    pub fn start(&mut self, path: &str, external_url: &str) {
        let config = Config {
            port: self.port,
            ha_url: external_url.to_string(),
            shl_url: external_url.to_string(),
            sse_url: format!("{external_url}/gameday/live/game/SHL"),
            apn_host: format!("{external_url}/apn/push"),

            apn_key_path: "./deployment/AuthKey_ZD8GX987XG.p8".to_string(),
            apn_key_id: "apn_key_id".to_string(), 
            apn_team_id: "apn_team_id".to_string(), 
            apn_topic: "com.integration.test".to_string(),

            db_path: format!("{}/db", path),
            api_admin_key: "API_KEY".to_string(),
            api_key: "API_KEY".to_string(),
            sse_sleep: 0,
            sse_file_append: false,
            ..Default::default()
        };

        let config_str = serde_json::to_string(&config).unwrap();
        let config_path = format!("{path}/config.json");
        std::fs::write(config_path.clone(), config_str).unwrap();
        let child_process = Command::cargo_bin("shl-server-rs")
            .unwrap()
            .env("CONFIG_PATH", config_path)
            .spawn()
            .expect("should start");

        self.child_process = Some(child_process);
    }

    pub async fn get_api_games(&self, season: Season) -> Result<Vec<ApiGame>, Box<dyn std::error::Error>> {
        Ok(reqwest::get(format!("http://localhost:{}/v2/games/{}", self.port, season))
                    .await?.json().await?)
    }

    pub async fn get_api_standings(&self, season: Season) -> Result<Standings, Box<dyn std::error::Error>> {
        Ok(reqwest::get(format!("http://localhost:{}/v2/standings/{}", self.port, season))
            .await?.json().await?)
    }

    pub async fn get_api_game_details(&self, game_uuid: &str) -> Result<ApiGameDetails, Box<dyn std::error::Error>> {
        Ok(reqwest::get(format!("http://localhost:{}/v2/game/{}", self.port, game_uuid))
            .await?.json().await?)
    }

    pub async fn retry_until_game_reaches(&self, game_uuid: &str, expected_status: &GameStatus, retry_ms: u64) -> ApiGameDetails {
        let predicate = predicates::function::function(|e: &ApiGameDetails| &e.game.status == expected_status);
        self.retry_until(game_uuid, predicate, retry_ms).await
    }

    pub async fn retry_until<F>(&self, game_uuid: &str, predicate: FnPredicate<F, ApiGameDetails>, retry_ms: u64) -> ApiGameDetails 
    where
        F: Fn(&ApiGameDetails) -> bool,
    {
        tokio::time::sleep(std::time::Duration::from_millis(5000)).await;
        let mut nr_loops = 0;
        loop {
            if let Ok(details) = self.get_api_game_details(game_uuid).await {
                if predicate.eval(&details) {
                    return details;
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(retry_ms)).await;
            nr_loops += 1;
            if nr_loops > 50 {
                panic!("retry failed");
            }
        }
    }

    pub async fn retry_add_user(&self, user: &AddUser) {
        while self.add_user(user).await.is_err() {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    }

    pub async fn add_user(&self, user: &AddUser) -> Result<Response, Box<dyn std::error::Error>> {
        Ok(reqwest::Client::builder()
            .build()?
            .post(format!("http://localhost:{}/v2/user", self.port))
            .json(&user)
            .send()
            .await?)
    }

    pub async fn vote(&self, vote: &VoteBody, api_key: Option<&str>) -> Result<Response, Box<dyn std::error::Error>> {
        Ok(reqwest::Client::builder()
            .build()?
            .post(format!("http://localhost:{}/v2/vote", self.port))
            .header("x-api-key", api_key.unwrap_or_default())
            .json(&vote)
            .send()
            .await?)
    }

    pub async fn start_live_acitivty(&self, req: &StartLiveActivity) -> Result<Response, Box<dyn std::error::Error>> {
        Ok(reqwest::Client::builder()
        .build()?
        .post(format!("http://localhost:{}/v2/live-activity/start", self.port))
        .json(&req)
        .send()
        .await?)
    }
}