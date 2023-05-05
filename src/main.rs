#![allow(non_snake_case, clippy::upper_case_acronyms)]

use std::fmt::Display;
use std::time::Duration;
use api::{Api};
use api_season_service::{SafeApiSeasonService};
use api_teams_service::ApiTeamsService;
use api_ws::WsMsg;
use bounded_vec_deque::BoundedVecDeque;
use config_handler::Config;
use futures::future::join_all;
use standing_service::StandingService;
use tokio::select;
use tokio::sync::{mpsc};
use tokio::sync::mpsc::{Sender, Receiver};

use models::{Season};
use vote_service::VoteService;
use crate::api_season_service::ApiSeasonService;
use crate::fetch_details_service::FetchDetailsService;
use crate::report_state_machine::{ReportStateMachine, ApiSseMsg};
use crate::event_service::{EventService, ApiEventType};
use crate::game_report_service::{GameReportService, ApiGameReport, GameStatus};
use crate::models2::external::season::SeasonTeam;
use crate::player_service::PlayerService;
use crate::sse_client::{SseClient};
use crate::season_service::SeasonService;
use crate::stats_service::StatsService;
use crate::user_service::UserService;
use tracing::{log};
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
mod models2;
mod vote_service;
mod standing_service;
mod api_teams_service;
mod api;
mod api_ws;
mod fetch_details_service;
mod user_service;

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
        ApiTeamsService::add(&responses.into_iter().flat_map(|e| e.1.teamList.into_iter()).collect::<Vec<SeasonTeam>>());
        StandingService::update(&season, &api_games);
    }

    let (live_game_sender, live_game_receiver) = mpsc::channel(1000);
    let (sse_msg_sender, sse_msg_receiver) = mpsc::channel(1000);
    let (broadcast_sender, _) = tokio::sync::broadcast::channel(1000);

    let loop_api_season_service = api_season_service.clone();
    let event_api_season_service = api_season_service.clone();
    let sse_api_season_service = api_season_service.clone();
    let sse_broadcast_sender = broadcast_sender.clone();
    let h1 = tokio::spawn(async move { Api::serve(CONFIG.port, api_season_service, vote_service, broadcast_sender).await });
    let h2 = tokio::spawn(async { start_loop(live_game_sender, loop_api_season_service).await });
    let h3 = tokio::spawn(async { game_start_end_listener(sse_api_season_service, live_game_receiver, sse_msg_sender).await });
    let h4 = tokio::spawn(async { handle_sse_events(event_api_season_service, sse_msg_receiver, sse_broadcast_sender).await });

    join_all(vec!(h1, h2, h3, h4)).await;

}

async fn start_loop(
    live_game_sender: Sender<String>, 
    api_season_service: SafeApiSeasonService,
) {
    let season_service = SeasonService { };
    let mut sent_live_games = BoundedVecDeque::new(40);

    loop {
        log::info!("[LOOP] loop");
        let season = Season::get_current();
        let (responses, updated) = season_service.update(&season).await;
        if updated {
            let api_games = api_season_service.write().await.update(&season, &responses);
            ApiTeamsService::add(&responses.iter().flat_map(|e| e.1.teamList.clone().into_iter()).collect::<Vec<SeasonTeam>>());
            StandingService::update(&season, &api_games);
        }

        FetchDetailsService::update().await;

        let live_games: &Vec<String> = &responses.iter()
            .flat_map(|e| e.1.gameInfo.iter())
            .filter(|e| e.is_potentially_live())
            .map(|e| e.uuid.to_string())
            .filter(|e| !sent_live_games.contains(e))
            .collect();

        if !live_games.is_empty() {
            log::info!("[LOOP] Found {} live games", &live_games.len());
            for game_uuid in live_games {
                live_game_sender.send(game_uuid.to_owned()).await
                    .ok_log("[SSE] Failed to send live game");
                sent_live_games.push_front(game_uuid.clone());
            }
        }
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
    }  
}

