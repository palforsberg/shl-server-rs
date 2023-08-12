use std::{sync::Arc, time::Duration, collections::HashMap, net::SocketAddr, convert::Infallible};

use async_stream::try_stream;
use axum::{Router, extract::{Path, State, Query}, response::{IntoResponse, Sse, sse::{KeepAlive, Event}}, Json, body::StreamBody, routing::{get, post}};
use reqwest::StatusCode;
use serde::Deserialize;
use shl_server_rs::{models_external::{season::{SeasonGame, SeasonRsp}, event::SseEvent}, models::{Season, GameType, League}};
use tokio::{sync::{RwLock, broadcast::Sender}, task::JoinHandle};
use tokio_util::io::ReaderStream;

use super::models_apn::ApnBody;

pub struct ExternalServer {
    port: u16,
    handles: Vec<JoinHandle<()>>,

    sse_events: Arc<RwLock<Vec<SseEvent>>>,
    pub api_state: Arc<RwLock<AppState>>,
}   

impl Drop for ExternalServer {
    fn drop(&mut self) {
        for e in &self.handles {
            e.abort();
        }
    }
}

#[derive(Eq, PartialEq, Hash, Clone)]
pub struct GameKey(Season, League, GameType);

#[derive(Clone)]
pub struct AppState {
    pub sender: Sender<SseEvent>,
    pub notifications: Vec<(String, ApnBody)>,
    pub live_acitivies: HashMap<String, u16>,
    pub stat_calls: HashMap<String, u16>,
    pub added_games: HashMap<GameKey, Vec<SeasonGame>>,
}


#[derive(Deserialize)]
struct SportsQuery {
    seasonUuid: String,
    seriesUuid: String,
    gameTypeUuid: String,
}

impl ExternalServer {
    pub fn new(port: u16) -> ExternalServer {

        let sse_events = Arc::new(RwLock::new(vec![]));
        let (sse_handle, sse_sender) = ExternalServer::start_sse_listener(sse_events.clone(), Duration::from_micros(10));

        let api_state = Arc::new(RwLock::new(AppState { 
            sender: sse_sender, 
            notifications: vec![], 
            live_acitivies: HashMap::new(),
            stat_calls: HashMap::new(),
            added_games: HashMap::new()
        }));

        ExternalServer {
            port,
            handles: vec![sse_handle],
            sse_events,
            api_state,
        }
    }

    pub async fn start(&mut self) {
        let external_mock = {
            let port = self.port;
            let state = self.api_state.clone();
            tokio::spawn(async move { ExternalServer::serve_external_data(state, port).await })
        };

        self.handles.push(external_mock);

        tokio::time::sleep(Duration::from_secs(2)).await; // wait for mock to start
    }

    pub async fn push_events(&mut self, events: Vec<SseEvent>) {
        self.sse_events.write().await.extend(events);
    }

    pub async fn add_game(&mut self, season: Season, game_type: GameType, game: SeasonGame) {
        let mut safe_state = self.api_state.write().await;
        let added_games = safe_state.added_games
            .entry(GameKey(season, game.seriesInfo.code.clone(), game_type))
            .or_insert_with(Vec::new);

        added_games.push(game);
    }

    pub fn get_url(&self) -> String {
        format!("http://localhost:{}", self.port)
    }

    async fn serve_external_data(state: Arc<RwLock<AppState>>, port: u16) {
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        let app = Router::new()
            .route("/gameday/boxscore/:game_uuid", get(ExternalServer::get_boxscore_file))
            .route("/gameday/periodstats/:game_uuid", get(ExternalServer::get_periodstats_file))
            .route("/sports/game-info", get(ExternalServer::get_sports_file))
            .route("/gameday/live/game/SHL/:game_uuid", get(ExternalServer::get_sse))
            .route("/apn/push/3/device/:device_token", post(ExternalServer::post_apn))
            .with_state(state);
    
        axum::Server::bind(&addr)
            .serve(app.into_make_service())
            .await
            .unwrap();
    }

    async fn get_boxscore_file(Path(game_uuid): Path<String>) -> impl IntoResponse {
        ExternalServer::get_file_from(format!("./tests/integration/external/gameday/boxscore/{}", game_uuid)).await
    }
    
    async fn get_periodstats_file(Path(game_uuid): Path<String>, State(state): State<Arc<RwLock<AppState>>>) -> impl IntoResponse {
        let mut safe_state = state.write().await;
        let val = safe_state.stat_calls.entry(game_uuid.clone()).or_insert_with(|| 0);
        *val += 1;
        ExternalServer::get_file_from(format!("./tests/integration/external/gameday/periodstats/{}", game_uuid)).await
    }
    
    async fn post_apn(Path(device_token): Path<String>, State(state): State<Arc<RwLock<AppState>>>, Json(payload): Json<serde_json::Value>) -> impl IntoResponse {
        let apn_body = serde_json::from_value::<ApnBody>(payload).expect("APN should decode");
        
        if apn_body.aps.content_state.is_some() {
            let mut safe_state = state.write().await;
            let val = safe_state.live_acitivies.entry(device_token).or_insert_with(|| 0);
            *val += 1;
        } else {
            state.write().await.notifications.push((device_token, apn_body));
        }
    }

