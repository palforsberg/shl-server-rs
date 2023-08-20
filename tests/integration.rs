#![allow(non_snake_case)]
use chrono::Utc;
use common::models_apn::ApnBody;
use reqwest::StatusCode;
use serde::Deserialize;
use shl_server_rs::{models_api::{standings::Standings, game_details::ApiGameDetails, game::ApiGame, user::AddUser, report::GameStatus, live_activity::StartLiveActivity, vote::{VoteBody, VotePerGame, ApiVotePerGame}}, models_external::{event::{SseEvent, GameReport}, season::{SeasonGame, GameTeamInfo, SeriesInfo}}, models::{Season, StringOrNum, GameType}};
use std::{time::Instant, vec, fs::File, io::BufReader};
use tempdir::TempDir;
use std::io::BufRead;

use crate::common::{shl_server::ShlServer, external_server::ExternalServer};

mod common;

#[tokio::test]
async fn test_vote_service() -> Result<(), Box<dyn std::error::Error>> {
    // Given - Start external server
    let temp_dir = TempDir::new("integration_test").expect("dir to be created");
    let path = temp_dir.path().to_str().unwrap();

    let mut external_server = ExternalServer::new(8003);
    external_server.start().await;
    external_server.add_game(Season::get_current(), GameType::Season, SeasonGame { 
        uuid: "game_uuid_1".to_string(), 
        homeTeamInfo: GameTeamInfo { code: "SAIK".to_string(), score: StringOrNum::Number(0) }, 
        awayTeamInfo: GameTeamInfo { code: "OHK".to_string(), score: StringOrNum::Number(0) }, 
        startDateTime: Utc::now() - chrono::Duration::days(5),
        state: "pre-game".to_string(), 
        shootout: false,
        overtime: false, 
        seriesInfo: SeriesInfo { code: shl_server_rs::models::League::SHL },
    }).await;

    // Given - Start server
    let mut server = ShlServer::new(8004);
    server.start(path, &external_server.get_url());

    let req = AddUser { id: "user_id_1".to_string(), teams: vec!["SAIK".to_string()], apn_token: Some("apn_token_SAIK_1".to_string()), ios_version: None, app_version: None };
    server.retry_add_user(&req).await;

    // When - make request without api-key
    let req = &VoteBody { game_uuid: "game_uuid_1".to_string(), user_id: "user_id_1".to_string(), team_code: "SAIK".to_string() };
    let res = server.vote(req, None).await?;
    // Then - should be unauthorized
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // When - make request with invalid team
    let req = &VoteBody { game_uuid: "game_uuid_1".to_string(), user_id: "user_id_1".to_string(), team_code: "LHF".to_string() };
    let res = server.vote(req, Some("API_KEY")).await?;
    // Then - should be bad request
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // When - make request with invalid game uuid
    let req = &VoteBody { game_uuid: "INVALID".to_string(), user_id: "user_id_1".to_string(), team_code: "SAIK".to_string() };
    let res = server.vote(req, Some("API_KEY")).await?;
    // Then - should be bad request
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // When - add multiple votes
    for i in 0..=100 {
        let user_id = format!("user_id_SAIK_{i}");
        let req = AddUser { id: user_id.clone(), teams: vec!["SAIK".to_string()], apn_token: Some(format!("apn_token_SAIK_{i}")), ios_version: None, app_version: None };
        server.add_user(&req).await?;
        let req = &VoteBody { game_uuid: "game_uuid_1".to_string(), user_id, team_code: "SAIK".to_string() };
        let res = server.vote(req, Some("API_KEY")).await?;
        // Then - should be successful and response should be updated vote_per_game
        assert_eq!(res.status(), StatusCode::OK);
        let vote_per_game: ApiVotePerGame = res.json().await?;
        let expected: ApiVotePerGame = VotePerGame { home_count: i + 1, away_count: 0 }.into();
        assert_eq!(vote_per_game.home_perc, expected.home_perc);
        assert_eq!(vote_per_game.away_perc, expected.away_perc);
        assert_eq!(vote_per_game.home_perc + vote_per_game.away_perc, 100);
    }
    // When - add multiple votes
    for i in 0..=9 {
        let user_id = format!("user_id_OHK_{i}");
        let req = AddUser { id: user_id.clone(), teams: vec!["SAIK".to_string()], apn_token: Some(format!("apn_token_OHK_{i}")), ios_version: None, app_version: None };
        server.add_user(&req).await?;
        let req = &VoteBody { game_uuid: "game_uuid_1".to_string(), user_id, team_code: "OHK".to_string() };
        let res = server.vote(req, Some("API_KEY")).await?;
        // Then - should be successful and response should be updated vote_per_game
        assert_eq!(res.status(), StatusCode::OK);
        let vote_per_game: ApiVotePerGame = res.json().await?;
        let expected: ApiVotePerGame = VotePerGame { home_count: 101, away_count: i + 1 }.into();
        assert_eq!(vote_per_game.away_perc, expected.away_perc);
        assert_eq!(vote_per_game.home_perc, expected.home_perc);
        assert_eq!(vote_per_game.home_perc + vote_per_game.away_perc, 100);
    }

    let details = server.get_api_game_details("game_uuid_1").await?;
    assert_eq!(details.game.votes.unwrap().home_perc, 91);
    assert_eq!(details.game.votes.unwrap().away_perc, 9);

    let season_games = server.get_api_games(Season::Season2023).await?;
    let season_game = season_games.iter().find(|e| e.game_uuid == "game_uuid_1").unwrap();
    assert_eq!(details.game.votes.unwrap().home_perc, season_game.votes.unwrap().home_perc);
    assert_eq!(details.game.votes.unwrap().away_perc, season_game.votes.unwrap().away_perc);

    Ok(())
}
    
