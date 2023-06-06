use std::{collections::HashMap, time::Instant};

use serde::{Serialize, Deserialize};
use tracing::log;

use crate::{player_service::{PlayerService}, api_season_service::{ApiGame}, db::Db, game_report_service::GameStatus, models::Season};


/**
 * keys: id -> see all seasons, teams etc.
 * season + team -> Players[]
 */

type Team = String;

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct PlayerSeasonKey (// -> PlayerSeasonStats // when
    pub i32,
    pub Season,
    pub Team,
);
impl std::fmt::Display for PlayerSeasonKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}/{}", self.0, self.1, self.2)
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct TeamSeasonKey(pub Season, pub Team); // => Vec<PlayerSeasonStats>
impl std::fmt::Display for TeamSeasonKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.0, self.1)
    }
}

// type PlayerKey = i32; // => Vec<PlayerSeasonStats>

#[derive(Serialize, Deserialize, Clone)]
pub struct ApiPlayerSeasonStats {
    pub player_id: i32,
    pub first_name: String,
    pub family_name: String,
    pub jersey: i32,
    pub season: Season,
    pub team: String,
    pub position: String,
    pub stats: SeasonStats,
}

#[derive(Serialize, Deserialize, Clone, Copy, Default)]
pub struct PlayerPlayerSeasonStats {
    pub a: i32,
    pub g: i32,
    pub plus_minus: i32,
    pub gp: i32,
    pub sog: i32,
    pub toi_s: i32,
    pub pim: i32,
    pub fow: i32,
    pub hits: i32,
}

#[derive(Serialize, Deserialize, Clone, Copy, Default)]
pub struct GoalkeeperSeasonStats {
    pub ga: i32,
    pub soga: i32,
    pub spga: i32,
    pub svs: i32,
    pub gp: i32,
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub enum SeasonStats {
    Player(PlayerPlayerSeasonStats),
    Goalkeeper(GoalkeeperSeasonStats),
}
pub struct ApiPlayerStatsService {
}

impl ApiPlayerStatsService {
    pub fn update(games: &[ApiGame]) {
        log::info!("[API.PLAYERSTATS] Start with {} games", games.len());
        let before = Instant::now();
        let all_players = games.iter()
            .filter(|e| !matches!(e.status, GameStatus::Coming))
            .filter_map(|e| PlayerService::read(&e.league, &e.game_uuid).map(|stats| (e, stats)));
        
        let mut player_map: HashMap<PlayerSeasonKey, ApiPlayerSeasonStats> = HashMap::new();

        for (game, stats) in all_players {
            for e in stats.players {
                let key = PlayerSeasonKey(e.id, game.season.clone(), e.team_code.clone());
                let entry = player_map.entry(key).or_insert_with(|| ApiPlayerSeasonStats {
                    player_id: e.id, 
                    season: game.season.clone(),
                    team: e.team_code.clone(), 
                    position: e.position.clone(),
                    first_name: e.first_name.clone(),
                    family_name: e.family_name.clone(),
                    jersey: e.jersey,
                    stats: SeasonStats::Player(PlayerPlayerSeasonStats { ..Default::default() })
                });
                if let SeasonStats::Player(player_stats) = &mut entry.stats {
                    player_stats.a += e.a;
                    player_stats.g += e.g;
                    player_stats.gp += 1;
                    player_stats.sog += e.sog;
                    player_stats.pim += e.pim;
                    player_stats.plus_minus += e.plus_minus;
                    player_stats.toi_s += e.toi_s;
                    player_stats.fow += e.fow;
                    player_stats.hits += e.hits;
                }
            }
            for e in stats.goalkeepers {
                let key = PlayerSeasonKey(e.id, game.season.clone(), e.team_code.clone());
                let entry = player_map.entry(key).or_insert_with(|| ApiPlayerSeasonStats {
                    player_id: e.id, 
                    season: game.season.clone(),
                    team: e.team_code.clone(), 
                    position: "GK".to_string(),
                    first_name: e.first_name.clone(),
                    family_name: e.family_name.clone(),
                    jersey: e.jersey,
                    stats: SeasonStats::Goalkeeper(GoalkeeperSeasonStats { ..Default::default() })
                });
                if let SeasonStats::Goalkeeper(a) = &mut entry.stats {
                    a.ga += e.ga;
                    a.soga += e.soga;
                    a.spga += e.spga;
                    a.svs += e.svs;
                    a.gp += match e.svs > 0 { true => 1, false => 0 };
                }
            }
        }

        log::info!("[API.PLAYERSTATS] Found {} players", player_map.len());

        let teams_player_map = player_map.iter().fold(HashMap::new(), |mut map, player_entry| {
            let key = TeamSeasonKey(player_entry.0.1.clone(), player_entry.1.team.clone());
            map
                .entry(key)
                .or_insert_with(Vec::new)
                .push(player_entry.1.clone());
            map
        });
        let teams_db = ApiPlayerStatsService::get_team_player_db();
        for (id, p) in &teams_player_map {
            _ = teams_db.write(id, p);
        }

        let player_career_map = player_map.iter().fold(HashMap::new(), |mut map, player_entry| {
            let key = player_entry.0.0;
            map
                .entry(key)
                .or_insert_with(Vec::new)
                .push(player_entry.1.clone());
            map
        });
        let career_db = ApiPlayerStatsService::get_player_career_db();
        for (id, p) in &player_career_map {
            _ = career_db.write(id, p);
        }

        log::info!("[API.PLAYERSTATS] Finished in {:.0?}", before.elapsed());
    }

