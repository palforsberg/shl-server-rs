use std::{sync::Arc, time::Duration, collections::HashMap, net::SocketAddr, convert::Infallible};

use async_stream::try_stream;
use axum::{Router, extract::{Path, State, Query}, response::{IntoResponse, Sse, sse::{KeepAlive, Event}}, Json, body::StreamBody, routing::{get, post}};
use chrono::Utc;
use reqwest::StatusCode;
use serde::Deserialize;
use shl_server_rs::{models_external::{season::{SeasonGame, GameTeamInfo, SeasonRsp, SeriesInfo}, event::SseEvent}, models::StringOrNum};
use tokio::{sync::{RwLock, broadcast::Sender}, task::JoinHandle};
use tokio_util::io::ReaderStream;

use super::models_apn::ApnBody;

#[derive(Deserialize)]
struct SportsQuery {
    seasonUuid: String,
    seriesUuid: String,
    gameTypeUuid: String,
}

#[derive(Clone)]
pub struct AppState {
    pub sender: Sender<Event>,
    pub notifications: Vec<(String, ApnBody)>,
    pub live_acitivies: HashMap<String, u16>,
}


async fn remove_first<T>(vec: Arc<RwLock<Vec<T>>>) -> Option<T> {
    if vec.read().await.is_empty() {
        None
    } else {
        Some(vec.write().await.remove(0))
    }
}

pub struct ExternalServer {
    port: u16,
    handles: Vec<JoinHandle<()>>,

    sse_events: Arc<RwLock<Vec<SseEvent>>>,
}

impl Drop for ExternalServer {
    fn drop(&mut self) {
        for e in &self.handles {
            e.abort();
        }
    }
}

impl ExternalServer {
    pub fn new(port: u16) -> ExternalServer {
        ExternalServer { port, handles: vec![], sse_events: Arc::new(RwLock::new(vec![])) }
    }

    pub async fn start(&mut self) -> Arc<RwLock<AppState>> {
        let (sse_handle, sse_sender) = self.start_sse_listener(Duration::from_micros(10));

        let external_mock_state = Arc::new(RwLock::new(AppState { sender: sse_sender, notifications: vec![], live_acitivies: HashMap::new() }));
        let external_mock = {
            let port = self.port;
            let state = external_mock_state.clone();
            tokio::spawn(async move { ExternalServer::serve_external_data(state, port).await })
        };

        self.handles.push(external_mock);
        self.handles.push(sse_handle);

        tokio::time::sleep(Duration::from_secs(2)).await; // wait for mock to start
        
        external_mock_state
    }

    pub async fn push_events(&mut self, events: Vec<SseEvent>) {
        self.sse_events.write().await.extend(events);
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
    
    async fn get_periodstats_file(Path(game_uuid): Path<String>) -> impl IntoResponse {
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

    async fn get_sports_file(query: Query<SportsQuery>) -> impl IntoResponse {
        let path = format!("./tests/integration/external/sports/game-info?gamePlace=all&played=all&seasonUuid={}&seriesUuid={}&gameTypeUuid={}", query.seasonUuid, query.seriesUuid, query.gameTypeUuid);
        let mut data = std::fs::read_to_string(path).ok().and_then(|e| serde_json::from_str::<SeasonRsp>(&e).ok()).unwrap();
        data.gameInfo.push(SeasonGame { 
            uuid: "qcv-34ekyLqu8".to_string(), 
            homeTeamInfo: GameTeamInfo { code: "SAIK".to_string(), score: StringOrNum::Number(0) }, 
            awayTeamInfo: GameTeamInfo { code: "OHK".to_string(), score: StringOrNum::Number(0) }, 
            startDateTime: Utc::now() - chrono::Duration::minutes(5),
            state: "pre-game".to_string(), 
            shootout: false,
            overtime: false, 
            seriesInfo: SeriesInfo { code: shl_server_rs::models::League::SHL },
        });
        Json(data)
    }
    
    async fn get_sse(Path(game_uuid): Path<String>, State(state): State<Arc<RwLock<AppState>>>) -> Sse<impl futures::Stream<Item = Result<Event, Infallible>>> {
        let mut receiver = state.write().await.sender.subscribe();
        println!("[TEST] New SSE subscriber for {}", game_uuid);
        Sse::new(try_stream! {
            loop {
                match receiver.recv().await {
                    Ok(i) => { yield i; },
                    Err(e) => { tracing::error!(error = ?e, "Failed to get"); }
                }
            }
        })
        .keep_alive(KeepAlive::default())
    }

    fn start_sse_listener(&self, sleep_time: Duration) -> (JoinHandle<()>, Sender<Event>) {
        let (sender, _) = tokio::sync::broadcast::channel(10);
        println!("[TEST] Start SSE events");
        let handle = {
            let sender = sender.clone();
            let events = self.sse_events.clone();
            tokio::spawn(async move {
                loop {
                    if sender.receiver_count() > 0 {
                        println!("[TEST] SSE subscribers now {}", sender.receiver_count());
                        while let Some(entry) = remove_first(events.clone()).await {
                            println!("[TEST] Sending event");
                            let json_str = serde_json::to_string(&entry).expect("should encode to json");
                            sender.send(Event::default().data(json_str)).expect("should broadcast event");
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
    
}