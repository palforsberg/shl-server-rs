#![allow(non_snake_case, clippy::upper_case_acronyms)]

use std::fmt::Display;
use std::sync::Arc;
use std::time::Duration;
use api::Api;
use api_player_stats_service::ApiPlayerStatsService;
use api_season_service::SafeApiSeasonService;
use api_ws::WsMsg;
use bounded_vec_deque::BoundedVecDeque;
use config_handler::Config;
use futures::future::join_all;
use msg_bus::{Msg, MsgBus};
use sse_client::SseMsg;
use standing_service::StandingService;
use tokio::select;
use tokio::sync::{mpsc, RwLock};
use tokio::sync::mpsc::{Sender, Receiver};

use models::Season;
use vote_service::VoteService;
use crate::api_season_service::ApiSeasonService;
use crate::fetch_details_service::FetchDetailsService;
use crate::models_api::event::ApiEventTypeLevel;
use crate::models_api::report::{ApiGameReport, GameStatus};
use crate::notification_service::NotificationService;
use crate::report_state_machine::ReportStateMachine;
use crate::event_service::EventService;
use crate::game_report_service::GameReportService;
use crate::player_service::PlayerService;
use crate::sse_client::SseClient;
use crate::season_service::SeasonService;
use crate::stats_service::StatsService;
use tracing::log;
use crate::user_service::UserService;
use lazy_static::lazy_static;

mod config_handler;
mod rest_client;
mod models;
mod season_service;
mod db;
mod api_season_service;
mod game_report_service;
mod event_service;
mod sse_client;
mod report_state_machine;
mod api_game_details;
mod stats_service;
mod player_service;
mod models_external;
mod models_legacy;
mod models_api;
mod vote_service;
mod standing_service;
mod api_teams_service;
mod api;
mod api_ws;
mod fetch_details_service;
mod user_service;
mod notification_service;
mod apn_client;
mod in_mem_games;
mod api_player_stats_service;
mod playoff_service;
mod msg_bus;

#[cfg(test)]
mod mock_test;

lazy_static! {
    pub static ref CONFIG: Config = config_handler::get_config();
}

#[tokio::main]
async fn main() {
    if std::env::var_os("RUST_LOG").is_none() {
        // Set the RUST_LOG, if it hasn't been explicitly defined
        std::env::set_var("RUST_LOG", "debug,hyper=debug" )
    }

    // Configure a custom event formatter
    let format = tracing_subscriber::fmt::format()
        .with_level(true)
        .with_target(false)
        .with_ansi(false)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_file(false)
        .compact();
    tracing_subscriber::fmt()
        .event_format(format)
        .with_max_level(tracing::Level::INFO)
        .init();

    let api_season_service = ApiSeasonService::new();
    let vote_service = VoteService::new();
    for season in Season::get_all() {
        let (responses, _) = SeasonService { }.update(&season).await;
        let api_games = api_season_service.write().await.update(&season, &responses);
        
        StandingService::update(&season, &api_games);
    }
    
    let all_games = ApiSeasonService::read_all();
    ApiPlayerStatsService::update(&all_games);

    let (live_game_sender, live_game_receiver) = mpsc::channel(1000);
    let (broadcast_sender, _) = tokio::sync::broadcast::channel(1000);
    let msg_bus = Arc::new(MsgBus::new());

    let h1 = {
        let api_season_service = api_season_service.clone();
        let broadcast_sender = broadcast_sender.clone();
        tokio::spawn(async { Api::serve(CONFIG.port, api_season_service, vote_service, broadcast_sender).await })
    };
    let h2 = {
        let api_season_service = api_season_service.clone();
        tokio::spawn(async { handle_loop(live_game_sender, api_season_service).await })
    };
    let h3 = {
        let api_season_service = api_season_service.clone();
        let broadcast_sender = broadcast_sender.clone();
        let msg_bus = msg_bus.clone();
        tokio::spawn(async { handle_sse(api_season_service, live_game_receiver, broadcast_sender, msg_bus).await })
    };
    let h4 = {
        let api_season_service = api_season_service.clone();
        let msg_bus = msg_bus.clone();
        tokio::spawn(async { handle_stats_fetch(msg_bus, api_season_service).await })
    };

    join_all(vec!(h1, h2, h3, h4)).await;

}

