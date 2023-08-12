
use futures::StreamExt;
use reqwest_eventsource::{EventSource, Event};
use tokio::{task::JoinHandle, sync::mpsc::Receiver};
use tracing::log;

use crate::{models_external::{self, event::SseEvent}, LogResult, CONFIG};


pub enum SseMsg {
    Report(models_external::event::GameReport),
    Event(models_external::event::PlayByPlay),
}
pub struct SseClient;
impl SseClient {
    pub async fn spawn_listener(game_uuid: &str) -> (JoinHandle<()>, Receiver<(String, SseMsg)>) {
        let (sender, receiver) = tokio::sync::mpsc::channel(1000);
        let uuid = game_uuid.to_string();
        let handle = tokio::spawn(async move {
            log::info!("[SSE] Start listen to {uuid}");
            let mut last_report_id: u16 = 0;
            let mut last_event_id = "".to_string();
            let mut es = EventSource::get(format!("{}/{uuid}?instanceId=shl1_shl", CONFIG.sse_url));
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
                            if report.revision != last_report_id {
                                sender.send((uuid.to_string(), SseMsg::Report(report.clone()))).await
                                    .ok_log("[SSE] Error sending report");
                            }
                            last_report_id = report.revision;
                        }
                        if let Some(event) = event.playByPlay.map(|e| e.actions[0].clone()) {
                            if event.hash != last_event_id {
                                sender.send((uuid.to_string(), SseMsg::Event(event.clone()))).await
                                    .ok_log("[SSE] Error sending event");
                            }
                            last_event_id = event.hash.clone();
                        }
                    }
                    log::debug!("[SSE] task");
                }
            }
        });
        (handle, receiver)
    }
}
