use tokio::sync::broadcast::{Sender, Receiver, self};

use crate::{game_report_service::ApiGameReport, event_service::ApiGameEvent, stats_service::ApiGameStats, LogResult};

#[derive(Clone)]
pub enum Msg {
    SseEvent { event: ApiGameEvent, game_uuid: String, new_event: bool },
    SseReport{ report: ApiGameReport },
    SseClosed { game_uuid: String },
    GameEnded { game_uuid: String },
    GameStarted { game_uuid: String },
    StatsFetched { game_uuid: String, stats: ApiGameStats },
}


pub struct MsgBus {
    sender: Sender<Msg>,
}

impl MsgBus {
    pub fn new() -> MsgBus {
        let (sender, _) = broadcast::channel(1000);
        MsgBus { sender }
    }

    pub fn subscribe(&self) -> Receiver<Msg> {
        self.sender.subscribe()
    }

    pub fn send(&self, msg: Msg) {
        self.sender.send(msg)
            .ok_log("[MSGBUS] Error sending");
    }
}