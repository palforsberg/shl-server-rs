use std::{time::Instant, sync::Arc, collections::HashMap};

use chrono::{Utc, Duration};
use tokio::sync::RwLock;
use tracing::log;

use crate::{models::{Season, SeasonKey}, game_report_service::GameReportService, db::Db, models_external::season::{SeasonRsp, SeasonGame}, models_api::{game::ApiGame, report::{GameStatus, ApiGameReport}, vote::VotePerGame}};

impl SeasonGame {
    pub fn is_potentially_live(&self) -> bool {
        let five_min_in_future = Utc::now() + Duration::minutes(5);
        self.state == "pre-game" && (self.startDateTime < five_min_in_future)
    }
}

pub struct ApiSeasonService {
    current_season_in_mem: Vec<ApiGame>,
    rest_games: HashMap<String, ApiGame>,
    pub db: Db<Season, Vec<ApiGame>>,
}
pub type SafeApiSeasonService = Arc<RwLock<ApiSeasonService>>;
impl ApiSeasonService {
    pub fn new() -> SafeApiSeasonService {
        Arc::new(RwLock::new(ApiSeasonService { 
            current_season_in_mem: vec!(),
            rest_games: HashMap::new(),
            db: Db::<Season, Vec<ApiGame>>::new("v2_season_decorated")
        }))
    }
    pub fn update(&mut self, 
        season: &Season, 
        responses: &[(SeasonKey, SeasonRsp)],
        votes_per_game: HashMap<String, VotePerGame>,
    ) -> Vec<ApiGame> {
        let before = Instant::now();
        let decorated_games: Vec<ApiGame> = responses.iter().flat_map(|(key, rsp)| rsp.gameInfo.iter().map(|e| {
            let base_status = match e.state.as_str() {
                "post-game" => GameStatus::Finished,
                _ => GameStatus::Coming,
            };
            let votes = votes_per_game.get(&e.uuid).copied();
            let mut mapped = ApiGame {
                game_uuid: e.uuid.clone(),
                home_team_code: e.homeTeamInfo.get_code(),
                away_team_code: e.awayTeamInfo.get_code(),
                home_team_result: e.homeTeamInfo.score.to_num(),
                away_team_result: e.awayTeamInfo.score.to_num(),
                start_date_time: e.startDateTime,
                played: GameStatus::Finished == base_status,
                shootout: e.shootout,
                overtime: e.overtime,
                status: base_status,
                season: key.0.clone(),
                league: key.1.clone(),
                game_type: key.2.clone(),
                gametime: None,
                votes: votes.map(|e| e.into()),
            };
            if e.is_potentially_live() {
                if let Some(report) = GameReportService::read(&e.uuid) {
                    mapped.status = report.status.clone();
                    mapped.played = report.status == GameStatus::Finished;
                    mapped.home_team_result = report.home_team_result;
                    mapped.away_team_result = report.away_team_result;
                    mapped.gametime = Some(report.gametime);
                    mapped.overtime = report.overtime.unwrap_or(mapped.overtime);
                    mapped.shootout = report.shootout.unwrap_or(mapped.shootout);
                }
            } 
            mapped
        }))
        .collect();

        log::info!("[API.SEASON] Decorated {season} {} games {:.2?}", decorated_games.len(), before.elapsed());
        _ = self.db.write(season, &decorated_games);
        if season.is_current() {
            self.current_season_in_mem = decorated_games.clone();
        } else {
            for ele in &decorated_games {
                self.rest_games.insert(ele.game_uuid.clone(), ele.clone());
            }
        }
        decorated_games
    }

    pub fn update_from_report(&mut self, report: &ApiGameReport) -> Option<ApiGame> {
        let mut result = None;
        if let Some(pos) = self.current_season_in_mem.iter_mut().find(|e| e.game_uuid == report.game_uuid) {
            pos.status = report.status.clone();
            pos.played = report.status == GameStatus::Finished;
            pos.home_team_result = report.home_team_result;
            pos.away_team_result = report.away_team_result;
            pos.overtime = report.overtime.unwrap_or(pos.overtime);
            pos.shootout = report.shootout.unwrap_or(pos.shootout);
            pos.gametime = Some(report.gametime.clone());
            result = Some(pos.clone());
            
            _ = self.db.write(&pos.season.clone(), &self.current_season_in_mem);
        }
        result
    }

    pub fn update_from_votes(&mut self, game_uuid: &str, votes: VotePerGame) {
        if let Some(pos) = self.current_season_in_mem.iter_mut().find(|e| e.game_uuid == game_uuid) {
            pos.votes = Some(votes.into());
            _ = self.db.write(&pos.season.clone(), &self.current_season_in_mem);
        }
    }

    pub fn read_current_season(&self) -> Vec<ApiGame> {
        self.current_season_in_mem.clone()
    }

    pub fn read_current_season_game(&self, game_uuid: &str) -> Option<ApiGame> {
        self.current_season_in_mem
            .iter()
            .find(|e| e.game_uuid == game_uuid)
            .cloned()
    }