    async fn get_sports_file(query: Query<SportsQuery>, State(state): State<Arc<RwLock<AppState>>>) -> impl IntoResponse {
        let path = format!("./tests/integration/external/sports/game-info?gamePlace=all&played=all&seasonUuid={}&seriesUuid={}&gameTypeUuid={}", query.seasonUuid, query.seriesUuid, query.gameTypeUuid);

        let league = ExternalServer::parse_series_uuid(&query.seriesUuid).expect("[TEST] invalid league id");
        let game_type = ExternalServer::parse_game_type_uuid(&query.gameTypeUuid).expect("[TEST] invalid game type id");
        let season = ExternalServer::parse_season_uuid(&query.seasonUuid).expect("[TEST] invalid season id");
        let mut data = std::fs::read_to_string(path).ok().and_then(|e| serde_json::from_str::<SeasonRsp>(&e).ok()).unwrap();
        let safe_state = state.read().await;
        if let Some(games_to_add) = safe_state.added_games.get(&GameKey(season, league, game_type)) {
            data.gameInfo.extend(games_to_add.clone());
        }
        Json(data)
    }
    
    async fn get_sse(Path(game_uuid): Path<String>, State(state): State<Arc<RwLock<AppState>>>) -> Sse<impl futures::Stream<Item = Result<Event, Infallible>>> {
        let mut receiver = state.write().await.sender.subscribe();
        println!("[TEST] New SSE subscriber for {}", game_uuid);
        Sse::new(try_stream! {
            loop {
                match receiver.recv().await {
                    Ok(msg) => {
                        let msg_game_uuid = match (msg.gameReport.as_ref(), msg.playByPlay.as_ref()) {
                            (Some(e), _) => e.gameUuid.clone(),
                            (_, Some(e)) => e.gameUuid.clone(),
                            _ => panic!(),
                        };
                        if msg_game_uuid == game_uuid { // is the game_uuid of the message is the same as the game_uuid in the URL
                            let json_str = serde_json::to_string(&msg).expect("should encode to json");
                            yield Event::default().data(json_str);
                        }
                    },
                    Err(e) => { tracing::error!(error = ?e, "Failed to get"); }
                }
            }
        })
        .keep_alive(KeepAlive::default())
    }

    fn start_sse_listener(sse_events: Arc<RwLock<Vec<SseEvent>>>, sleep_time: Duration) -> (JoinHandle<()>, Sender<SseEvent>) {
        let (sender, _) = tokio::sync::broadcast::channel(10);
        println!("[TEST] Start SSE events");
        let handle = {
            let sender = sender.clone();
            let events = sse_events.clone();
            tokio::spawn(async move {
                loop {
                    if sender.receiver_count() > 0 {
                        println!("[TEST] SSE subscribers now {}", sender.receiver_count());
                        while let Some(entry) = remove_first(events.clone()).await {
                            sender.send(entry).expect("should broadcast event");
                            tokio::time::sleep(sleep_time).await;
                            
                            if sender.receiver_count() == 0 {
                                println!("[LOG] break {}", sender.receiver_count());
                                break;
                            }
                        }
                    }
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            })
        };
        (handle, sender)
    }
    
    async fn get_file_from(path: String) -> impl IntoResponse {
        let file = match tokio::fs::File::open(path).await {
            Ok(file) => file,
            Err(err) => return Err((StatusCode::NOT_FOUND, format!("File not found: {}", err))),
        };
        let stream = ReaderStream::new(file);
        let body = StreamBody::new(stream);
        Ok(body)
    }


    fn parse_series_uuid(s: &str) -> Result<League, ()> {
        match s {
            "qQ9-bb0bzEWUk" => Ok(League::SHL),
            "qQ9-594cW8OWD" => Ok(League::HA),
            _ => Err(()),
        }
    }

    fn parse_game_type_uuid(str: &str) -> Result<GameType, ()> {
        match str {
            "qQ9-af37Ti40B" => Ok(GameType::Season),
            "qQ9-7debq38kX" => Ok(GameType::PlayOff),
            "qRf-347BaDIOc" => Ok(GameType::Demotion),
            _ => Err(()),
        }
    }


    fn parse_season_uuid(str: &str) -> Result<Season, ()> {
        match str {
            "qcz-3NvSZ2Cmh" => Ok(Season::Season2023),
            "qbN-XMFfjGVt" => Ok(Season::Season2022),
            "qZl-8qa6OaFXf" => Ok(Season::Season2021),
            "qY7-AdVh5z1XJ" => Ok(Season::Season2020),
            "qWX-334j11U5o1" => Ok(Season::Season2019),
            "qUv-YXiuQN45" => Ok(Season::Season2018),
            _ => Err(()),
        }
    } 
}

async fn remove_first<T>(vec: Arc<RwLock<Vec<T>>>) -> Option<T> {
    if vec.read().await.is_empty() {
        None
    } else {
        Some(vec.write().await.remove(0))
    }
}   