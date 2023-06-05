use std::{time::Instant};

use chrono::{Utc, Duration};
use futures::{future::join_all, FutureExt};
use tracing::log;

use crate::{event_service::{ApiGameEvent, ApiEventType, ApiEventTypeLevel}, api_season_service::ApiGame, user_service::{UserService, User}, apn_client::{ApnClient, ApnPush, ApnAlert, ApnBody, ApnHeader, ApnAps, LiveActivityContentState, ApnPushType, ApnError}, CONFIG, api_teams_service::{TeamsMap}, game_report_service::GameStatus};

impl ApiGameEvent {
    fn get_time_info(&self) -> String {
        match self.status {
            GameStatus::Period1 => format!("P1 {}", self.gametime),
            GameStatus::Period2 => format!("P2 {}", self.gametime),
            GameStatus::Period3 => format!("P3 {}", self.gametime),
            GameStatus::Overtime => format!("Övertid {}", self.gametime),
            GameStatus::Shootout => "Straffar".to_string(),
            _ => "".to_string(),
        }
    }

    fn get_images(&self, game: &ApiGame) -> Vec<String> {
        match &self.info {
            ApiEventType::GameStart => vec!(game.home_team_code.clone(), game.away_team_code.clone()),
            ApiEventType::GameEnd(a) => {
                match &a.winner {
                    Some(winner) => vec!(winner.clone()),
                    None => vec!(game.home_team_code.clone(), game.away_team_code.clone()),
                }
            }
            ApiEventType::Goal(a) => vec!(a.team.clone()),
            ApiEventType::Penalty(a) => vec!(a.team.clone()),
            _ => vec!(),
        }
    }
}
impl ApnAlert {
    fn from(game: &ApiGame, event: &ApiGameEvent, teams: &TeamsMap, user_teams: &[String]) -> ApnAlert {

        let (title, body) = match &event.info {

            ApiEventType::GameStart => {
                let home = teams.get_shortname(&game.home_team_code);
                let away = teams.get_shortname(&game.away_team_code);
                ("Nedsläpp".to_string(), format!("{home} : {away}"))
            }

            ApiEventType::GameEnd(a) => {
                let excited = a.winner.as_ref().map(|e| user_teams.contains(e)).unwrap_or(false);
                let (home, away) = (teams.get_shortname(&game.home_team_code), teams.get_shortname(&game.away_team_code));
                let title = match (&a.winner, excited) {
                    (Some(winner), true) => format!("{winner} vinner! 🥇"),
                    (Some(winner), false) => format!("{winner} vann"),
                    (None, _) => "Matchen slutade".to_string(),
                };
                let body = format!("{} {} - {} {}", home, game.home_team_result, game.away_team_result, away);
                (title, body)
            },

            ApiEventType::Goal(a) => {
                let excited = user_teams.contains(&a.team);
                let team_name = teams.get_shortname(&a.team);
                let title = match excited {
                    true => format!("MÅÅÅL för {}! 🎉", team_name),
                    false => format!("Mål för {}", team_name),
                };

                let player = a.player.as_ref().map(|p| format!("{}. {}", p.first_name.chars().next().unwrap(), p.family_name)).unwrap_or_default();
                let home_code = teams.get_display_code(&game.home_team_code);
                let away_code = teams.get_display_code(&game.away_team_code);
                let score_board = format!("{} {} - {} {}", home_code, a.home_team_result, a.away_team_result, away_code);
                let bottom = format!("{player} • {}", event.get_time_info());
                let body = format!("{score_board}\n{bottom}");
                (title, body)
            },

            _ => {
                let title = format!("{:?}", event.info);
                let home_code = teams.get_display_code(&game.home_team_code);
                let away_code = teams.get_display_code(&game.away_team_code);
                let score_board = format!("{} {} - {} {}", home_code, game.home_team_result, game.away_team_result, away_code);
                (title, score_board)
            }
        };
        ApnAlert { title, body, subtitle: None }
    }
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

pub struct NotificationService {
    apn_client: ApnClient,
    teams: TeamsMap,
}

impl NotificationService {
    pub fn new() -> NotificationService {
        NotificationService { 
            apn_client: ApnClient::new(), 
            teams: TeamsMap::new(),
        }
    }