#[tokio::test]
async fn test_process_full_game() -> Result<(), Box<dyn std::error::Error>> {
    // Given - start servers with sse entries
    let temp_dir = TempDir::new("integration_test").expect("dir to be created");
    let path = temp_dir.path().to_str().unwrap();

    let sse_entries = parse_sse_log("./log/sse-real-2023-04-09.log");

    let mut external_server = ExternalServer::new(8001);
    external_server.start().await;
    external_server.add_game(Season::get_current(), GameType::Season, SeasonGame { 
        uuid: "qcv-34ekyLqu8".to_string(), 
        homeTeamInfo: GameTeamInfo { code: "SAIK".to_string(), score: StringOrNum::Number(0) }, 
        awayTeamInfo: GameTeamInfo { code: "OHK".to_string(), score: StringOrNum::Number(0) }, 
        startDateTime: Utc::now() - chrono::Duration::minutes(5),
        state: "pre-game".to_string(), 
        shootout: false,
        overtime: false, 
        seriesInfo: SeriesInfo { code: shl_server_rs::models::League::SHL },
    }).await;
    external_server.push_events(sse_entries).await;

    let external_api_state = external_server.api_state.clone();


    let mut server = ShlServer::new(8002);
    server.start(path, &external_server.get_url());

    // When - add users that should receive notifications
    for i in 0..100 {
        let req = AddUser { id: format!("user_id_SAIK_{i}"), teams: vec!["SAIK".to_string()], apn_token: Some(format!("apn_token_SAIK_{i}")), ios_version: None, app_version: None };
        server.retry_add_user(&req).await;
    }

    // When - add users that should not receive notifications
    for i in 0..100 {
        let req = AddUser { id: format!("user_id_LHF_{i}"), teams: vec!["LHF".to_string()], apn_token: Some(format!("apn_token_LHF_{i}")), ios_version: None, app_version: None };
        server.retry_add_user(&req).await;
    }

    // When - add users that should receive live activities
    for i in 0..10 {
        let req = AddUser { id: format!("user_id_SAIK_live_{i}"), teams: vec!["SAIK".to_string()], apn_token: Some(format!("apn_token_SAIK_live_{i}")), ios_version: None, app_version: None };
        server.retry_add_user(&req).await;
        let start_req = StartLiveActivity { user_id: req.id, token: req.apn_token.unwrap(), game_uuid: "qcv-34ekyLqu8".to_string() };
        _ = server.start_live_acitivty(&start_req).await;
    }

    // When - wait until all events have been processed
    let before = Instant::now();
    server.retry_until_game_reaches("qcv-34ekyLqu8", &GameStatus::Finished, 1000).await;
    println!("[TEST] Game finished in {:.2?}", before.elapsed()); // 20230723 - 14.30s 

    {
        // Then - all games should be available
        let games_rsp: Vec<ApiGame> = server.get_api_games(Season::Season2023).await?;
        assert_eq!(games_rsp.len(), 728 + 1);
    }

    {
        // Then - standings should be available
        let standings_rsp: Standings = server.get_api_standings(Season::Season2022).await?;
        assert_eq!(standings_rsp.SHL.len(), 14);
        assert_eq!(standings_rsp.SHL[0].team_code, "VLH");
        assert_eq!(standings_rsp.SHL[0].points, 102);
        assert_eq!(standings_rsp.HA.len(), 14);
        assert_eq!(standings_rsp.HA[0].team_code, "MODO");
        assert_eq!(standings_rsp.HA[0].points, 109);
    }

    {
        // Then - game details should be available
        let details: ApiGameDetails = server.get_api_game_details("qcv-34ekyLqu8").await?;
        assert_eq!(details.game.game_uuid, "qcv-34ekyLqu8");
        assert_eq!(details.stats.as_ref().unwrap().home.g, 3);
        assert_eq!(details.game.home_team_result, 3);
        assert_eq!(details.stats.as_ref().unwrap().away.g, 2);
        assert_eq!(details.game.away_team_result, 2);
        assert_eq!(details.events.len(), 289);
        assert_eq!(details.game.status, GameStatus::Finished);
        assert_eq!(details.game.gametime.unwrap(), "20:00".to_string());
    }
    {
        // Then - stats api should have been called once
        let safe_state = external_api_state.read().await;
        let stats_state = safe_state.stat_calls.get("qcv-34ekyLqu8").unwrap();
        assert_eq!(stats_state, &1);
    }

    let all_notifications = external_api_state.read().await.notifications.clone();
    assert_eq!(all_notifications.len(), 7 * 100);
    {
        // Then - all notifications should have been sent
        let pushed_notifications: Vec<(String, ApnBody)> = all_notifications.into_iter().filter(|e| e.0 == "apn_token_SAIK_0").collect();
        assert_eq!(pushed_notifications.len(), 7);

        let expected_titles = vec!["Nedsläpp", "Mål för OHK", "MÅÅÅL för SAIK! 🎉", "Mål för OHK", "MÅÅÅL för SAIK! 🎉", "MÅÅÅL för SAIK! 🎉", "SAIK vinner! 🥇"];
        for (i, (_, notification)) in pushed_notifications.iter().enumerate() {
            let expected_title = expected_titles.get(i).unwrap();
            let actual_title = notification.aps.alert.as_ref().map(|e| e.title.clone()).unwrap_or_default();
            assert_eq!(&actual_title, expected_title, "Notifications should be sent in correct order with correct title");
        }
    }

    let all_live_activity_updates = external_api_state.read().await.live_acitivies.clone();
    assert_eq!(all_live_activity_updates.len(), 10);
    {
        // Then - all live activity pushes should have been sent
        let updates = all_live_activity_updates.get("apn_token_SAIK_live_0").unwrap();
        assert_eq!(updates, &446);
        // TODO: assert for live activity
    }
    Ok(())
}