    pub fn get_player_career_db() -> Db<i32, Vec<ApiPlayerSeasonStats>> {
        Db::<i32, Vec<ApiPlayerSeasonStats>>::new("v2_api_player_career")
    }

    pub fn get_team_player_db() -> Db<TeamSeasonKey, Vec<ApiPlayerSeasonStats>> {
        Db::<TeamSeasonKey, Vec<ApiPlayerSeasonStats>>::new("v2_api_team_players")
    }
}


#[cfg(test)]
mod tests {
    use std::{collections::HashMap};

    use chrono::Utc;
    use tempdir::TempDir;

    use crate::{api_player_stats_service::{ApiPlayerStatsService, SeasonStats, TeamSeasonKey}, api_season_service::ApiGame, models2::external::player::{PlayerStatsRsp, EachTeamStats, PlayerName, PlayerStats, GoalkeeperStats}, rest_client, db::Db};

    fn before() {
        std::env::set_var("DB_PATH", TempDir::new("test").expect("dir to be created").path().to_str().unwrap());
    }

    #[test]
    fn sunny_day_player() {
        before();
        let team = "LHF";
        let rest_db = Db::<String, PlayerStatsRsp>::new("rest");
        let player_id = 123;
        let game1 = get_played_game("game1", team);
        let game2 = get_played_game("game2", team);
        let player = get_player(player_id, team);
        let playerRsp = PlayerStatsRsp {
            stats: EachTeamStats { homeTeamValue: [player.0.clone()].to_vec(), awayTeamValue: [].to_vec() },
            players: EachTeamStats { homeTeamValue: HashMap::from([(player.0.info.playerId, player.1)]), awayTeamValue: HashMap::from([]), },
            ..Default::default()
        };

        //Store
        _ = rest_db.write(&rest_client::get_player_stats_url(&crate::models::League::SHL, &game1.game_uuid), &playerRsp);
        _ = rest_db.write(&rest_client::get_player_stats_url(&crate::models::League::SHL, &game2.game_uuid), &playerRsp);

        ApiPlayerStatsService::update(&[game1, game2]);

        let player_db = ApiPlayerStatsService::get_player_career_db();
        let stored_players = player_db.read(&player_id);
        assert!(stored_players.is_some());
        
        let stored_player = stored_players.unwrap();
        assert_eq!(stored_player.len(), 1);
        assert_eq!(stored_player[0].player_id, player_id);
        assert_eq!(stored_player[0].team, team.to_string());

        let stats = match stored_player[0].stats {
            SeasonStats::Player(a) => a,
            _ => panic!("not good"),
        };
        assert_eq!(stats.a, 4);
        assert_eq!(stats.g, 4);
        assert_eq!(stats.gp, 2);
        assert_eq!(stats.toi_s, 817 * 2);

        let team_db = ApiPlayerStatsService::get_team_player_db();
        let stored_team = team_db.read(&TeamSeasonKey(crate::models::Season::Season2022, team.to_string())).unwrap();
        assert_eq!(stored_team.len(), 1);
        let team_player = stored_team.get(0).unwrap();
        assert_eq!(team_player.player_id, stored_player[0].player_id);
        let team_stats = match team_player.stats {
            SeasonStats::Player(a) => a,
            _ => panic!("not good"),
        };
        assert_eq!(team_stats.a, stats.a);
    }

