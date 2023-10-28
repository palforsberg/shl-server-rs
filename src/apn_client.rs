
use std::fmt::Display;

use crate::{CONFIG, models_api::report::GameStatus};
use axum::http::{HeaderMap, HeaderValue};
use chrono::{DateTime, Utc, Duration};
use jsonwebtoken::{Header, EncodingKey};
use reqwest::StatusCode;
use serde::{Serialize, Deserialize};
use tracing::log;

pub struct ApnClient {
    apn_host: String,
    team_id: String,
    key_id: String,
    key_path: String,
    token: Option<Token>
}

#[derive(Debug)]
pub enum ApnError {
    BadDeviceToken,
    Other,
}
impl Display for ApnError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadDeviceToken => write!(f, "BadDeviceToken"),
            Self::Other => write!(f, "Other"),
        }
    }
}
impl std::error::Error for ApnError {}

impl ApnClient {
    pub fn new() -> ApnClient {
        let mut a = ApnClient { 
            apn_host: CONFIG.apn_host.to_string(), 
            team_id: CONFIG.apn_team_id.to_string(), 
            key_id: CONFIG.apn_key_id.to_string(), 
            key_path: CONFIG.apn_key_path.to_string(),
            token: None,
        };
        a.update_token();
        a
    }

    pub async fn push_notification<CT : Serialize, D : Serialize>(&self, push: ApnPush<CT, D>, device_token: String) -> anyhow::Result<(), ApnError> {
        let c = match reqwest::Client::builder()
            .http2_prior_knowledge()
            .http2_keep_alive_interval(std::time::Duration::from_secs(60 * 55))
            .http2_keep_alive_timeout(std::time::Duration::from_secs(60 * 55))
            .http2_keep_alive_while_idle(true)
            .build() {
                Ok(e) => e,
                Err(_) => { return Err(ApnError::Other); }
            };
        let headers = match push.header.clone().try_into() {
            Ok(e) => e,
            Err(_) => { return Err(ApnError::Other); },
        };
        let response = match c
            .post(format!("{}/3/device/{}", self.apn_host, device_token))
            .bearer_auth(&self.token.as_ref().expect("").value)
            .headers(headers)
            .json(&push.body)
            .send()
            .await {
                Ok(e) => e,
                Err(e) => {
                    log::error!("[APN] Request failed {e}");
                    return Err(ApnError::Other);
                }
            };

        if response.status() == StatusCode::OK {
            log::debug!("[APN] Notified {}", device_token);
            return Ok(());
        }
        
        let body: ApnResponse = match response.json().await {
            Ok(e) => e,
            Err(e) => {
                log::error!("[APN] Failed parsing response {e}");
                return Err(ApnError::Other);
            } 
        };
        log::error!("[APN] Failed notifying {} {:?}", device_token, body.reason);

        if let Some(str) = body.reason {
            match str.as_str() {
                "BadDeviceToken" => Err(ApnError::BadDeviceToken),
                "Unregistered" => Err(ApnError::BadDeviceToken),
                _ => Err(ApnError::Other)
            }
        } else {
            Err(ApnError::Other)
        }
    }

    pub fn update_token(&mut self) {
        if let Some(token) = &self.token {
            if Utc::now() - token.expiration < Duration::minutes(55) {
                return
            }
        }
        let token = ApnClient::create_token(&self.team_id, &self.key_id, &self.key_path);
        self.token = Some(token);
    }

    fn create_token(team_id: &str, key_id: &str, key_path: &str) -> Token {
        let claims = Claims {
            iss: team_id.to_string(),
            iat: Utc::now().timestamp(),
        };
        let header = Header {
            alg: jsonwebtoken::Algorithm::ES256,
            kid: Some(key_id.to_string()),
            ..Default::default()
        };
        let key_file = std::fs::read(key_path).expect("[APN] Cant read key file");
        let key = EncodingKey::from_ec_pem(&key_file).expect("[APN] Cant encode token");
        let token = jsonwebtoken::encode(&header, &claims, &key).expect("[APN] Cant create jwt");
        log::info!("[APN] Created token");
        Token { value: token, expiration: Utc::now() }
    }
}


#[derive(Deserialize)]
pub struct ApnResponse {
    reason: Option<String>,
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
    pub report: LiveActivityReport,
    pub event: Option<LiveActivityEvent>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveActivityReport {
    pub home_score: i16,
    pub away_score: i16,
    pub status: Option<GameStatus>,
    pub gametime: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveActivityEvent {
    pub title: String,
    pub body: Option<String>,
    pub team_code: Option<String>,
}

#[derive(Serialize, Default)]
#[serde(rename_all="kebab-case")]
pub struct ApnAps<T : Serialize> {
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

#[derive(Serialize)]
#[serde(rename_all="kebab-case")]
pub struct ApnBody<CS : Serialize, D : Serialize> {
    pub aps: ApnAps<CS>,
    #[serde(flatten)]
    pub data: D,

    #[serde(rename="localAttachements")]
    pub local_attachements: Vec<String>,
}

pub struct ApnPush<CS : Serialize, D : Serialize> {
    pub header: ApnHeader,
    pub body: ApnBody<CS, D>,
}

#[derive(Clone, Serialize, PartialEq, Debug)]
pub enum ApnPushType {
    LiveActivity,
    Alert,
}
impl Display for ApnPushType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LiveActivity => write!(f, "liveactivity"),
            Self::Alert => write!(f, "alert"),
        }
    }
}

#[derive(Clone)]
pub struct ApnHeader {
    pub push_type: ApnPushType,
    pub priority: usize,
    pub topic: String,
    pub collapse_id: Option<String>,
    pub expiration: Option<i64>,
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