async fn handle_loop(live_game_sender: Sender<String>, api_season_service: SafeApiSeasonService) {
    let season_service = SeasonService { };
    let mut sent_live_games = BoundedVecDeque::new(40);

    loop {
        let season = Season::get_current();
        let (responses, updated) = season_service.update(&season).await;
        if updated {
            let api_games = api_season_service.write().await.update(&season, &responses);
            
            StandingService::update(&season, &api_games);
            ApiPlayerStatsService::update(&api_games);
        }

        let live_games: &Vec<String> = &responses.iter()
            .flat_map(|e| e.1.gameInfo.iter())
            .filter(|e| e.is_potentially_live())
            .map(|e| e.uuid.to_string())
            .collect();

        for game_uuid in live_games {
            if !sent_live_games.contains(game_uuid) {
                log::info!("[LOOP] Found live game {game_uuid}");
                live_game_sender.send(game_uuid.to_owned()).await
                    .ok_log("[SSE] Failed to send live game");
                sent_live_games.push_front(game_uuid.clone());
            }
        }

        FetchDetailsService::update().await;

        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
    }  
}

async fn handle_sse(
    api_season_service: SafeApiSeasonService,
    mut live_game_receiver: Receiver<String>, 
    broadcast_sender: tokio::sync::broadcast::Sender<WsMsg>,
    msg_bus: Arc<MsgBus>,
) {
    log::info!("[SSE] Start sse_listener");

    let notification_service = Arc::new(RwLock::new(NotificationService::new()));

    loop {
        if let Some(game_uuid) = live_game_receiver.recv().await {
            let (uuid, api_season_service, broadcast_sender, notification_service, msg_bus) = (game_uuid.clone(), api_season_service.clone(), broadcast_sender.clone(), notification_service.clone(), msg_bus.clone());
            tokio::spawn(async move {
                log::info!("[SSE] Start SSE {uuid}");
                let mut rsm = ReportStateMachine::new();
                let (handle, mut sse_msg_receiver) = SseClient::spawn_listener(&uuid).await;
                loop {
                    select! {
                        Some((game_uuid, msg)) = sse_msg_receiver.recv() => {
                            match msg {
                                SseMsg::Report(raw_report) => {
                                    let report: ApiGameReport = raw_report.into();
                                    let report_event = rsm.process(&report);
                                    GameReportService::store(&game_uuid, &report);
                                    log::info!("[SSE] REPORT {report}");

                                    let updated_api_game = api_season_service.write().await.update_from_report(&report);
                                
                                    if let Some(g) = updated_api_game {
                                        if let Some(report) = report_event {
                                            notification_service.write().await.process(&g, &report).await;
                                        }
                                        notification_service.write().await.process_live_activity(&g).await;
                                    } else {
                                        log::error!("[SSE] Notification error, no game found for {}", game_uuid);
                                    }

                                    _ = broadcast_sender.send(report.clone().into());
                                    msg_bus.send(Msg::Report { report, game_uuid });
                                },

                                SseMsg::Event(raw_event) => {
                                    let new_event = EventService::store_raw(&uuid, &raw_event);
                                    let event = raw_event.into_mapped_event(&uuid);
                                    log::info!("[SSE] EVENT {event}");
        
                                    if new_event && event.info.get_level() != ApiEventTypeLevel::Low {
                                        if let Some(game) = api_season_service.read().await.read_current_season_game(&game_uuid) {
                                            notification_service.write().await.process(&game, &event).await;
                                        } else {
                                            log::error!("[SSE] Notification error, no game found for {}", game_uuid);
                                        }
                                    }
                
                                    _ = broadcast_sender.send(event.clone().into());
                                    msg_bus.send(Msg::Event { event, new_event, game_uuid });
                                },
                            }
                        },
                        // if 5 minutes has passed without any new events and status is finished => abort
                        _ = tokio::time::sleep(Duration::from_secs(60 * 5)) => {
                            if let Some(GameStatus::Finished) = api_season_service.read().await.read_current_season_game(&uuid).map(|e| e.status) {
                                log::info!("[SSE] Game Finished, Abort {}", uuid);

                                UserService::remove_references_to(&uuid);
                                msg_bus.send(Msg::SseClosed { game_uuid });
                                break;
                            }
                        }
                    }
                }
                handle.abort();
                log::info!("[SSE] Aborted {}", uuid);
            });
        }
    }
}

async fn handle_stats_fetch(msg_bus: Arc<MsgBus>, api_season_service: SafeApiSeasonService) {
    let mut receiver = msg_bus.subscribe();
    loop {
        if let Ok(msg) = receiver.recv().await {
            if let Some(g) = api_season_service.read().await.read_current_season_game(msg.get_game_uuid()) {
                StatsService::update(&g.league, &g.game_uuid, Some(std::time::Duration::from_secs(30))).await;
                PlayerService::update(&g.league, &g.game_uuid, Some(std::time::Duration::from_secs(30))).await;
            }
        }
    }
}

pub trait LogResult<T, E: Display> {
    fn ok_log(self, msg: &str) -> Option<T>;
}

impl<T, E: Display> LogResult<T, E> for Result<T, E> {
    fn ok_log(self, msg: &str) -> Option<T> {
        match self {
            Ok(o) => Some(o),
            Err(e) => {
                log::error!("{}: {}", msg, e);
                None
            }
        }
    } 
}