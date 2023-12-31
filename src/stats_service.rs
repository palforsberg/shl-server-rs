use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::{models::{League, Season}, rest_client::{self}, models_external::game_stats::StatsRsp, db::Db, models_api::stats::{ApiGameStats, ApiGameTeamStats}};


#[derive(Serialize, Deserialize, Clone)]
struct StatsEntry {
    key: String,
    value: i32,
}

#[derive(Serialize, Deserialize, Clone)]
struct TeamPeriodStats {
    period: u8,
    parsedTotalStatistics: Vec<StatsEntry>
}

#[derive(Serialize, Deserialize, Clone, Default)]
struct TeamStats {
    statistics: Vec<TeamPeriodStats>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
struct TeamStatsRsp {
    home: TeamStats,
    away: TeamStats,
}

impl From<TeamStatsRsp> for ApiGameStats {
    fn from(v: TeamStatsRsp) -> Self {

        fn get_key(key: &str, entries: &[StatsEntry]) -> i32 {
            entries.iter().find(|e| e.key == key).map(|e| e.value).unwrap_or_default()
        }

        let home = v.home.statistics.iter().find(|e| e.period == 0)
            .map(|e| e.parsedTotalStatistics.clone())
            .unwrap_or_default();
        let away = v.away.statistics.iter().find(|e| e.period == 0)
            .map(|e| e.parsedTotalStatistics.clone())
            .unwrap_or_default();
        let home_stats = ApiGameTeamStats {
            g: get_key("G", &home),
            sog: get_key("SOG", &home),
            pim: get_key("PIM", &home),
            fow: get_key("FOW", &home),
        };
        let away_stats = ApiGameTeamStats {
            g: get_key("G", &away),
            sog: get_key("SOG", &away),
            pim: get_key("PIM", &away),
            fow: get_key("FOW", &away),
        };

        ApiGameStats { home: home_stats, away: away_stats }
    }
}

impl From<StatsRsp> for ApiGameStats {
    fn from(v: StatsRsp) -> Self {
        let stats = v.period_stats_breakdown.iter()
            .find(|e| e.period.value.to_str() == "Total")
            .map(|e| e.statistics.clone());
        
        let goals = &stats.as_ref().and_then(|e| e.iter().find(|e| e.caption == "G"));
        let sog = &stats.as_ref().and_then(|e| e.iter().find(|e| e.caption == "SOG"));
        let fow = &stats.as_ref().and_then(|e| e.iter().find(|e| e.caption == "FOWon"));
        let pim = &stats.as_ref().and_then(|e| e.iter().find(|e| e.caption == "PIM"));

        let home = ApiGameTeamStats { 
            g: goals.map(|e| e.homeTeamValue).unwrap_or_default(), 
            sog: sog.map(|e| e.homeTeamValue).unwrap_or_default(),
            pim: pim.map(|e| e.homeTeamValue).unwrap_or_default(),
            fow: fow.map(|e| e.homeTeamValue).unwrap_or_default(),
        };

        let away = ApiGameTeamStats { 
            g: goals.map(|e| e.awayTeamValue).unwrap_or_default(), 
            sog: sog.map(|e| e.awayTeamValue).unwrap_or_default(),
            pim: pim.map(|e| e.awayTeamValue).unwrap_or_default(),
            fow: fow.map(|e| e.awayTeamValue).unwrap_or_default(),
        };
        ApiGameStats { home, away }
    }
}
pub struct StatsService;

impl StatsService {

    pub async fn update(league: &League, season: &Season, game_uuid: &str, throttle_s: Option<Duration>) -> Option<ApiGameStats> {
        match season {
            Season::Season2023 => StatsService::update_2023(league, game_uuid, throttle_s).await,
            _ => StatsService::update_old(league, game_uuid, throttle_s).await,
        }
    }

    pub async fn update_2023(league: &League, game_uuid: &str, throttle_s: Option<Duration>) -> Option<ApiGameStats> {
        let url = rest_client::get_team_stats_url(league, game_uuid);
        let rsp: Option<TeamStatsRsp> = rest_client::throttle_call(&url, throttle_s).await;
        rsp.map(|e| e.into())
    }

    pub async fn update_old(league: &League, game_uuid: &str, throttle_s: Option<Duration>) -> Option<ApiGameStats> {
        let url = rest_client::get_stats_url(league, game_uuid);
        let rsp: Option<StatsRsp> = rest_client::throttle_call(&url, throttle_s).await;
        rsp.map(|e| e.into())
    }

    pub fn is_stale(league: &League, game_uuid: &str, throttle_s: Option<Duration>) -> bool {
        let url = rest_client::get_stats_url(league, game_uuid);
        let db = Db::<String, StatsRsp>::new("rest");
        db.is_stale(&url, throttle_s)
    }
}
