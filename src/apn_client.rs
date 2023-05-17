
use std::fmt::Display;

use crate::{api_season_service::ApiGame, event_service::ApiGameEvent, CONFIG};
use axum::http::{HeaderMap, HeaderValue};
use chrono::{DateTime, Utc, Duration};
use jsonwebtoken::{Header, EncodingKey};
use reqwest::Response;
use serde::{Serialize};
use tracing::log;

pub struct ApnClient {
    base_url: String,
    team_id: String,
    key_id: String,
    key_path: String,
    token: Option<Token>
}

impl ApnClient {
    pub fn new() -> ApnClient {
        let mut a = ApnClient { 
            base_url: CONFIG.apn_host.to_string(), 
            team_id: CONFIG.apn_team_id.to_string(), 
            key_id: CONFIG.apn_key_id.to_string(), 
            key_path: CONFIG.apn_key_path.to_string(),
            token: None,
        };
        a.get_token().unwrap();
        a
    }

    pub async fn push<CT, D>(&mut self, push: &ApnPush<CT, D>, device_token: String) -> Result<Response, anyhow::Error> {
        let c = reqwest::Client::new();
        
        let response = c
            .post(format!("{}/{}", self.base_url, device_token))
            .bearer_auth(self.get_token()?)
            .headers(push.header.clone().try_into()?)
            .send()
            .await?;

        Ok(response)
    }

    fn get_token(&mut self) -> Result<String, anyhow::Error> {
        if let Some(token) = &self.token {
            if Utc::now() - token.expiration < Duration::minutes(55) {
                return Ok(token.value.to_string())
            }
        }
        let token = ApnClient::create_token(&self.team_id, &self.key_id, &self.key_path)?;
        let token_str = token.value.clone();
        self.token = Some(token);
        Ok(token_str)
    }

    fn create_token(team_id: &str, key_id: &str, key_path: &str) -> Result<Token, anyhow::Error> {
        let claims = Claims {
            iss: team_id.to_string(),
            iat: Utc::now().timestamp(),
        };
        let header = Header {
            alg: jsonwebtoken::Algorithm::ES256,
            kid: Some(key_id.to_string()),
            ..Default::default()
        };
        let key_file = std::fs::read(key_path)?;
        let key = EncodingKey::from_ec_pem(&key_file)?;
        let token = jsonwebtoken::encode(&header, &claims, &key)?;
        log::info!("[APN] Created token");
        Ok(Token { value: token, expiration: Utc::now() })
    }
}


struct Token {
    value: String,
    expiration: DateTime<Utc>,
}

#[derive(Serialize)]
struct Claims {
    iss: String,
    iat: i64,
}

#[derive(Serialize, Clone)]
pub struct ApnAlert {
    pub title: String,
    pub body: String,
    pub subtitle: Option<String>,
}

#[derive(Serialize)]
pub struct LiveActivityContentState {
    pub game: ApiGame,
    pub event: Option<ApiGameEvent>,
}

#[derive(Serialize, Default)]
#[serde(rename_all="kebab-case")]
pub struct ApnAps<T> {
    pub alert: Option<ApnAlert>,
    pub mutable_content: Option<u8>,
    pub content_available: Option<u8>,
    pub sound: Option<String>,
    pub badge: Option<u8>,

    pub event: Option<String>,
    pub relevance_score: Option<u8>,
    pub stale_date: Option<u32>,
    pub timestamp: Option<u32>,
    pub content_state: T,
}

#[derive(Serialize)]
#[serde(rename_all="kebab-case")]
pub struct ApnBody<CS, D> {
    pub aps: ApnAps<CS>,
    #[serde(flatten)]
    pub data: D,
}

pub struct ApnPush<CT, D> {
    pub header: ApnHeader,
    pub body: ApnBody<CT, D>,
}

#[derive(Clone, Serialize, PartialEq)]
pub enum ApnPushType {
    LiveActivity,
    Notification,
}
impl Display for ApnPushType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LiveActivity => write!(f, "liveactivity"),
            Self::Notification => write!(f, "notification"),
        }
    }
}

#[derive(Clone)]
pub struct ApnHeader {
    pub push_type: ApnPushType,
    pub priority: usize,
    pub topic: String,
    pub collapse_id: Option<String>,
    pub expiration: Option<u32>,
}

impl TryFrom<ApnHeader> for HeaderMap {
    type Error = anyhow::Error;
    fn try_from(value: ApnHeader) -> Result<Self, Self::Error> {
        let mut headers = HeaderMap::new();
        headers.insert("apns-priority", value.priority.into());
        headers.insert("apns-topic", HeaderValue::from_str(value.topic.as_str())?);
        headers.insert("apns-push-type", HeaderValue::from_str(value.push_type.to_string().as_str())?);
        if let Some(exp) = value.expiration {
            headers.insert("apns-expiration", exp.into());
        }
        if let Some(cid) = value.collapse_id {
            headers.insert("apns-collapse-id", HeaderValue::from_str(cid.as_str())?);
        }
        Ok(headers)
    }
}

