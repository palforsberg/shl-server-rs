use std::fmt::write;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use reqwest::Error;
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;
use tracing::log;
use crate::{LogResult, CONFIG};
use crate::db::{Db};
use crate::models2::external::game_stats::GameStatsV2;
use crate::models2::external::player::PlayerStatsRsp;
use crate::models::{League, GameType, Season, SeasonKey, StringOrNum};
use crate::models2::external::season::SeasonRsp;

pub trait IdentifiableEnum {
    fn get_uuid(&self) -> &str;
}

impl IdentifiableEnum for League {
    fn get_uuid(&self) -> &str {
        match self {
            League::SHL => "qQ9-bb0bzEWUk",
            League::HA => "qQ9-594cW8OWD",
        }
    }
}

impl IdentifiableEnum for GameType {
    fn get_uuid(&self) -> &str {
        match self {
            GameType::Season => "qQ9-af37Ti40B",
            GameType::PlayOff => "qQ9-7debq38kX",
            GameType::Demotion => "qRf-347BaDIOc",
        }
    }
}

impl IdentifiableEnum for Season {
    fn get_uuid(&self) -> &str {
        match self {
            Season::Season2022 => "qbN-XMFfjGVt",
            Season::Season2021 => "qZl-8qa6OaFXf",
            Season::Season2020 => "qY7-AdVh5z1XJ",
            Season::Season2019 => "qWX-334j11U5o1",
            Season::Season2018 => "qUv-YXiuQN45",
        }
    }
}

pub async fn get_season(key: &SeasonKey) -> Option<SeasonRsp> {
    let season_param = format!("seasonUuid={}", key.0.get_uuid());
    let league_param = format!("seriesUuid={}", key.1.get_uuid());
    let game_type_param = format!("gameTypeUuid={}", key.2.get_uuid());
    let url = format!("{}/sports/game-info?gamePlace=all&played=all&{season_param}&{league_param}&{game_type_param}", CONFIG.get_url(&key.1));
    get_call(&url).await
}

pub async fn get_events(game_uuid: &str) -> Option<Vec<crate::models2::external::event::PlayByPlay>> {
    let url = format!("{}/gameday/play-by-play/initial-events/{game_uuid}", CONFIG.get_url(&League::SHL));
    get_call(&url).await
}

pub fn get_stats_url(league: &League, game_uuid: &str) -> String {
    format!("{}/gameday/periodstats/{game_uuid}", CONFIG.get_url(league))
}

pub fn get_player_stats_url(league: &League, game_uuid: &str) -> String {
    format!("{}/gameday/boxscore/{game_uuid}", CONFIG.get_url(league))
}

pub async fn throttle_call<T: DeserializeOwned + Serialize>(url: &str, throttle_s: Option<Duration>) -> Option<T> {
    let db = Db::<String, T>::new("rest");

    if db.is_stale(&url.to_string(), throttle_s) {
        let rsp: Option<T> = get_call(url).await;
        if let Some(rsp) = rsp {
            db.write(&url.to_string(), &rsp);
            Some(rsp)
        } else {
            None
        }
    } else {
        db.read(&url.to_string())
    }
}

async fn get_call<T: DeserializeOwned>(url: &str) -> Option<T> {
    let before = Instant::now();
    if let Some(rsp) = reqwest::get(url).await.ok_log("[API] Call failed") {
        let res = rsp.json().await.ok_log("[API] Parse failed");
        log::info!("[REST] Call {url} {:.2?}", before.elapsed());
        res
    } else {
        None
    }
}
