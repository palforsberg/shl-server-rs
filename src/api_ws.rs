use std::time::Duration;

use axum::extract::ws::{WebSocket, Message};
use futures::{StreamExt, SinkExt};
use serde::{Serialize, Deserialize};
use tokio::select;
use tracing::log;

use crate::{api::ApiState, event_service::ApiGameEvent, game_report_service::ApiGameReport, stats_service::ApiGameStats};




#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WsMsg {
    pub game_uuid: String,
    
    #[serde(flatten)]
    pub body: WsMsgBody,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type", rename_all="snake_case")]
pub enum WsMsgBody {
    Event { event: ApiGameEvent },
    Report { report: ApiGameReport },
    Stats { stats: ApiGameStats }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WsReq {
    pub game_uuid: String,
}


impl From<ApiGameEvent> for WsMsg {
    fn from(event: ApiGameEvent) -> Self {
        WsMsg { game_uuid: event.game_uuid.clone(), body: WsMsgBody::Event { event } }
    }
}
impl From<ApiGameReport> for WsMsg {
    fn from(report: ApiGameReport) -> Self {
        WsMsg { game_uuid: report.game_uuid.clone(), body: WsMsgBody::Report { report } }
    }
}
pub struct ApiWs {

}

impl ApiWs {
    pub async fn handle(stream: WebSocket, state: ApiState) {
        let (mut sender, mut receiver) = stream.split();
        let mut broadcast_receiver = state.broadcast_sender.subscribe();

        log::info!("[API.WS] Open, in total = {}", ApiWs::update_nr_connections(1, &state).await);

        let receive_handle = tokio::spawn(async move {
            while let Some(Ok(msg)) = receiver.next().await {
                if let Some(ws_req) = msg.into_text().ok().and_then(|e| serde_json::from_str::<WsReq>(&e).ok()) {
                    log::info!("[API.WS] Req {:?}", ws_req);
                }
            }
        });
        _ = tokio::spawn(async move {
            loop {
                let msg = select! {
                    msg = broadcast_receiver.recv() => match msg {
                        Ok(msg) => Message::Text(serde_json::to_string(&msg).unwrap_or_default()),
                        Err(e) => {
                            log::error!("[API.WS] broadcast receive {:?}", e);
                            Message::Pong(vec![])
                        }
                    },
                    _ = tokio::time::sleep(Duration::from_secs(60)) => {
                        log::info!("[API.WS] ping");
                        // if no broadcast is received, send a ping every 60 sec to ensure connection is open
                        Message::Ping(vec![42])
                    }
                };
                if let Err(e) = sender.send(msg).await {
                    log::info!("[API.WS] Error sending {e}");
                    break;
                }
            }
        }).await;
        
        receive_handle.abort();

        log::info!("[API.WS] Close, in total = {}", ApiWs::update_nr_connections(-1, &state).await);
    }

    async fn update_nr_connections(delta: i16, state: &ApiState) -> i16{
        let mut nr_ws = state.nr_ws.write().await;
        *nr_ws += delta;
        *nr_ws
    }
}