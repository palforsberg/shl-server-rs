use std::{net::SocketAddr, sync::Arc, time::Duration};

use axum::{Router, extract::{Path, State, WebSocketUpgrade, ws::{WebSocket, Message}}, response::IntoResponse, Json};
use futures::{StreamExt, SinkExt};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use tokio::{select, sync::{broadcast::{Receiver, Sender, self}, RwLock}};
use tower::ServiceBuilder;
use tower_http::compression::CompressionLayer;
use tracing::log;

use crate::{SafeApiSeasonService, api_game_details::{ApiGameDetailsService, GameDetails}, api_season_service::ApiSeasonService, api_teams_service::ApiTeamsService, standing_service::StandingService, models::{League, Season}, vote_service::{VoteService, Vote, SafeVoteService}, event_service::ApiGameEvent, stats_service::ApiGameStats, game_report_service::ApiGameReport, api_ws::{ApiWs, WsMsg}};



#[derive(Clone)]
pub struct ApiState {
    pub game_details_service: ApiGameDetailsService,
    pub season_service: SafeApiSeasonService,
    pub vote_service: SafeVoteService,
    pub broadcast_sender: Sender<WsMsg>,

    pub nr_ws: Arc<RwLock<i16>>,
}
pub struct Api;
impl Api {
    pub async fn serve(port: u16, season_service: SafeApiSeasonService, vote_service: SafeVoteService, broadcast_sender: Sender<WsMsg>) {
        let state = ApiState {
            game_details_service: ApiGameDetailsService::new(season_service.clone()),
            season_service,
            vote_service,
            broadcast_sender,
            nr_ws: Arc::new(RwLock::new(0)),
        };
        let app = Router::new()
            .route("/games/:season", axum::routing::get(Api::get_games))
            .route("/game/:game_uuid/:game_id", axum::routing::get(Api::get_game_details))
            .route("/teams/:season", axum::routing::get(Api::get_teams))
            .route("/standings/:season", axum::routing::get(Api::get_shl_standings))
            .route("/playoffs/:season", axum::routing::get(Api::get_playoffs))
            .route("/players/:team", axum::routing::get(Api::get_players))
    
            .route("/live-activity/start", axum::routing::post(Api::start_live_activity))
            .route("/live-activity/end", axum::routing::post(Api::end_live_activity))

            .route("/vote", axum::routing::post(Api::vote))

            .route("/ws", axum::routing::get(Api::ws_handler))
    
            .route("/", axum::routing::get(Api::root))
            .with_state(state)
            .layer(ServiceBuilder::new()
                .layer(CompressionLayer::new()) // adds 50ms
            );
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        log::info!("[API] Listening on {}", addr);
        axum::Server::bind(&addr)
            .serve(app.into_make_service())
            .await;
    }
    
    
    async fn root() -> &'static str {
        "Puck puck puck"
    }
    
    async fn get_games(Path(season): Path<String>) -> impl IntoResponse {
        if let Ok(season) = season.parse() {
            (StatusCode::OK, ApiSeasonService::read_raw(&season))
        } else {
            (StatusCode::NOT_FOUND, "404".to_string())
        }
    }
    
    async fn get_game_details(
        Path((game_uuid, game_id)): Path<(String, String)>, 
        State(state): State<ApiState>) -> Json<Option<GameDetails>> {
            
        Json(state.game_details_service.read(&game_uuid).await)
    }
    
    async fn get_teams() -> impl IntoResponse {
        ApiTeamsService::read_raw()
    }
    
    async fn get_shl_standings(season: Option<Path<(String)>>) -> impl IntoResponse {
        if let Ok(season) = season.map(|e| e.parse()).unwrap_or_else(|| Ok(Season::get_current())) {
            let shl = StandingService::read_raw(League::SHL, season.clone());
            let ha = StandingService::read_raw(League::HA, season);
            (StatusCode::OK, format!("{{\"SHL\":{shl}, \"HA\":{ha}}}"))
        } else {
            (StatusCode::NOT_FOUND, "404".to_string())
        }
    }
    
    async fn get_players() -> impl IntoResponse {
        todo!()
    }

    async fn get_playoffs() -> impl IntoResponse {
        todo!()
    }
    
    async fn start_live_activity() -> impl IntoResponse {
        todo!()
    }
    
    
    async fn end_live_activity() -> impl IntoResponse {
        todo!()
    }

    async fn vote(State(state): State<ApiState>, Json(vote): Json<VoteBody>) -> impl IntoResponse {
        if let Some(game) = state.season_service.read().await.read_current_season_game(&vote.game_uuid) {
            if game.home_team_code != vote.team_code && game.away_team_code != vote.team_code {
                (StatusCode::BAD_REQUEST, "Invalid team_code".to_string())
            } else {
                let is_home_winner = game.home_team_code == vote.team_code;
                let vote = Vote { user_id: vote.user_id, game_uuid: vote.game_uuid, team_code: vote.team_code, is_home_winner };
                let mut vs = state.vote_service.write().await;
                (StatusCode::OK, serde_json::to_string(&vs.vote(vote)).ok().unwrap_or_default())
            }
        } else {
            (StatusCode::NOT_FOUND, "404".to_string())
        }
    }  

    async fn ws_handler(
        ws: WebSocketUpgrade,
        State(state): State<ApiState>) -> impl IntoResponse {
        ws.on_upgrade(|socket| ApiWs::handle(socket, state))
    } 
}


#[derive(Deserialize)]
struct VoteBody {
    game_uuid: String,
    user_id: String,
    team_code: String,
}