async fn game_start_end_listener(
    api_season_service: SafeApiSeasonService,
    mut live_game_receiver: Receiver<String>, 
    sse_msg_sender: Sender<(String, ApiSseMsg)>,
) {
    log::info!("[SSE] Start sse_listener");
    loop {
        if let Some(game_uuid) = live_game_receiver.recv().await {
            let (uuid, sse_sender, ass) = (game_uuid.clone(), sse_msg_sender.clone(), api_season_service.clone());
            tokio::spawn(async move {
                log::info!("[SSE] Start SSE {uuid}");
                let (handle, mut report_receiver, mut event_receiver) = SseClient::spawn_listener(&uuid).await;
                let mut rsm = ReportStateMachine::new();
                loop {
                    select! {
                        Some((game_uuid, report)) = report_receiver.recv() => {
                            let mapped: ApiGameReport = report.into();
                            if let Some(report_event) = rsm.process(&mapped) {
                                sse_sender.send((game_uuid.clone(), ApiSseMsg::Event(report_event))).await
                                    .ok_log("[SSE] Failed to send event");
                            }
                            sse_sender.send((uuid.clone(), ApiSseMsg::Report(mapped))).await
                                .ok_log("[SSE] Failed to send report");
                        },
                        Some((game_uuid, event)) = event_receiver.recv() => {
                            EventService::store_raw(&uuid, &event);
                            let mapped = event.into_mapped_event(&uuid);
                            sse_sender.send((game_uuid.clone(), ApiSseMsg::Event(mapped))).await
                                .ok_log("[SSE] Failed to send event");
                        }
                        // if 10 minutes has passed without any new events and status is finished => abort
                        _ = tokio::time::sleep(Duration::from_secs(60 * 10)) => {
                            if let Some(GameStatus::Finished) = ass.read().await.read_current_season_game(&uuid).map(|e| e.status) {
                                log::info!("[SSE] Abort {}", game_uuid);
                                break;
                            }
                        }
                    }
                }
                handle.abort();
                log::info!("[SSE] Aborted {}", game_uuid);
            });
        }
    }
}

async fn handle_sse_events(
    api_season_service: SafeApiSeasonService,
    mut sse_msg_receiver: Receiver<(String, ApiSseMsg)>, 
    broadcast_sender: tokio::sync::broadcast::Sender<WsMsg>,
) {

    log::info!("[SSE] Start sse handler");
    loop {
        if let Some((game_uuid, msg)) = sse_msg_receiver.recv().await {
            match msg {
                ApiSseMsg::Report(report) => {
                    log::info!("[SSE] REPORT {report}");
                    GameReportService::store(&game_uuid, &report);
                    
                    broadcast_sender.send(report.clone().into())
                        .ok_log("[SSE] Failed to broadcast report");

                    let updated_api_game = api_season_service.write().await.update_from_report(&report);
                    if let Some(g) = updated_api_game {
                        StatsService::update(&g.league, &game_uuid, Some(std::time::Duration::from_secs(30))).await;
                        PlayerService::update(&g.league, &game_uuid, Some(std::time::Duration::from_secs(30))).await;
                    }
                },
                ApiSseMsg::Event(event) => {
                    log::info!("[SSE] EVENT {event}");
                    let new_event = EventService::store(&game_uuid, &event);
                    if new_event && event.should_publish() {
                        if let Some(g) = api_season_service.read().await.read_current_season_game(&game_uuid) {
                            let _ = UserService::get_users_for(&g.home_team_code, &g.away_team_code);
                            log::info!("[SSE] Send notification {:?} {:?}", event, g);
                        }
                    }

                    broadcast_sender.send(event.clone().into())
                        .ok_log("[SSE] Failed to broadcast event");

                    if let Some(g) = api_season_service.read().await.read_current_season_game(&game_uuid) {
                        StatsService::update(&g.league, &game_uuid, Some(std::time::Duration::from_secs(30))).await;
                        PlayerService::update(&g.league, &game_uuid, Some(std::time::Duration::from_secs(30))).await;
                    }
                    if matches!(event.info, ApiEventType::GameEnd(_)) {
                        let season_service = api_season_service.clone();
                        tokio::spawn(async move {
                            log::info!("[SSE] Game Ended, Updating in 5min");
                            tokio::time::sleep(Duration::from_secs(60 * 5)).await;
                            if let Some(g) = season_service.read().await.read_current_season_game(&game_uuid) {
                                StatsService::update(&g.league, &game_uuid, Some(std::time::Duration::from_secs(30))).await;
                                PlayerService::update(&g.league, &game_uuid, Some(std::time::Duration::from_secs(30))).await;
                                log::info!("[SSE] Updated");
                            }
                        });
                    }
                },
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