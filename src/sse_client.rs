use std::{collections::HashMap, sync::Arc, sync::Mutex};

use futures::{StreamExt, Stream};
use reqwest_eventsource::{EventSource, Event};
use serde::{Deserialize, Serialize};
use tokio::{task::JoinHandle, sync::mpsc::{Sender, Receiver}};
use tracing::log;

use crate::{game_report_service::ApiGameReport, models::StringOrNum, models2::external::{self, event::{SseEvent, PlayByPlay, GameReport}}, LogResult, CONFIG};


pub enum SseMsg {
    Report(String, external::event::GameReport),
    Event(String, external::event::PlayByPlay),
}
pub struct SseClient;
impl SseClient {
    pub async fn spawn_listener(game_uuid: &str) -> (JoinHandle<()>, Receiver<(String, external::event::GameReport)>, Receiver<(String, external::event::PlayByPlay)>) {
        let (report_sender, report_receiver) = tokio::sync::mpsc::channel(10);
        let (event_sender, event_receiver) = tokio::sync::mpsc::channel(10);
        let uuid = game_uuid.to_string();
        let handle = tokio::spawn(async move {
            log::info!("[SSE] Start listen to {uuid}");
            let mut last_report_id: u16 = 0;
            let mut last_event_id = "".to_string();
            let mut es = EventSource::get(format!("{}{uuid}?instanceId=shl1_shl", CONFIG.sse_url));
            //"https://sse.dev/test");
            loop {
                if let Some(event) = es.next().await {
                    let message = match event {
                        Ok(Event::Open) => { log::info!("[SSE] Open"); None }
                        Ok(Event::Message(message)) => { Some(message) }
                        Err(err) => { log::error!("[SSE] Error: {err}"); None }
                    };
                    let event = message
                            .and_then(|msg| serde_json::from_str::<SseEvent>(&msg.data)
                            .ok_log(&format!("[SSE] Parse failed {}", &msg.data)));
                    if let Some(event) = event {
                        if let Some(report) = event.gameReport {
                            if (report.revision != last_report_id) {
                                report_sender.send((uuid.to_string(), report.clone())).await;
                            }
                            last_report_id = report.revision;
                        }
                        if let Some(event) = event.playByPlay.map(|e| e.actions[0].clone()) {
                            if event.hash != last_event_id {
                                event_sender.send((uuid.to_string(), event.clone())).await;
                            }
                            last_event_id = event.hash.clone();
                        }
                    }
                    log::debug!("[SSE] task");
                }
            }
            log::error!("[SSE] Thread stopped {uuid}");
        });
        (handle, report_receiver, event_receiver)
    }
}
