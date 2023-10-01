
use std::{io::Write, fs::{OpenOptions, File}};

use futures::StreamExt;
use reqwest_eventsource::{EventSource, Event};
use tokio::{task::JoinHandle, sync::mpsc::Receiver};
use tracing::log;

use crate::{models_external::{self, event::{SseEvent, LiveEvent, TeamStatistics, GameTime, LiveStateEvent}}, LogResult, CONFIG};


struct FileAppend {
    file: Option<File>,
}
impl FileAppend {
    fn new(id: &str) -> FileAppend {
        let path = std::path::PathBuf::from(format!("./log/{id}.log"));
        std::fs::create_dir_all(path.parent().unwrap()).ok_log("could not create directory");
        log::info!("create apth {:?}", path);
        let file = OpenOptions::new()
            .create(true)
            .write(true).append(true).open(path)
            .ok_log("could not create FileAppend");
        FileAppend {
            file,
        }
    }

    fn append(&mut self, val: &str) {
        if self.file.is_some() && !val.is_empty() {
            writeln!(self.file.as_ref().unwrap(), "{}", val)
                .ok_log("Failed appending to file");
        }
    }
}

pub enum SseMsg {
    Report(models_external::event::GameReport),
    GameTime(models_external::event::GameTime),
    Event(models_external::event::PlayByPlay),
    LiveEvent(LiveEvent),
    TeamStats(TeamStatistics),
    LiveState(LiveStateEvent),
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
            let mut es = EventSource::get(format!("{}?gameUuid={uuid}", CONFIG.sse_url));
            let mut file_append = FileAppend::new(&uuid);
            //"https://sse.dev/test");
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(CONFIG.sse_sleep)).await;
                if let Some(event) = es.next().await {
                    let message = match event {
                        Ok(Event::Open) => { log::info!("[SSE] Open {uuid}"); None }
                        Ok(Event::Message(message)) => { Some(message) }
                        Err(err) => { log::error!("[SSE] Error: {err}"); None }
                    };
                    if CONFIG.sse_file_append {
                        file_append.append(&message.as_ref().map(|e| e.data.clone()).unwrap_or("".to_string()));
                    }
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
                        if let Some(live_event) = event.liveEvent {
                            sender.send((uuid.to_string(), SseMsg::LiveEvent(live_event))).await
                                .ok_log("[SSE] Error sending live_event");
                        }
                        if let Some(game_time) = event.gameTime {
                            if let Some(game_time) = Option::<GameTime>::from(game_time) {
                                sender.send((uuid.to_string(), SseMsg::GameTime(game_time))).await
                                    .ok_log("[SSE] Error sending game_time");
                            } else {
                                log::info!("[SSE] empty game_time");
                            }
                        }
                        if let Some(team_stats) = event.teamStatistics {
                            sender.send((uuid.to_string(), SseMsg::TeamStats(team_stats))).await
                                .ok_log("[SSE] Error sending team_stats");
                        }
                        if let Some(live_state) = event.liveState {
                            sender.send((uuid.to_string(), SseMsg::LiveState(live_state))).await
                                .ok_log("[SSE] Error sending live_state");
                        }
                    }
                    log::debug!("[SSE] task");
                }
            }
        });
        (handle, receiver)
    }
}
