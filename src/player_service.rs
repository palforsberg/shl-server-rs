use std::time::Duration;

use serde::{Serialize, Deserialize};

use crate::{models::League, rest_client, models2::external::{player::{PlayerStatsRsp, PlayerName}, self}, db::Db};


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlayerGameStats {
    pub id: i32,
    pub first_name: String,
    pub family_name: String,
    pub jersey: i32,
    pub team_code: String,

    pub position: String,

    #[serde(rename = "+/-")]
    pub plus_minus: i32,
    pub a: i32,
    pub fol: i32,
    #[serde(rename = "fo%")]
    pub fo_perc: f32,
    pub fow: i32,
    pub g: i32,
    pub hits: i32,
    pub pim: i32,
    pub sog: i32,
    pub sw: i32,
    pub toi_s: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GoalkeeperGameStats {
    pub id: i32,
    pub first_name: String,
    pub family_name: String,
    pub jersey: i32,
    pub team_code: String,

    pub ga: i32,
    pub soga: i32,
    pub spga: i32,
    pub svs: i32,
    #[serde(rename = "svs%")]
    pub svs_perc: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Players {
    pub players: Vec<PlayerGameStats>,
    pub goalkeepers: Vec<GoalkeeperGameStats>,
}

impl From<(PlayerName, external::player::GoalkeeperStats)> for GoalkeeperGameStats {
    fn from(value: (PlayerName, external::player::GoalkeeperStats)) -> Self {
        let gk_info = value.0;
        let gk = value.1;
        GoalkeeperGameStats {
            id: gk.info.playerId,
            first_name: gk_info.firstName,
            family_name:  gk_info.lastName,
            jersey: gk.NR,
            team_code: gk.info.teamId.clone(),
            ga: gk.GA,
            soga: gk.SOGA,
            spga: gk.SPGA,
            svs: gk.SVS,
            svs_perc: gk.SVS_perc,
        }
    }
}
fn parse_toi(s: &str) -> i32 {
    let (min_str, secs_str) = s.split_once(':').unwrap_or(("0", "0"));
    let min: i32 = min_str.parse().ok().unwrap_or_default();
    let secs: i32 = secs_str.parse().ok().unwrap_or_default();
    min * 60 + secs
}
impl From<(PlayerName, external::player::PlayerStats)> for PlayerGameStats {
    fn from(value: (PlayerName, external::player::PlayerStats)) -> Self {
        let p_info = value.0;
        let p = value.1;
        PlayerGameStats {
            id: p.info.playerId,
            first_name: p_info.firstName,
            family_name: p_info.lastName,
            jersey: p.NR,
            team_code: p.info.teamId.clone(),
            position: p.POS.to_str(),
            plus_minus: p.plus_minus,
            a: p.A,
            fol: p.FOL,
            fo_perc: p.FOPerc,
            fow: p.FOW,
            g: p.G,
            hits: p.Hits,
            pim: p.PIM,
            sog: p.SOG,
            sw: p.SW,
            toi_s: parse_toi(&p.TOI),
        }
    }
}
impl From<PlayerStatsRsp> for Players {
    fn from(v: PlayerStatsRsp) -> Self {
        let gks = [v.gkStats.homeTeamValue, v.gkStats.awayTeamValue].concat();
        let mut gk_map = v.goalkeepers.homeTeamValue.clone();
        gk_map.extend(v.goalkeepers.awayTeamValue);

        let goalkeepers = gks.into_iter().map(|gk| {
            let gk_info = gk_map.get(&gk.info.playerId).cloned().unwrap_or_default();
            (gk_info, gk).into()
        }).collect();

        let ps = [v.stats.homeTeamValue, v.stats.awayTeamValue].concat();
        let mut player_map = v.players.homeTeamValue;
        player_map.extend(v.players.awayTeamValue);

        let players = ps.into_iter().map(|p| {
            let p_info = player_map.get(&p.info.playerId).cloned().unwrap_or_default();
            (p_info, p).into()
        }).collect();

        Players { players, goalkeepers }
    }
}

pub struct PlayerService;
impl PlayerService {

    pub async fn update(league: &League, game_uuid: &str, throttle_s: Option<Duration>) -> Players {
        let url = rest_client::get_player_stats_url(league, game_uuid);
        let rsp: Option<PlayerStatsRsp> = rest_client::throttle_call(&url, throttle_s).await;
        rsp.map(|e| e.into()).unwrap_or_default()
    }

    pub fn read(league: &League, game_uuid: &str) -> Option<Players> {
        let db = Db::<String, PlayerStatsRsp>::new("rest");
        let url = rest_client::get_player_stats_url(league, game_uuid);
        let rsp: Option<PlayerStatsRsp> = db.read(&url);
        rsp.map(|e| e.into())
    }

    pub fn is_stale(league: &League, game_uuid: &str) -> bool {
        let url = rest_client::get_player_stats_url(league, game_uuid);
        let db = Db::<String, PlayerStatsRsp>::new("rest");
        db.is_stale(&url, None)
    }
}