#[tokio::test]
async fn test_game_decoration() -> Result<(), Box<dyn std::error::Error>> {
    // Given - start servers with sse entries
    let temp_dir = TempDir::new("integration_test").expect("dir to be created");
    let path = temp_dir.path().to_str().unwrap();

    let mut external_server = ExternalServer::new(8005);
    external_server.start().await;
    external_server.add_game(Season::get_current(), GameType::Season, SeasonGame { 
        uuid: "game_uuid_1".to_string(), 
        homeTeamInfo: GameTeamInfo { code: "SAIK".to_string(), score: StringOrNum::Number(0) }, 
        awayTeamInfo: GameTeamInfo { code: "OHK".to_string(), score: StringOrNum::Number(0) }, 
        startDateTime: Utc::now() - chrono::Duration::minutes(5),
        state: "pre-game".to_string(), 
        shootout: false,
        overtime: false, 
        seriesInfo: SeriesInfo { code: shl_server_rs::models::League::SHL },
    }).await;

    let mut server = ShlServer::new(8006);
    server.start(path, &external_server.get_url());

    let req = AddUser { id: "user_id_SAIK_1".to_string(), teams: vec!["SAIK".to_string()], apn_token: Some("apn_token_SAIK_1".to_string()), ios_version: None, app_version: None };
    server.retry_add_user(&req).await;

    // When - without any report
    external_server.push_events(vec![SseEvent { gameReport: None, playByPlay: None }]).await;
    let game_uuid = "game_uuid_1";
    {
        // Then - game should be in status Coming
        let rsp = server.get_api_games(Season::Season2023).await?;
        let game: &ApiGame = rsp.iter().find(|e| e.game_uuid == game_uuid).unwrap();
        assert_eq!(game.status, GameStatus::Coming);
    }

    // When - with report with status Period1
    let mut report = GameReport { 
        gameUuid: game_uuid.to_string(),
        gameTime: "13:37".to_string(),
        statusString: "not used".to_string(),
        gameState: "Ongoing".to_string(),
        period: StringOrNum::String("1".to_string()),
        homeTeamId: Some("SAIK".to_string()),
        awayTeamId: Some("OHK".to_string()),
        homeTeamScore: StringOrNum::Number(2),
        awayTeamScore: StringOrNum::Number(1),
        revision: 1,
    };
    external_server.push_events(vec![SseEvent { gameReport: Some(report.clone()), playByPlay: None }]).await;
    {
        // Then - game should be in status Period1
        let details = server.retry_until_game_reaches(game_uuid, &GameStatus::Period1, 1000).await;
        assert_eq!(details.game.gametime.unwrap(), "13:37");
        assert!(!details.game.played);
        let rsp = server.get_api_games(Season::Season2023).await?;
        let game: &ApiGame = rsp.iter().find(|e| e.game_uuid == game_uuid).unwrap();
        assert_eq!(game.status, GameStatus::Period1);
        assert_eq!(game.home_team_result, 2);
        assert_eq!(game.away_team_result, 1);
        assert_eq!(game.gametime.as_ref().unwrap(), "13:37");
    }

    // When - with report with status Intermission
    report.gameState = "Intermission".to_string();
    report.gameTime = "20:00".to_string();
    report.revision += 1;
    external_server.push_events(vec![SseEvent { gameReport: Some(report.clone()), playByPlay: None }]).await;

    {
        // Then - game should be in status Intermission
        let details = server.retry_until_game_reaches(game_uuid, &GameStatus::Intermission, 1000).await;
        assert_eq!(details.game.gametime.unwrap(), "20:00");
        assert!(!details.game.played);
        let rsp = server.get_api_games(Season::Season2023).await?;
        let game: &ApiGame = rsp.iter().find(|e| e.game_uuid == game_uuid).unwrap();
        assert_eq!(game.status, GameStatus::Intermission);
    }

    // When - with report with status Finished
    report.gameState = "GameEnded".to_string();
    report.revision += 1;
    external_server.push_events(vec![SseEvent { gameReport: Some(report.clone()), playByPlay: None }]).await;
    {
        // Then - game should be in status Finished
        let details = server.retry_until_game_reaches(game_uuid, &GameStatus::Finished, 1000).await;
        assert!(details.game.played);
        let rsp = server.get_api_games(Season::Season2023).await?;
        let game: &ApiGame = rsp.iter().find(|e| e.game_uuid == game_uuid).unwrap();
        assert_eq!(game.status, GameStatus::Finished);
        assert!(game.played);
    }

    Ok(())
}