    #[test]
    fn sunny_day_goalkeeper() {
        before();
        let team = "TIK";
        let rest_db = Db::<String, PlayerStatsRsp>::new("rest");
        let player_id = 1234;
        let game1 = get_played_game("game1_2", team);
        let game2 = get_played_game("game2_2", team);
        let player = get_goalkeeper(player_id, team);
        let playerRsp = PlayerStatsRsp {
            gkStats: EachTeamStats { homeTeamValue: [player.0.clone()].to_vec(), awayTeamValue: [].to_vec() },
            goalkeepers: EachTeamStats { homeTeamValue: HashMap::from([(player.0.info.playerId, player.1)]), awayTeamValue: HashMap::from([]), },
            ..Default::default()
        };

        //Store player rsps
        _ = rest_db.write(&rest_client::get_player_stats_url(&crate::models::League::SHL, &game1.game_uuid), &playerRsp);
        _ = rest_db.write(&rest_client::get_player_stats_url(&crate::models::League::SHL, &game2.game_uuid), &playerRsp);

        ApiPlayerStatsService::update(&[game1, game2]);
        let player_db = ApiPlayerStatsService::get_player_career_db();
        let stored_players = player_db.read(&player_id);
        assert!(stored_players.is_some());
        
        let stored_player = stored_players.unwrap();
        assert_eq!(stored_player.len(), 1);
        assert_eq!(stored_player[0].player_id, player_id);
        assert_eq!(stored_player[0].team, team.to_string());

        let stats = match stored_player[0].stats {
            SeasonStats::Goalkeeper(a) => a,
            _ => panic!("not good"),
        };
        assert_eq!(stats.svs, 10);
        assert_eq!(stats.ga, 2);
        assert_eq!(stats.gp, 2);
    }

    #[test]
    fn goalkeeper_without_saves_shouldnt_have_gp() {
        before();
        let team = "FHC";
        let rest_db = Db::<String, PlayerStatsRsp>::new("rest");
        let player_id = 12345;
        let game1 = get_played_game("game1_3", team);
        let game2 = get_played_game("game2_3", team);
        let mut player = get_goalkeeper(player_id, team);
        player.0.SVS = 0;
        let playerRsp = PlayerStatsRsp {
            gkStats: EachTeamStats { homeTeamValue: [player.0.clone()].to_vec(), awayTeamValue: [].to_vec() },
            goalkeepers: EachTeamStats { homeTeamValue: HashMap::from([(player.0.info.playerId, player.1)]), awayTeamValue: HashMap::from([]), },
            ..Default::default()
        };

        //Store player rsps
        _ = rest_db.write(&rest_client::get_player_stats_url(&crate::models::League::SHL, &game1.game_uuid), &playerRsp);
        _ = rest_db.write(&rest_client::get_player_stats_url(&crate::models::League::SHL, &game2.game_uuid), &playerRsp);

        ApiPlayerStatsService::update(&[game1, game2]);
        let player_db = ApiPlayerStatsService::get_player_career_db();
        let stored_players = player_db.read(&player_id);
        assert!(stored_players.is_some());
        
        let stored_player = stored_players.unwrap();
        assert_eq!(stored_player.len(), 1);
        assert_eq!(stored_player[0].player_id, player_id);
        assert_eq!(stored_player[0].team, team.to_string());

        let stats = match stored_player[0].stats {
            SeasonStats::Goalkeeper(a) => a,
            _ => panic!("not good"),
        };
        assert_eq!(stats.svs, 0);
        assert_eq!(stats.ga, 2);
        assert_eq!(stats.gp, 0);
    }

    pub fn get_played_game(game_uuid: &str, team: &str) -> ApiGame {
        ApiGame {
            game_uuid: game_uuid.to_string(),
            game_id: 123,
            home_team_code: team.to_string(),
            away_team_code: "FHC".to_string(),
            home_team_result: 3,
            away_team_result: 0,
            start_date_time: Utc::now(),
            status: crate::game_report_service::GameStatus::Finished,
            penalty_shots: false,
            overtime: false,
            played: true,
            game_type: crate::models::GameType::Season,
            league: crate::models::League::SHL,
            season: crate::models::Season::Season2022,
            gametime: None,
        }
    }

    pub fn get_player(player_id: i32, team: &str) -> (PlayerStats, PlayerName) {
        let player_stats = PlayerStats {
            info: crate::models2::external::player::PlayerInfo { playerId: player_id, teamId: team.to_string(), period: 0 },
            plus_minus: 1,
            A: 2,
            FOL: 1,
            FOPerc: 1.0,
            FOW: 1,
            G: 2,
            Hits: 1,
            NR: 1,
            PIM: 1,
            POS: crate::models::StringOrNum::String("FW".to_string()),
            PPG: 1,
            PPSOG: 1,
            SOG: 1,
            SW: 1,
            TOI: "13:37".to_string(),
        };
        let player_name = PlayerName{firstName: "olle".to_string(), lastName: "karlsson".to_string() };
        (player_stats, player_name)
    }

    pub fn get_goalkeeper(player_id: i32, team: &str) -> (GoalkeeperStats, PlayerName) {
        let player_stats = GoalkeeperStats { 
            info: crate::models2::external::player::PlayerInfo { playerId: player_id, teamId: team.to_string(), period: 0 },
            GA: 1,
            NR: 2,
            SOGA: 3,
            SPGA: 4,
            SVS: 5,
            SVS_perc: 5.0,
        };
        let player_name = PlayerName{firstName: "goalie".to_string(), lastName: "karlsson".to_string() };
        (player_stats, player_name)
    }

}