
use std::fmt::Display;

use crate::{api_season_service::ApiGame, event_service::ApiGameEvent};
use axum::http::{HeaderMap, HeaderValue};
use chrono::{DateTime, Utc, Duration};
use jsonwebtoken::{Header, EncodingKey};
use reqwest::Response;
use serde::{Serialize};

pub struct ApnClient {
    base_url: String,
    team_id: String,
    key_id: String,
    token: Option<Token>
}

impl ApnClient {
    pub fn new(sandbox: bool) -> ApnClient {
        let api_version = 3;
        let host = match sandbox {
            true => "api.development.push.apple.com",
            false => "api.push.apple.com",
        };
        let base_url = format!("{host}/{api_version}");

        ApnClient { team_id: "team_id".to_string(), key_id: "key_id".to_string(), base_url, token: None }
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

    fn get_token(&mut self) -> Result<String, jsonwebtoken::errors::Error> {
        if let Some(token) = &self.token {
            if Utc::now() - token.expiration < Duration::minutes(55) {
                return Ok(token.value.to_string())
            }
        }
        let claims = Claims {
            iss: self.team_id.to_string(),
            iat: Utc::now().timestamp(),
        };
        let header = Header {
            alg: jsonwebtoken::Algorithm::ES256,
            kid: Some(self.key_id.to_string()),
            ..Default::default()
        };
        let key = EncodingKey::from_rsa_pem(b"")?;
        let token = jsonwebtoken::encode(&header, &claims, &key)?;
        self.token = Some(Token { value: token.clone(), expiration: Utc::now() });
        Ok(token)
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

