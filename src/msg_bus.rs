use tokio::sync::broadcast::{Sender, Receiver, self};

use crate::{models_api::{event::ApiGameEvent, report::ApiGameReport}, LogResult};


#[derive(Clone)]
pub enum Msg {
    Event { event: ApiGameEvent, game_uuid: String, new_event: bool },
    Report{ report: ApiGameReport, game_uuid: String },
    SseClosed { game_uuid: String },
}

impl Msg {
    pub fn get_game_uuid(&self) -> &String {
        match self {
            Msg::SseClosed { game_uuid } => game_uuid,
            Msg::Event { event:_, game_uuid, new_event:_ } => game_uuid,
            Msg::Report { report:_, game_uuid} => game_uuid,
         }
    }
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