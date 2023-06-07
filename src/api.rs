use std::{net::SocketAddr, sync::Arc};

use axum::{Router, extract::{Path, State, WebSocketUpgrade}, response::IntoResponse, Json, routing::{get, post}};
use reqwest::StatusCode;
use serde::{Deserialize};
use tokio::{sync::{RwLock, broadcast::Sender}};
use tower::ServiceBuilder;
use tower_http::compression::CompressionLayer;
use tracing::log;

use crate::{SafeApiSeasonService, api_game_details::{ApiGameDetailsService, ApiGameDetails}, api_season_service::ApiSeasonService, api_teams_service::ApiTeamsService, standing_service::StandingService, models::{League, Season}, vote_service::{Vote, SafeVoteService}, api_ws::{ApiWs, WsMsg}, user_service::{UserService}, models2::legacy::{game_details::LegacyGameDetails, player_stats::LegacyPlayerStats}, api_player_stats_service::{ApiPlayerStatsService, TeamSeasonKey}, playoff_service::PlayoffService};

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
            .route("/games/:season", get(Api::get_games))
            .route("/game/:game_uuid/:game_id", get(Api::get_legacy_game_details))
            .route("/game/:game_uuid", get(Api::get_game_details))
            .route("/teams", get(Api::get_teams))
            .route("/rankings/:season", get(Api::get_leagues))
            .route("/standings/:season", get(Api::get_legacy_standings))
            .route("/playoffs/:season", get(Api::get_playoffs))
            .route("/player/:player_id", get(Api::get_player))
            .route("/players/:team/:season", get(Api::get_players))
            .route("/players/:team", get(Api::get_legacy_players))
    
            .route("/live-activity/start", post(Api::start_live_activity))
            .route("/live-activity/end", post(Api::end_live_activity))

            .route("/vote", post(Api::vote))

            .route("/user", post(Api::add_user))

            .route("/ws", get(Api::ws_handler))
    
            .route("/", get(Api::root))
            .with_state(state)
            .layer(ServiceBuilder::new()
                .layer(CompressionLayer::new()) // adds 50ms
            );
        let addr = SocketAddr::from(([0, 0, 0, 0], port));
        log::info!("[API] Listening on {}", addr);
        _ = axum::Server::bind(&addr)
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
    
    async fn get_game_details(Path(game_uuid): Path<String>, State(state): State<ApiState>) -> Json<Option<ApiGameDetails>> {
        Json(state.game_details_service.read(&game_uuid).await)
    }

    
    async fn get_teams() -> impl IntoResponse {
        ApiTeamsService::read_raw()
    }
    
    async fn get_leagues(season: Option<Path<String>>) -> impl IntoResponse {
        if let Ok(season) = season.map(|e| e.parse()).unwrap_or_else(|| Ok(Season::get_current())) {
            let shl = StandingService::read_raw(League::SHL, season.clone());
            let ha = StandingService::read_raw(League::HA, season);
            (StatusCode::OK, format!("{{\"SHL\":{shl}, \"HA\":{ha}}}"))
        } else {
            (StatusCode::NOT_FOUND, "404".to_string())
        }
    }

    async fn get_legacy_game_details(
        Path((game_uuid, _)): Path<(String, String)>, 
        State(state): State<ApiState>) -> Json<Option<LegacyGameDetails>> {
            
        Json(state.game_details_service.read(&game_uuid).await.map(|e| e.into()))
    }

    async fn get_legacy_standings(season: Option<Path<String>>) -> impl IntoResponse {
        if let Ok(season) = season.map(|e| e.parse()).unwrap_or_else(|| Ok(Season::get_current())) {
            (StatusCode::OK, StandingService::read_raw(League::SHL, season))
        } else {
            (StatusCode::NOT_FOUND, "404".to_string())
        }
    }
    
    async fn get_legacy_players(Path(team): Path<String>) -> impl IntoResponse {
        let db = ApiPlayerStatsService::get_team_player_db();
        let data: Vec<LegacyPlayerStats> = db.read(&TeamSeasonKey(Season::Season2022, team))
            .unwrap_or_default()
            .into_iter()
            .map(|e| e.into())
            .collect();
        Json(data)
    }

    async fn get_players(Path((season, team)): Path<(String, String)>) -> impl IntoResponse {
        if let Ok(e) = season.parse() {
            let db = ApiPlayerStatsService::get_team_player_db();
            (StatusCode::OK, db.read_raw(&TeamSeasonKey(e, team)))
        } else {
            (StatusCode::NOT_FOUND, "404".to_string())
        }
    } 

    async fn get_player(Path(player_id): Path<i32>) -> impl IntoResponse {
        let db = ApiPlayerStatsService::get_player_career_db();
        db.read_raw(&player_id)
    } 

    async fn get_playoffs(Path(season): Path<String>) -> impl IntoResponse {
        if let Ok(e) = season.parse() {
            (StatusCode::OK, PlayoffService::get_db().read_raw(&e))
        } else {
            (StatusCode::NOT_FOUND, "404".to_string())
        }          
    }
    
    async fn start_live_activity(Json(req): Json<StartLiveActivity>) -> impl IntoResponse {
        UserService::start_live_activity(&req);
    }

    async fn end_live_activity(Json(req): Json<EndLiveActivity>) -> impl IntoResponse {
        UserService::end_live_activity(&req.user_id, &req.game_uuid);
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

    async fn add_user(Json(user): Json<AddUser>) -> impl IntoResponse {
        UserService::handle(user);
        (StatusCode::OK, "success".to_string())
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

#[derive(Deserialize)]
pub struct AddUser {
    pub id: String,
    pub teams: Vec<String>,
    pub apn_token: Option<String>,
    pub ios_version: Option<String>,
    pub app_version: Option<String>,
}

#[derive(Deserialize)]
pub struct StartLiveActivity {
    pub user_id: String,
    pub token: String,
    pub game_uuid: String,
}


#[derive(Deserialize)]
pub struct EndLiveActivity {
    pub user_id: String,
    pub game_uuid: String,
}