#[tokio::test]
async fn test_multiple_live_games() -> Result<(), Box<dyn std::error::Error>> {
    // Given - start servers with sse entries
    let temp_dir = TempDir::new("integration_test").expect("dir to be created");
    let path = temp_dir.path().to_str().unwrap();

    let mut external_server = ExternalServer::new(8007);
    external_server.start().await;

    let mut server = ShlServer::new(8008);
    server.start(path, &external_server.get_url());
    external_server.add_game(Season::get_current(), GameType::Season, SeasonGame { 
        uuid: "game_uuid_1".to_string(), 
        homeTeamInfo: GameTeamInfo { code: "SAIK".to_string(), score: StringOrNum::Number(0) }, 
        awayTeamInfo: GameTeamInfo { code: "OHK".to_string(), score: StringOrNum::Number(0) }, 
        startDateTime: Utc::now() - chrono::Duration::minutes(5),
        state: "pre-game".to_string(), 
        shootout: false,
        overtime: false, 
        seriesInfo: SeriesInfo { code: shl_server_rs::models::League::SHL },
    }).await;
    external_server.add_game(Season::get_current(), GameType::Season, SeasonGame { 
        uuid: "game_uuid_2".to_string(), 
        homeTeamInfo: GameTeamInfo { code: "LHF".to_string(), score: StringOrNum::Number(0) }, 
        awayTeamInfo: GameTeamInfo { code: "MODO".to_string(), score: StringOrNum::Number(0) }, 
        startDateTime: Utc::now() - chrono::Duration::minutes(5),
        state: "pre-game".to_string(), 
        shootout: false,
        overtime: false, 
        seriesInfo: SeriesInfo { code: shl_server_rs::models::League::SHL },
    }).await;

    external_server.add_game(Season::get_current(), GameType::Season, SeasonGame { 
        uuid: "game_uuid_3".to_string(), 
        homeTeamInfo: GameTeamInfo { code: "TIK".to_string(), score: StringOrNum::Number(0) }, 
        awayTeamInfo: GameTeamInfo { code: "FBK".to_string(), score: StringOrNum::Number(0) }, 
        startDateTime: Utc::now() - chrono::Duration::minutes(5),
        state: "pre-game".to_string(), 
        shootout: false,
        overtime: false, 
        seriesInfo: SeriesInfo { code: shl_server_rs::models::League::SHL },
    }).await;

    // When - add users that should receive notifications
    for i in 0..10 {
        let req = AddUser { id: format!("user_id_SAIK_{i}"), teams: vec!["SAIK".to_string()], apn_token: Some(format!("apn_token_SAIK_{i}")), ios_version: None, app_version: None };
        server.retry_add_user(&req).await;
    }

    // When - add users that should receive notifications
    for i in 0..10 {
        let req = AddUser { id: format!("user_id_LHF_{i}"), teams: vec!["LHF".to_string()], apn_token: Some(format!("apn_token_LHF_{i}")), ios_version: None, app_version: None };
        server.retry_add_user(&req).await;
    }

    // When - new report for game with 10 subscribers
    external_server.push_events(vec![SseEvent { playByPlay: None, gameReport: Some(GameReport { 
        gameUuid: "game_uuid_1".to_string(),
        gameTime: "00:00".to_string(),
        statusString: "Ongoing".to_string(),
        gameState: "Ongoing".to_string(),
        period: StringOrNum::Number(1),
        homeTeamId: Some("SAIK".to_string()),
        awayTeamId: Some("OHK".to_string()),
        homeTeamScore: StringOrNum::Number(0),
        awayTeamScore: StringOrNum::Number(0),
        revision: 1,
    })}]).await;

    server.retry_until_game_reaches("game_uuid_1", &GameStatus::Period1, 1000).await;

    // Then - 10 more notifications should be pushed
    let all_notifications = external_server.api_state.read().await.notifications.clone();
    assert_eq!(all_notifications.len(), 10);

    // When - new report for game with 10 subscribers
    external_server.push_events(vec![SseEvent { playByPlay: None, gameReport: Some(GameReport { 
        gameUuid: "game_uuid_2".to_string(),
        gameTime: "00:00".to_string(),
        statusString: "Ongoing".to_string(),
        gameState: "Ongoing".to_string(),
        period: StringOrNum::Number(1),
        homeTeamId: Some("LHF".to_string()),
        awayTeamId: Some("MODO".to_string()),
        homeTeamScore: StringOrNum::Number(0),
        awayTeamScore: StringOrNum::Number(0),
        revision: 1,
    })}]).await;

    server.retry_until_game_reaches("game_uuid_2", &GameStatus::Period1, 1000).await;

    // Then - 10 more notifications should be pushed
    let all_notifications = external_server.api_state.read().await.notifications.clone();
    assert_eq!(all_notifications.len(), 20);

    // When - new report for game with no subscribers
    external_server.push_events(vec![SseEvent { playByPlay: None, gameReport: Some(GameReport { 
        gameUuid: "game_uuid_3".to_string(),
        gameTime: "00:00".to_string(),
        statusString: "Ongoing".to_string(),
        gameState: "Ongoing".to_string(),
        period: StringOrNum::Number(1),
        homeTeamId: Some("TIK".to_string()),
        awayTeamId: Some("FBK".to_string()),
        homeTeamScore: StringOrNum::Number(0),
        awayTeamScore: StringOrNum::Number(0),
        revision: 1,
    })}]).await;

    server.retry_until_game_reaches("game_uuid_3", &GameStatus::Period1, 1000).await;

    // Then - no more notifications should be pushed
    let all_notifications = external_server.api_state.read().await.notifications.clone();
    assert_eq!(all_notifications.len(), 20);

    Ok(())
}

fn parse_sse_log(path: &str) -> Vec<SseEvent> {
    #[derive(Deserialize)]
    struct LogEntry { data: String, }
    BufReader::new(File::open(path).expect("no such file")).lines()
        .map(|l| l.expect("Could not parse line"))
        .map(|l| serde_json::from_str::<LogEntry>(&l).expect("Could not decode json"))
        .map(|l| serde_json::from_str(&l.data).expect("Could not decode json data"))
        .collect()
}

