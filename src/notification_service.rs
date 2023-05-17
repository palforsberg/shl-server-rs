use std::fmt::Display;

use chrono::{Utc, Timelike, Duration};

use crate::{event_service::{ApiGameEvent, ApiEventType}, api_season_service::ApiGame, user_service::{UserService, User}, apn_client::{ApnClient, ApnPush, ApnAlert, ApnBody, ApnHeader, ApnAps, LiveActivityContentState, ApnPushType}};


impl Display for ApiGame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} - {} {}", self.home_team_code, self.home_team_result, self.away_team_result, self.away_team_code)
    }
}

impl ApnAlert {
    fn from(game: &ApiGame, event: &ApiGameEvent) -> ApnAlert {
        let title = format!("{:?}", event.info);
        let body = game.to_string();
        ApnAlert { title, body, subtitle: None }
    }
}
pub struct NotificationService {
    apn_client: ApnClient,
}

impl User {
    fn should_send(&self, game: &ApiGame) -> bool {
        if self.apn_token.is_none() || self.muted_games.contains(&game.game_uuid) {
            false
        } else { 
            self.explicit_games.contains(&game.game_uuid) || 
            self.teams.contains(&game.home_team_code) || 
            self.teams.contains(&game.away_team_code) 
        }
    }
}

impl NotificationService {
    pub fn new() -> NotificationService {
        NotificationService { apn_client: ApnClient::new() }
    }

    pub async fn process(&mut self, game: &ApiGame, event: Option<&ApiGameEvent>) {
        for user in UserService::stream_all() {
            if let Some((device_token, push)) = NotificationService::get_apn_push(&user, game, event) {
                let push_type = push.header.push_type.clone();
                let apn_result = self.apn_client.push(&push, device_token).await;
                match (apn_result, push_type) {
                    (Err(_), ApnPushType::Notification) => { UserService::remove_apn_token(&user.id); },
                    (Err(_), ApnPushType::LiveActivity) => { UserService::remove_live_activity(&user.id, &game.game_uuid); },
                    (Ok(_), ApnPushType::LiveActivity) => {
                        if push.body.aps.event == Some("end".to_string()) {
                            UserService::remove_live_activity(&user.id, &game.game_uuid);
                        }
                    },
                    _ => {},
                }
            }
        }
    }

    pub async fn process_live_activity(&mut self, game: &ApiGame, event: Option<&ApiGameEvent>) {
        for user in UserService::stream_all() {
            if let Some((device_token, push)) = NotificationService::get_apn_push(&user, game, event) {
                let push_type = push.header.push_type.clone();
                if push_type == ApnPushType::LiveActivity {
                    let apn_result = self.apn_client.push(&push, device_token).await;
                    match (apn_result, push_type) {
                        (Err(_), ApnPushType::LiveActivity) => { UserService::remove_live_activity(&user.id, &game.game_uuid); },
                        (Ok(_), ApnPushType::LiveActivity) => {
                            if push.body.aps.event == Some("end".to_string()) {
                                UserService::remove_live_activity(&user.id, &game.game_uuid);
                            }
                        },
                        _ => {},
                    }
                }
            }
        }
    }

    fn get_apn_push(user: &User, game: &ApiGame, event: Option<&ApiGameEvent>) -> Option<(String, ApnPush<Option<LiveActivityContentState>, ApiGame>)> {
        let now = Utc::now().second();
        let expiration = (Utc::now() + Duration::hours(1)).second();

        
        let alert = match event.as_ref().map(|e| e.should_notify()).unwrap_or(false) {
            true => Some(ApnAlert::from(game, event.unwrap())),
            false => None,
        };
        
        let live_activity_entry = user.live_activities.iter().find(|e| e.game_uuid == game.game_uuid);
        if let Some(live_activity_entry) = live_activity_entry {
            let aps = ApnAps {
                alert: alert.clone(),
                mutable_content: None,
                content_available: None,
                sound: match alert.is_some() { true => Some("ping.aiff".to_string()), false => None, },
                badge: None,
                event: Some(match event.as_ref().map(|e| e.info.clone()) { Some(ApiEventType::GameEnd(_)) => "end".to_string(), _ => "update".to_string() }),
                relevance_score: Some(match alert.is_some() { true => 100, false => 75, }),
                stale_date: Some(expiration),
                timestamp: Some(now),
                content_state: Some(LiveActivityContentState { game: game.clone(), event: event.cloned() }),
            };
            let body = ApnBody {
                aps,
                data: game.clone(),
            };
            let header = ApnHeader {
                push_type: ApnPushType::LiveActivity,
                priority: match alert.is_some() { true => 100, false => 75 },
                topic: format!("com.palforsberg.shl-app-ios{}", ".push-type.liveactivity"),
                collapse_id: Some(game.game_uuid.clone()),
                expiration: Some(expiration),
            };
            Some((live_activity_entry.apn_token.clone(), ApnPush { header, body, }))
        } else if alert.is_some() && user.should_send(game) {

            let aps = ApnAps {
                alert,
                sound: Some("ping.aiff".to_string()),
                content_state: None,
                ..Default::default()
            };
            let body = ApnBody {
                aps,
                data: game.clone(),
            };
            let header = ApnHeader {
                push_type: ApnPushType::Notification,
                priority: 100,
                topic: "com.palforsberg.shl-app-ios".to_string(),
                collapse_id: Some(game.game_uuid.clone()),
                expiration: Some(expiration),
            };
            Some((user.apn_token.to_owned().unwrap(), ApnPush { header, body, }))
        } else {
            None
        }
    }
}