    pub async fn process(&mut self, game: &ApiGame, event: Option<&ApiGameEvent>) {
        let before = Instant::now();
        self.apn_client.update_token();
        let mut futures = vec!();
        for user in UserService::stream_all() {
            if let Some((device_token, push)) = self.get_apn_push(&user, game, event) {
                let push_type = push.header.push_type.clone();
                let future = self.apn_client.push_notification(push, device_token).map(move |e| {
                    if let Err(ApnError::BadDeviceToken) = e {
                        match push_type {
                            ApnPushType::LiveActivity => UserService::end_live_activity(&user.id, &game.game_uuid),
                            ApnPushType::Alert => UserService::remove_apn_token(&user.id),
                        }
                    }
                });
                futures.push(future);
            } 
        }
        let size = futures.len();
        join_all(futures).await;
        if size > 0 {
            log::info!("[NOTIFICATION] Sent notifications in {:.0?}", before.elapsed());
        }
    }

    pub async fn process_live_activity(&mut self, game: &ApiGame, event: Option<&ApiGameEvent>) {
        let before = Instant::now();
        self.apn_client.update_token();
        let mut futures = vec!();
        for user in UserService::stream_all() {
            if let Some((device_token, push)) = self.get_apn_push(&user, game, event) {
                let push_type = push.header.push_type.clone();
                if push_type == ApnPushType::LiveActivity {
                    let future = self.apn_client.push_notification(push, device_token).map(move |e| {
                        if let Err(ApnError::BadDeviceToken) = e {
                            match push_type {
                                ApnPushType::LiveActivity => UserService::end_live_activity(&user.id, &game.game_uuid),
                                ApnPushType::Alert => UserService::remove_apn_token(&user.id),
                            }
                        }
                    });
                    futures.push(future);
                }
            } 
        }
        let size = futures.len();
        join_all(futures).await;
        if size > 0 {
            log::info!("[NOTIFICATION] Sent notifications in {:.0?}", before.elapsed());
        }
        // TODO: Handle end live activity
    }

    fn get_apn_push(&self, user: &User, game: &ApiGame, event: Option<&ApiGameEvent>) -> Option<(String, ApnPush<Option<LiveActivityContentState>, ApiGame>)> {
        let now = Utc::now().timestamp();
        let expiration = (Utc::now() + Duration::hours(1)).timestamp();
        let alert = match event.as_ref().map(|e| e.info.get_level() == ApiEventTypeLevel::High).unwrap_or(false) {
            true => Some(ApnAlert::from(game, event.unwrap(), &self.teams, &user.teams)),
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
                local_attachements: event.map(|e| e.get_images(game)).unwrap_or_default(),
            };
            let header = ApnHeader {
                push_type: ApnPushType::LiveActivity,
                priority: match alert.is_some() { true => 100, false => 75 },
                topic: format!("{}.push-type.liveactivity", CONFIG.apn_topic),
                collapse_id: Some(game.game_uuid.clone()),
                expiration: Some(expiration),
            };
            Some((live_activity_entry.apn_token.clone(), ApnPush { header, body, }))
        } else if alert.is_some() && user.should_send(game) {
            let body = ApnBody {
                aps: ApnAps {
                    alert,
                    sound: Some("ping.aiff".to_string()),
                    content_state: None,
                    ..Default::default()
                },
                data: game.clone(),
                local_attachements: event.map(|e| e.get_images(game)).unwrap_or_default(),
            };
            let header = ApnHeader {
                push_type: ApnPushType::Alert,
                priority: 100,
                topic: CONFIG.apn_topic.to_string(),
                collapse_id: Some(game.game_uuid.clone()),
                expiration: Some(expiration),
            };
            Some((user.apn_token.to_owned().unwrap(), ApnPush { header, body, }))
        } else {
            None
        }
    }
}