    pub fn read_game(&self, game_uuid: &str) -> Option<ApiGame> {
        let current_season_game = self.read_current_season_game(game_uuid);
        if current_season_game.is_some() {
            current_season_game
        } else {
            self.rest_games.get(&game_uuid.to_string()).cloned()
        }
    }

    pub fn read_raw(season: &Season) -> String {
        let db: Db<Season, Vec<ApiGame>> = Db::new("v2_season_decorated");
        db.read_raw(season)
    }

    pub fn read(season: &Season) -> Vec<ApiGame> {
        let db: Db<Season, Vec<ApiGame>> = Db::new("v2_season_decorated");
        db.read(season).unwrap_or_default()
    }

    pub fn read_all() -> Vec<ApiGame> {
        let before = Instant::now();
        let db: Db<Season, Vec<ApiGame>> = Db::new("v2_season_decorated");
        let res: Vec<ApiGame> = db.read_all().iter().flat_map(|e| e.iter()).cloned().collect();
        log::info!("[API.SEASON] Read all {} {:.0?}", &res.len(), before.elapsed());
        res
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use chrono::{Utc, Duration};
    use tempdir::TempDir;

    use crate::{models::{Season, SeasonKey, StringOrNum::Number}, models_external::season::{SeasonRsp, SeasonGame, GameTeamInfo, SeriesInfo, TeamNames}, models_api::report::{GameStatus, ApiGameReport}, game_report_service::GameReportService};

    use super::ApiSeasonService;

    #[tokio::test]
    async fn test_report_is_used_pre() -> Result<(), ()> {
        std::env::set_var("DB_PATH", TempDir::new("test").expect("dir to be created").path().to_str().unwrap());
        let service = ApiSeasonService::new();

        let report = crate::models_api::report::ApiGameReport { game_uuid: "uuid".to_string(), gametime: "13:37".to_string(), status: GameStatus::Period1, home_team_code: "LHF".to_string(), away_team_code: "SAIK".to_string(), home_team_result: 1, away_team_result: 1, overtime: Some(true), shootout: Some(true) };
        GameReportService::store("uuid", &report);

        let season_key = SeasonKey(Season::Season2023, crate::models::League::SHL, crate::models::GameType::Season);
        let season_game = SeasonGame { 
            uuid: "uuid".to_string(), 
            awayTeamInfo: GameTeamInfo { code: "SAIK".to_string(), score: Number(0), names: TeamNames { code: "SAIK".to_string(), long: "SAIK".to_string(), short: "SAIK".to_string() } },
            homeTeamInfo: GameTeamInfo { code: "LHF".to_string(), score: Number(0), names: TeamNames { code: "LHF".to_string(), long: "LHF".to_string(), short: "LHF".to_string() } },
            startDateTime: Utc::now() + Duration::minutes(10), 
            state: "pre-game".to_string(), 
            shootout: false, 
            overtime: false, 
            seriesInfo: SeriesInfo { code: crate::models::League::SHL },
        };
        let result = service.write().await.update(&crate::models::Season::Season2023, &[(season_key, SeasonRsp { gameInfo: vec![season_game], teamList: vec![] })], HashMap::new());
        let api_game = result.get(0).unwrap();
        assert_eq!(api_game.status, GameStatus::Coming);
        assert_eq!(api_game.gametime, None);
        assert_eq!(api_game.home_team_result, 0);
        assert_eq!(api_game.away_team_result, 0);
        assert!(!api_game.overtime);
        assert!(!api_game.shootout);
        Ok(())
    }

    #[tokio::test]
    async fn test_report_is_used_live() -> Result<(), ()> {
        std::env::set_var("DB_PATH", TempDir::new("test").expect("dir to be created").path().to_str().unwrap());
        let service = ApiSeasonService::new();

        let report = crate::models_api::report::ApiGameReport { game_uuid: "uuid".to_string(), gametime: "13:37".to_string(), status: GameStatus::Period1, home_team_code: "LHF".to_string(), away_team_code: "SAIK".to_string(), home_team_result: 1, away_team_result: 1, overtime: Some(true), shootout: Some(true) };
        GameReportService::store("uuid", &report);

        let season_key = SeasonKey(Season::Season2023, crate::models::League::SHL, crate::models::GameType::Season);
        let season_game = SeasonGame { 
            uuid: "uuid".to_string(), 
            awayTeamInfo: GameTeamInfo { code: "SAIK".to_string(), score: Number(0), names: TeamNames { code: "SAIK".to_string(), long: "SAIK".to_string(), short: "SAIK".to_string() } },
            homeTeamInfo: GameTeamInfo { code: "LHF".to_string(), score: Number(0), names: TeamNames { code: "LHF".to_string(), long: "LHF".to_string(), short: "LHF".to_string() } },
            startDateTime: Utc::now() - Duration::minutes(10), 
            state: "pre-game".to_string(), 
            shootout: false, 
            overtime: false, 
            seriesInfo: SeriesInfo { code: crate::models::League::SHL },
        };
        let result = service.write().await.update(&crate::models::Season::Season2023, &[(season_key, SeasonRsp { gameInfo: vec![season_game], teamList: vec![] })], HashMap::new());
        let api_game = result.get(0).unwrap();
        assert_eq!(api_game.status, GameStatus::Period1);
        assert_eq!(api_game.gametime, Some("13:37".to_string()));
        assert_eq!(api_game.home_team_result, 1);
        assert_eq!(api_game.away_team_result, 1);
        assert!(api_game.overtime);
        assert!(api_game.shootout);
        Ok(())
    }


    #[tokio::test]
    async fn test_report_is_used_post() -> Result<(), ()> {
        std::env::set_var("DB_PATH", TempDir::new("test").expect("dir to be created").path().to_str().unwrap());
        let service = ApiSeasonService::new();

        let report = crate::models_api::report::ApiGameReport { game_uuid: "uuid".to_string(), gametime: "13:37".to_string(), status: GameStatus::Period1, home_team_code: "LHF".to_string(), away_team_code: "SAIK".to_string(), home_team_result: 1, away_team_result: 1, overtime: Some(true), shootout: Some(true) };
        GameReportService::store("uuid", &report);

        let season_key = SeasonKey(Season::Season2023, crate::models::League::SHL, crate::models::GameType::Season);
        let season_game = SeasonGame { 
            uuid: "uuid".to_string(), 
            awayTeamInfo: GameTeamInfo { code: "SAIK".to_string(), score: Number(0), names: TeamNames { code: "SAIK".to_string(), long: "SAIK".to_string(), short: "SAIK".to_string() } },
            homeTeamInfo: GameTeamInfo { code: "LHF".to_string(), score: Number(0), names: TeamNames { code: "LHF".to_string(), long: "LHF".to_string(), short: "LHF".to_string() } },
            startDateTime: Utc::now() - Duration::minutes(10), 
            state: "post-game".to_string(), 
            shootout: false, 
            overtime: false, 
            seriesInfo: SeriesInfo { code: crate::models::League::SHL },
        };
        let result = service.write().await.update(&crate::models::Season::Season2023, &[(season_key, SeasonRsp { gameInfo: vec![season_game], teamList: vec![] })], HashMap::new());
        let api_game = result.get(0).unwrap();
        assert_eq!(api_game.status, GameStatus::Finished);
        assert_eq!(api_game.gametime, None);
        assert_eq!(api_game.home_team_result, 0);
        assert_eq!(api_game.away_team_result, 0);
        assert!(!api_game.overtime);
        assert!(!api_game.shootout);
        Ok(())
    }

    #[tokio::test]
    async fn test_report_is_used_in_mem() -> Result<(), ()> {
        std::env::set_var("DB_PATH", TempDir::new("test").expect("dir to be created").path().to_str().unwrap());
        let service = ApiSeasonService::new();

        let report = crate::models_api::report::ApiGameReport { game_uuid: "uuid".to_string(), gametime: "13:37".to_string(), status: GameStatus::Period1, home_team_code: "LHF".to_string(), away_team_code: "SAIK".to_string(), home_team_result: 1, away_team_result: 1, overtime: Some(true), shootout: Some(true) };
        GameReportService::store("uuid", &report);

        let season_key = SeasonKey(Season::Season2023, crate::models::League::SHL, crate::models::GameType::Season);
        let season_game = SeasonGame { 
            uuid: "uuid".to_string(), 
            awayTeamInfo: GameTeamInfo { code: "SAIK".to_string(), score: Number(0), names: TeamNames { code: "SAIK".to_string(), long: "SAIK".to_string(), short: "SAIK".to_string() } },
            homeTeamInfo: GameTeamInfo { code: "LHF".to_string(), score: Number(0), names: TeamNames { code: "LHF".to_string(), long: "LHF".to_string(), short: "LHF".to_string() } },
            startDateTime: Utc::now() - Duration::minutes(10), 
            state: "post-game".to_string(), 
            shootout: false, 
            overtime: false, 
            seriesInfo: SeriesInfo { code: crate::models::League::SHL },
        };
        service.write().await.update(&crate::models::Season::Season2023, &[(season_key, SeasonRsp { gameInfo: vec![season_game], teamList: vec![] })], HashMap::new());

        service.write().await.update_from_report(&ApiGameReport { game_uuid: "uuid".to_string(), gametime: "13:37".to_string(), status: GameStatus::Period2, home_team_code: "SAIK".to_string(), away_team_code: "LHF".to_string(), home_team_result: 1, away_team_result: 1, overtime: None, shootout: None });
        let all_games = service.read().await.read_current_season();
        let updated = all_games.get(0).unwrap();
        assert_eq!(updated.status, GameStatus::Period2);
        Ok(())
    }
}
