use std::{time::Instant, sync::Arc, collections::HashMap};

use tokio::sync::RwLock;
use tracing::log;

use crate::{models::{Season, SeasonKey}, game_report_service::GameReportService, db::Db, models_external::season::SeasonRsp, models_api::{game::ApiGame, report::{GameStatus, ApiGameReport}}};


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
    pub fn update(&mut self, season: &Season, responses: &[(SeasonKey, SeasonRsp)]) -> Vec<ApiGame> {
        let before = Instant::now();
        let decorated_games: Vec<ApiGame> = responses.iter().flat_map(|(key, rsp)| rsp.gameInfo.iter().map(|e| {
            let base_status = match e.state.as_str() {
                "post-game" => GameStatus::Finished,
                _ => GameStatus::Coming,
            };
            let mut mapped = ApiGame {
                game_uuid: e.uuid.clone(),
                home_team_code: e.homeTeamInfo.code.clone(),
                away_team_code: e.awayTeamInfo.code.clone(),
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
            };
            if e.is_potentially_live() {
                if let Some(report) = GameReportService::read(&e.uuid) {
                    mapped.status = report.status.clone();
                    mapped.played = report.status == GameStatus::Finished;
                    mapped.home_team_result = report.home_team_result;
                    mapped.away_team_result = report.away_team_result;
                    mapped.gametime = Some(report.gametime);
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
            pos.gametime = Some(report.gametime.clone());
            result = Some(pos.clone());
            
            _ = self.db.write(&pos.season.clone(), &self.current_season_in_mem);
        }
        result
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
