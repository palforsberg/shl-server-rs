use std::time::Duration;

use crate::{models::{League, Season}, rest_client, models_external::{player::{PlayerStatsRsp, PlayerName}, self}, db::Db, models_api::athlete::{ApiAthlete, ApiGoalkeeperStats, ApiAthleteStats, ApiPlayerStats}};

fn parse_toi(s: &str) -> i32 {
    let (min_str, secs_str) = s.split_once(':').unwrap_or(("0", "0"));
    let min: i32 = min_str.parse().ok().unwrap_or_default();
    let secs: i32 = secs_str.parse().ok().unwrap_or_default();
    min * 60 + secs
}

impl ApiAthlete {
    fn to_gk(league: League, name: PlayerName, gk: models_external::player::GoalkeeperStats) -> Self {
        let stats = ApiGoalkeeperStats {
            ga: gk.GA,
            soga: gk.SOGA,
            spga: gk.SPGA,
            svs: gk.SVS,
            gp: match gk.SVS > 0 { true => 1, false => 0 },
        };
        ApiAthlete { 
            id: gk.info.playerId, 
            first_name: name.firstName,
            family_name: name.lastName,
            jersey: gk.NR,
            season: Season::Season2023,
            league,
            team_code: gk.info.teamId,
            position: "GK".to_string(),
            stats: ApiAthleteStats::Goalkeeper(stats)
        }
    }
    fn to_p(league: League, name: PlayerName, p: models_external::player::PlayerStats) -> Self {
        let stats = ApiPlayerStats {
            plus_minus: p.plus_minus,
            a: p.A,
            fol: p.FOL,
            fow: p.FOW,
            g: p.G,
            hits: p.Hits,
            pim: p.PIM,
            sog: p.SOG,
            sw: p.SW,
            toi_s: parse_toi(&p.TOI),
            gp: 1,
        };
        ApiAthlete { 
            id: p.info.playerId,
            first_name: name.firstName,
            family_name: name.lastName,
            jersey: p.NR,
            season: Season::Season2023,
            league,
            team_code: p.info.teamId,
            position: p.POS.to_str(),
            stats: ApiAthleteStats::Player(stats), 
        }
    }
    fn to_vec(league: League, v: PlayerStatsRsp) -> Vec<Self> {
        let gks = [v.gkStats.homeTeamValue, v.gkStats.awayTeamValue].concat();
        let mut gk_map = v.goalkeepers.homeTeamValue.clone();
        gk_map.extend(v.goalkeepers.awayTeamValue);

        let goalkeepers: Vec<ApiAthlete> = gks.into_iter().map(|gk| {
            let gk_info = gk_map.get(&gk.info.playerId).cloned().unwrap_or_default();
            ApiAthlete::to_gk(league.clone(), gk_info, gk)
        }).collect();

        let ps = [v.stats.homeTeamValue, v.stats.awayTeamValue].concat();
        let mut player_map = v.players.homeTeamValue;
        player_map.extend(v.players.awayTeamValue);

        let players: Vec<ApiAthlete> = ps.into_iter().map(|p| {
            let p_info = player_map.get(&p.info.playerId).cloned().unwrap_or_default();
            ApiAthlete::to_p(league.clone(), p_info, p)
        }).collect();

        [players, goalkeepers].concat()
    }
}

pub struct PlayerService;
impl PlayerService {

    pub async fn update(league: &League, game_uuid: &str, throttle_s: Option<Duration>) -> Vec<ApiAthlete> {
        let url = rest_client::get_player_stats_url(league, game_uuid);
        let rsp: Option<PlayerStatsRsp> = rest_client::throttle_call(&url, throttle_s).await;
        rsp.map(|e| ApiAthlete::to_vec(league.clone(), e)).unwrap_or_default()
    }

    pub fn read(league: &League, game_uuid: &str) -> Option<Vec<ApiAthlete>> {
        let db = Db::<String, PlayerStatsRsp>::new("rest");
        let url = rest_client::get_player_stats_url(league, game_uuid);
        let rsp: Option<PlayerStatsRsp> = db.read(&url);
        rsp.map(|e| ApiAthlete::to_vec(league.clone(), e))
    }

    pub fn is_stale(league: &League, game_uuid: &str, throttle: Option<Duration>) -> bool {
        let url = rest_client::get_player_stats_url(league, game_uuid);
        let db = Db::<String, PlayerStatsRsp>::new("rest");
        db.is_stale(&url, throttle)
    }
}