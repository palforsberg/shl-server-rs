#![allow(non_snake_case)]
use chrono::Utc;
use common::models_apn::ApnBody;
use reqwest::StatusCode;
use serde::Deserialize;
use shl_server_rs::{models_api::{standings::Standings, game_details::ApiGameDetails, game::ApiGame, user::AddUser, report::GameStatus, live_activity::StartLiveActivity, vote::{VoteBody, VotePerGame, ApiVotePerGame}}, models_external::{event::{SseEvent, GameReport, LiveEvent, PeriodType, EventType, ShotType, LiveEventTeam, EventTeam, EventPlayer, SseGameTime, LiveState, LiveStateEvent}, season::{SeasonGame, GameTeamInfo, SeriesInfo, TeamNames}}, models::{Season, StringOrNum, GameType}};
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
    external_server.add_game(Season::Season2023, GameType::Season, SeasonGame { 
        uuid: "game_uuid_1".to_string(), 
        homeTeamInfo: get_team_info("SAIK", 0), 
        awayTeamInfo: get_team_info("OHK", 0), 
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
    
#[ignore]
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
        homeTeamInfo: get_team_info("SAIK", 0), 
        awayTeamInfo: get_team_info("OHK", 0), 
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
        assert_eq!(games_rsp.len(), 1);
    }

    {
        // Then - standings should be available
        let standings_rsp: Standings = server.get_api_standings(Season::Season2023).await?;
        assert_eq!(standings_rsp.SHL.len(), 2);
        assert_eq!(standings_rsp.SHL[0].team_code, "OHK");
        assert_eq!(standings_rsp.SHL[0].points, 0);
        assert_eq!(standings_rsp.SHL[1].team_code, "SAIK");
        assert_eq!(standings_rsp.SHL[1].points, 0);
    }

    {
        // Then - game details should be available
        let details: ApiGameDetails = server.get_api_game_details("qcv-34ekyLqu8").await?;
        assert_eq!(details.game.game_uuid, "qcv-34ekyLqu8");
        assert_eq!(details.stats.as_ref().unwrap().home.g, 3);
        assert_eq!(details.game.home_team_result, 3);
        assert_eq!(details.stats.as_ref().unwrap().away.g, 2);
        assert_eq!(details.game.away_team_result, 2);
        assert_eq!(details.events.len(), 0);
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
        assert_notifications(expected_titles, pushed_notifications);
    }

    let all_live_activity_updates = external_api_state.read().await.nr_live_activities.clone();
    assert_eq!(all_live_activity_updates.len(), 10);
    {
        // Then - all live activity pushes should have been sent
        let updates = all_live_activity_updates.get("apn_token_SAIK_live_0").unwrap();
        assert_eq!(updates, &444);
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
        homeTeamInfo: get_team_info("SAIK", 0), 
        awayTeamInfo: get_team_info("OHK", 0), 
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
    external_server.push_events(vec![SseEvent { ..Default::default()}]).await;
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
    external_server.push_events(vec![SseEvent { gameReport: Some(report.clone()), ..Default::default()}]).await;
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
    external_server.push_events(vec![SseEvent { gameReport: Some(report.clone()), ..Default::default() }]).await;

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
    external_server.push_events(vec![SseEvent { gameReport: Some(report.clone()), ..Default::default() }]).await;
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
        homeTeamInfo: get_team_info("SAIK", 0), 
        awayTeamInfo: get_team_info("OHK", 0), 
        startDateTime: Utc::now() - chrono::Duration::minutes(5),
        state: "pre-game".to_string(), 
        shootout: false,
        overtime: false, 
        seriesInfo: SeriesInfo { code: shl_server_rs::models::League::SHL },
    }).await;
    external_server.add_game(Season::get_current(), GameType::Season, SeasonGame { 
        uuid: "game_uuid_2".to_string(), 
        homeTeamInfo: get_team_info("LHF", 0), 
        awayTeamInfo: get_team_info("MODO", 0), 
        startDateTime: Utc::now() - chrono::Duration::minutes(5),
        state: "pre-game".to_string(), 
        shootout: false,
        overtime: false, 
        seriesInfo: SeriesInfo { code: shl_server_rs::models::League::SHL },
    }).await;

    external_server.add_game(Season::get_current(), GameType::Season, SeasonGame { 
        uuid: "game_uuid_3".to_string(), 
        homeTeamInfo: get_team_info("TIK", 0), 
        awayTeamInfo: get_team_info("FBK", 0), 
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
    external_server.push_events(vec![SseEvent { gameReport: Some(GameReport { 
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
    }), ..Default::default()}]).await;

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
    }), ..Default::default()}]).await;

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
    }), ..Default::default()}]).await;

    server.retry_until_game_reaches("game_uuid_3", &GameStatus::Period1, 1000).await;

    // Then - no more notifications should be pushed
    let all_notifications = external_server.api_state.read().await.notifications.clone();
    assert_eq!(all_notifications.len(), 20);

    Ok(())
}

#[tokio::test]
async fn test_live_events() -> Result<(), Box<dyn std::error::Error>> {
    // Given - start servers with sse entries
    let temp_dir = TempDir::new("integration_test").expect("dir to be created");
    let path = temp_dir.path().to_str().unwrap();

    let mut external_server = ExternalServer::new(8009);
    external_server.start().await;

    let mut server = ShlServer::new(8010);
    server.start(path, &external_server.get_url());
    external_server.add_game(Season::get_current(), GameType::Season, SeasonGame { 
        uuid: "game_uuid_1".to_string(), 
        homeTeamInfo: get_team_info("SAIK", 0), 
        awayTeamInfo: get_team_info("OHK", 0), 
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

    for i in 0..10 {
        let req = AddUser { id: format!("user_id_SAIK_live_{i}"), teams: vec!["SAIK".to_string()], apn_token: Some(format!("apn_token_SAIK_{i}")), ios_version: None, app_version: None };
        server.retry_add_user(&req).await;
        let start_req = StartLiveActivity { user_id: req.id, token: req.apn_token.unwrap(), game_uuid: "game_uuid_1".to_string() };
        _ = server.start_live_acitivty(&start_req).await;
    }

    // When - new report for game with 10 subscribers
    external_server.push_events(vec![SseEvent { liveEvent: Some(LiveEvent { 
        gameUuid: "game_uuid_1".to_string(),
        eventId: Some(StringOrNum::Number(1)),
        period: StringOrNum::Number(1),
        eventType: Some(EventType::Period( PeriodType { started: true, finished: false }))
    }), ..Default::default()}]).await;

    let details = server.retry_until_game_reaches("game_uuid_1", &GameStatus::Period1, 1000).await;

    // Then - 10 more notifications should be pushed
    let all_notifications = external_server.api_state.read().await.notifications.clone();
    assert_eq!(all_notifications.len(), 10);

    let not = all_notifications.get(0).unwrap();
    assert_eq!(not.1.aps.alert.as_ref().unwrap().title, "Nedsläpp");

    let all_live = external_server.api_state.read().await.nr_live_activities["apn_token_SAIK_0"];
    assert_eq!(all_live, 2); // RSM event, Live Event
    
    assert_eq!(details.events.len(), 1);

    // When - new report for game with 10 subscribers
    external_server.push_events(vec![SseEvent { liveEvent: Some(LiveEvent { 
        gameUuid: "game_uuid_1".to_string(),
        eventId: Some(StringOrNum::Number(2)),
        period: StringOrNum::Number(1),
        eventType: Some(EventType::Shot( ShotType { 
            time: "13:37".to_string(),
            gameState: "Ongoing".to_string(),
            goalStatus: None,
            homeTeam: LiveEventTeam { teamId: "SAIK".to_string(), score: StringOrNum::Number(1) },
            awayTeam: LiveEventTeam { teamId: "MODO".to_string(), score: StringOrNum::Number(2) },
            eventTeam: EventTeam { teamId: "SAIK".to_string() },
            revision: 1,
            player: EventPlayer { playerId: StringOrNum::Number(1337), familyName: "Olle".to_string(), firstName: "Karlsson".to_string(), jerseyToday: StringOrNum::Number(33) } 
        }))
    }), ..Default::default()}]).await;

    // Then game should be updated - score shouldn't be affected
    let predicate = predicates::function::function(|e: &ApiGameDetails| e.game.gametime == Some("13:37".to_string()));
    let details = server.retry_until("game_uuid_1", predicate, 1000).await;
    assert_eq!(details.game.gametime, Some("13:37".to_string()));
    assert_eq!(details.game.home_team_result, 0);
    assert_eq!(details.game.away_team_result, 0);

    // Then - 0 more notifications should be pushed
    let all_notifications = external_server.api_state.read().await.notifications.clone();
    assert_eq!(all_notifications.len(), 10);
    
    let all_live = external_server.api_state.read().await.nr_live_activities["apn_token_SAIK_0"];
    assert_eq!(all_live, 3);
    
    assert_eq!(details.events.len(), 2);

    // // When - new team stats for game with 10 subscribers
    // external_server.push_events(vec![SseEvent { teamStatistics: Some(TeamStatistics { 
    //     gameUuid: "game_uuid_1".to_string(),
    //     teamId: "SAIK".to_string(), 
    //     statistics: vec![PeriodStats { period: StringOrNum::Number(0), parsedTotalStatistics: vec![
    //         StatsValue { caption: "G".to_string(), value: Some(StringOrNum::Number(5)) },
    //         StatsValue { caption: "GA".to_string(), value: Some(StringOrNum::Number(3)) },
    //     ]}]
    // }), ..Default::default()}]).await;

    // // Then game should be updated
    // let predicate = predicates::function::function(|e: &ApiGameDetails| e.game.home_team_result == 5);
    // let details = server.retry_until("game_uuid_1", predicate, 1000).await;
    // assert_eq!(details.game.gametime, Some("13:37".to_string()));
    // assert_eq!(details.game.home_team_result, 5);
    // assert_eq!(details.game.away_team_result, 3);

    // // Then - 0 more notifications should be pushed
    // let all_notifications = external_server.api_state.read().await.notifications.clone();
    // assert_eq!(all_notifications.len(), 10);
    
    // let all_live = external_server.api_state.read().await.nr_live_activities["apn_token_SAIK_0"];
    // assert_eq!(all_live, 4);

    // When - new live event for game with 10 subscribers
    external_server.push_events(vec![SseEvent { liveEvent: Some(LiveEvent { 
        gameUuid: "game_uuid_1".to_string(),
        eventId: Some(StringOrNum::Number(3)),
        period: StringOrNum::Number(1),
        eventType: Some(EventType::Goal( ShotType { 
            time: "13:37".to_string(),
            gameState: "Ongoing".to_string(),
            goalStatus: Some("EQ".to_string()),
            homeTeam: LiveEventTeam { teamId: "SAIK".to_string(), score: StringOrNum::Number(6) },
            awayTeam: LiveEventTeam { teamId: "MODO".to_string(), score: StringOrNum::Number(2) },
            eventTeam: EventTeam { teamId: "SAIK".to_string() },
            revision: 1,
            player: EventPlayer { playerId: StringOrNum::Number(1337), familyName: "Olle".to_string(), firstName: "Karlsson".to_string(), jerseyToday: StringOrNum::Number(33) } 
        }))
    }), ..Default::default()}]).await;

    // Then game should be updated
    let predicate = predicates::function::function(|e: &ApiGameDetails| e.game.home_team_result == 6);
    let details = server.retry_until("game_uuid_1", predicate, 1000).await;
    assert_eq!(details.game.gametime, Some("13:37".to_string()));
    assert_eq!(details.game.home_team_result, 6);
    assert_eq!(details.game.away_team_result, 2);

    // Then - 10 more notifications should be pushed
    let all_notifications = external_server.api_state.read().await.notifications.clone();
    assert_eq!(all_notifications.len(), 20);

    let all_live = external_server.api_state.read().await.nr_live_activities["apn_token_SAIK_0"];
    assert_eq!(all_live, 5);

    assert_eq!(details.events.len(), 3);

    Ok(())
}

#[tokio::test]
async fn test_live_events_report() -> Result<(), Box<dyn std::error::Error>> {
    // Given - start servers with sse entries
    let temp_dir = TempDir::new("integration_test").expect("dir to be created");
    let path = temp_dir.path().to_str().unwrap();

    let mut external_server = ExternalServer::new(8011);
    external_server.api_state.write().await.store_live_activities = false;
    external_server.start().await;

    let mut server = ShlServer::new(8012);
    server.start(path, &external_server.get_url());
    external_server.add_game(Season::get_current(), GameType::Season, SeasonGame { 
        uuid: "game_uuid_1".to_string(), 
        homeTeamInfo: get_team_info("SAIK", 0), 
        awayTeamInfo: get_team_info("OHK", 0), 
        startDateTime: Utc::now() - chrono::Duration::minutes(5),
        state: "pre-game".to_string(), 
        shootout: false,
        overtime: false, 
        seriesInfo: SeriesInfo { code: shl_server_rs::models::League::SHL },
    }).await;

    // When - add users that should receive notifications
    for i in 0..1 {
        let req = AddUser { id: format!("user_id_SAIK_live_{i}"), teams: vec!["SAIK".to_string()], apn_token: Some(format!("apn_token_SAIK_{i}")), ios_version: None, app_version: None };
        server.retry_add_user(&req).await;
        let start_req = StartLiveActivity { user_id: req.id, token: req.apn_token.unwrap(), game_uuid: "game_uuid_1".to_string() };
        _ = server.start_live_acitivty(&start_req).await;
    }

    // When - new live event for game
    external_server.push_events(vec![SseEvent { liveEvent: Some(LiveEvent { 
        gameUuid: "game_uuid_1".to_string(),
        eventId: Some(StringOrNum::Number(1)),
        period: StringOrNum::Number(1),
        eventType: Some(EventType::Period( PeriodType { started: true, finished: false }))
    }), ..Default::default()}]).await;

    server.retry_until_game_reaches("game_uuid_1", &GameStatus::Period1, 1000).await;

    let all_live = external_server.api_state.read().await.nr_live_activities["apn_token_SAIK_0"];
    assert_eq!(all_live, 2);

    // When - new gametime for game
    external_server.push_events(vec![SseEvent { gameTime: Some(SseGameTime { 
        gameUuid: "game_uuid_1".to_string(),
        period: Some(StringOrNum::Number(1)),
        periodTime: Some("00:27".to_string()), 
    }), ..Default::default()}]).await;

    // Then - game is updated + new live actvity
    let predicate = predicates::function::function(|e: &ApiGameDetails| e.game.gametime == Some("00:27".to_string()));
    server.retry_until("game_uuid_1", predicate, 500).await;

    let all_live = external_server.api_state.read().await.nr_live_activities["apn_token_SAIK_0"];
    assert_eq!(all_live, 3);

    // When - new gametime going back in time
    external_server.push_events(vec![SseEvent { gameTime: Some(SseGameTime { 
        gameUuid: "game_uuid_1".to_string(),
        period: Some(StringOrNum::Number(1)),
        periodTime: Some("00:26".to_string()), 
    }), ..Default::default()}]).await;

    // Then - no new live activities
    let all_live = external_server.api_state.read().await.nr_live_activities["apn_token_SAIK_0"];
    assert_eq!(all_live, 3);

    // When - new live event for game
    external_server.push_events(vec![SseEvent { liveEvent: Some(LiveEvent { 
        gameUuid: "game_uuid_1".to_string(),
        eventId: Some(StringOrNum::Number(1)),
        period: StringOrNum::Number(1),
        eventType: Some(EventType::Goal( ShotType { 
            time: "13:37".to_string(),
            gameState: "Ongoing".to_string(),
            goalStatus: Some("EQ".to_string()),
            homeTeam: LiveEventTeam { teamId: "SAIK".to_string(), score: StringOrNum::Number(6) },
            awayTeam: LiveEventTeam { teamId: "MODO".to_string(), score: StringOrNum::Number(2) },
            eventTeam: EventTeam { teamId: "SAIK".to_string() },
            revision: 1,
            player: EventPlayer { playerId: StringOrNum::Number(1337), familyName: "Olle".to_string(), firstName: "Karlsson".to_string(), jerseyToday: StringOrNum::Number(33) } 
        }))
    }), ..Default::default()}]).await;

    // Then - game is updated + new live actvity
    let predicate = predicates::function::function(|e: &ApiGameDetails| e.game.home_team_result == 6);
    server.retry_until("game_uuid_1", predicate, 500).await;

    let all_live = external_server.api_state.read().await.nr_live_activities["apn_token_SAIK_0"];
    assert_eq!(all_live, 6);

    // When - old live event for game
    external_server.push_events(vec![SseEvent { liveEvent: Some(LiveEvent { 
        gameUuid: "game_uuid_1".to_string(),
        eventId: Some(StringOrNum::Number(1)),
        period: StringOrNum::Number(1),
        eventType: Some(EventType::Goal( ShotType { 
            time: "13:37".to_string(),
            gameState: "Ongoing".to_string(),
            goalStatus: Some("EQ".to_string()),
            homeTeam: LiveEventTeam { teamId: "SAIK".to_string(), score: StringOrNum::Number(6) },
            awayTeam: LiveEventTeam { teamId: "MODO".to_string(), score: StringOrNum::Number(2) },
            eventTeam: EventTeam { teamId: "SAIK".to_string() },
            revision: 2,
            player: EventPlayer { playerId: StringOrNum::Number(1337), familyName: "Olle".to_string(), firstName: "Karlsson".to_string(), jerseyToday: StringOrNum::Number(33) } 
        }))
    }), ..Default::default()}]).await;
    
    // Then - no new live activities
    let all_live = external_server.api_state.read().await.nr_live_activities["apn_token_SAIK_0"];
    assert_eq!(all_live, 6);

    // When - new period live event for game
    external_server.push_events(vec![SseEvent { liveEvent: Some(LiveEvent { 
        gameUuid: "game_uuid_1".to_string(),
        eventId: Some(StringOrNum::Number(33)),
        period: StringOrNum::Number(2),
        eventType: Some(EventType::Period( PeriodType { started: true, finished: false }))
    }), ..Default::default()}]).await;

    // Then - new live activities
    let predicate = predicates::function::function(|e: &ApiGameDetails| e.game.status == GameStatus::Period2);
    server.retry_until("game_uuid_1", predicate, 500).await;

    let all_live = external_server.api_state.read().await.nr_live_activities["apn_token_SAIK_0"];
    assert_eq!(all_live, 8);

    // When - new period live event for game
    external_server.push_events(vec![SseEvent { liveEvent: Some(LiveEvent { 
        gameUuid: "game_uuid_1".to_string(),
        eventId: Some(StringOrNum::Number(34)),
        period: StringOrNum::Number(3),
        eventType: Some(EventType::Period( PeriodType { started: true, finished: false }))
    }), ..Default::default()}]).await;

    // Then - new live activities
    let predicate = predicates::function::function(|e: &ApiGameDetails| e.game.status == GameStatus::Period3);
    server.retry_until("game_uuid_1", predicate, 500).await;

    let all_live = external_server.api_state.read().await.nr_live_activities["apn_token_SAIK_0"];
    assert_eq!(all_live, 10);

    // When - overtime live event for game
    external_server.push_events(vec![SseEvent { liveEvent: Some(LiveEvent { 
        gameUuid: "game_uuid_1".to_string(),
        eventId: Some(StringOrNum::Number(34)),
        period: StringOrNum::Number(4),
        eventType: Some(EventType::Period( PeriodType { started: true, finished: false }))
    }), ..Default::default()}]).await;

    // Then - new live activities
    let predicate = predicates::function::function(|e: &ApiGameDetails| e.game.status == GameStatus::Overtime);
    let game_details = server.retry_until("game_uuid_1", predicate, 500).await;

    assert!(game_details.game.overtime);
    assert!(!game_details.game.shootout);

    let all_live = external_server.api_state.read().await.nr_live_activities["apn_token_SAIK_0"];
    assert_eq!(all_live, 12);

    // When - live state for game
    external_server.push_events(vec![SseEvent { liveState: Some(LiveStateEvent {
        gameUuid: "game_uuid_1".to_string(),
        liveState: LiveState::Decided,
        previousLiveState: LiveState::Ongoing,
    }), ..Default::default()}]).await;

    // Then - game is updated + new live actvity
    let predicate = predicates::function::function(|e: &ApiGameDetails| e.game.status == GameStatus::Finished);
    let game_details = server.retry_until("game_uuid_1", predicate, 500).await;

    assert!(game_details.game.overtime);
    assert!(!game_details.game.shootout);

    let all_live = external_server.api_state.read().await.nr_live_activities["apn_token_SAIK_0"];
    assert_eq!(all_live, 13);

    Ok(())
}

#[tokio::test]
async fn test_vlh_mif_2023_09_16() -> Result<(), Box<dyn std::error::Error>> {
    // Given - start servers with sse entries
    let temp_dir = TempDir::new("integration_test").expect("dir to be created");
    let path = temp_dir.path().to_str().unwrap();

    let game_uuid = "qcz-3SCYzRBN3".to_string();
    let events = parse_sse_live_events(&format!("./log/sse-live-events/{game_uuid}.log"));

    let mut external_server = ExternalServer::new(8013);
    external_server.start().await;
    external_server.api_state.write().await.store_live_activities = true;

    let mut server = ShlServer::new(8014);
    server.start(path, &external_server.get_url());
    external_server.add_game(Season::get_current(), GameType::Season, SeasonGame { 
        uuid: game_uuid.clone(), 
        homeTeamInfo: get_team_info("VLH", 0), 
        awayTeamInfo: get_team_info("MIF", 0), 
        startDateTime: Utc::now() - chrono::Duration::minutes(5),
        state: "pre-game".to_string(), 
        shootout: false,
        overtime: false, 
        seriesInfo: SeriesInfo { code: shl_server_rs::models::League::SHL },
    }).await;

    // notification user
    let req = AddUser { id: "user_id_SAIK_live_0".to_string(), teams: vec!["VLH".to_string()], apn_token: Some("apn_token_SAIK_0".to_string()), ios_version: None, app_version: None };
    server.retry_add_user(&req).await;

    // live activity user
    let req = AddUser { id: "user_id_SAIK_live_1".to_string(), teams: vec!["VLH".to_string()], apn_token: Some("apn_token_SAIK_1".to_string()), ios_version: None, app_version: None };
    server.retry_add_user(&req).await;
    let start_req = StartLiveActivity { user_id: req.id, token: req.apn_token.unwrap(), game_uuid: game_uuid.clone() };
    _ = server.start_live_acitivty(&start_req).await;

    // When - processing the events
    external_server.push_events(events).await;

    // Then - game should be finished, all events should be sent
    let game_details = server.retry_until_game_reaches(&game_uuid, &GameStatus::Finished, 500).await;

    assert_eq!(game_details.events.len(), 98);
    assert!(!game_details.game.overtime);
    assert!(!game_details.game.shootout);

    let apn_state = external_server.api_state.read().await;
    assert_eq!(apn_state.notifications.len(), 10);

    assert_eq!(apn_state.nr_live_activities.get("apn_token_SAIK_1").unwrap(), &172);

    // TODO: Fix notifications - incorrect data. Need correction to fix.
    let expected_titles = vec!["Nedsläpp", "Mål för MIF", "MÅÅÅL för VLH! 🎉", "Mål för MIF", "MÅÅÅL för VLH! 🎉", "Mål för MIF", "MÅÅÅL för VLH! 🎉", "MÅÅÅL för VLH! 🎉", "Mål för MIF", "Matchen slutade"];
    assert_notifications(expected_titles, apn_state.notifications.clone());

    // TODO: Check if never intermission
    let expected_events = vec!["Nedsläpp", "Period 1", "Period 2", "Period 3", "Matchen slutade"];
    assert_live_activities(expected_events, &apn_state.live_activities);
    
    Ok(())
}

#[tokio::test]
async fn test_lhf_tik_2023_09_16() -> Result<(), Box<dyn std::error::Error>> {
    // Given - start servers with sse entries
    let temp_dir = TempDir::new("integration_test").expect("dir to be created");
    let path = temp_dir.path().to_str().unwrap();

    let game_uuid = "qcz-3SCTnS581".to_string();
    let events = parse_sse_live_events(&format!("./log/sse-live-events/{game_uuid}.log"));

    let mut external_server = ExternalServer::new(8015);
    external_server.start().await;
    external_server.api_state.write().await.store_live_activities = true;

    let mut server = ShlServer::new(8016);
    server.start(path, &external_server.get_url());
    external_server.add_game(Season::get_current(), GameType::Season, SeasonGame { 
        uuid: game_uuid.clone(), 
        homeTeamInfo: get_team_info("LHF", 0), 
        awayTeamInfo: get_team_info("TIK", 0), 
        startDateTime: Utc::now() - chrono::Duration::minutes(5),
        state: "pre-game".to_string(), 
        shootout: false,
        overtime: false, 
        seriesInfo: SeriesInfo { code: shl_server_rs::models::League::SHL },
    }).await;

    // notification user
    let req = AddUser { id: "user_id_SAIK_live_0".to_string(), teams: vec!["TIK".to_string()], apn_token: Some("apn_token_SAIK_0".to_string()), ios_version: None, app_version: None };
    server.retry_add_user(&req).await;

    // live activity user
    let req = AddUser { id: "user_id_SAIK_live_1".to_string(), teams: vec!["TIK".to_string()], apn_token: Some("apn_token_SAIK_1".to_string()), ios_version: None, app_version: None };
    server.retry_add_user(&req).await;
    let start_req = StartLiveActivity { user_id: req.id, token: req.apn_token.unwrap(), game_uuid: game_uuid.clone() };
    _ = server.start_live_acitivty(&start_req).await;

    // When - processing the events
    external_server.push_events(events).await;

    // Then - game should be finished, all events should be sent
    let game_details = server.retry_until_game_reaches(&game_uuid, &GameStatus::Finished, 500).await;

    assert_eq!(game_details.events.len(), 115);
    assert!(!game_details.game.overtime);
    assert!(!game_details.game.shootout);

    let apn_state = external_server.api_state.read().await;
    assert_eq!(apn_state.notifications.len(), 6);

    assert_eq!(apn_state.nr_live_activities.get("apn_token_SAIK_1").unwrap(), &184);

    let expected_titles = vec!["Nedsläpp", "Mål för LHF","Mål för LHF", "Mål för LHF", "Mål för LHF", "LHF vann"];
    assert_notifications(expected_titles, apn_state.notifications.clone());

    let expected_events = vec!["Nedsläpp", "Period 1", "Period 2", "Period 3", "LHF vann"];
    assert_live_activities(expected_events, &apn_state.live_activities);

    let standings = server.get_api_standings(Season::Season2023).await?;
    assert_standing(&standings, "LHF", 1, 1, 3).await;
    assert_standing(&standings, "TIK", 2, 1, 0).await;

    Ok(())
}

#[tokio::test]
async fn test_mif_modo_2023_09_30() -> Result<(), Box<dyn std::error::Error>> {
    // Given - start servers with sse entries
    let temp_dir = TempDir::new("integration_test").expect("dir to be created");
    let path = temp_dir.path().to_str().unwrap();

    let game_uuid = "qcz-3SD5hODQD".to_string();
    let events = parse_sse_live_events(&format!("./log/sse-live-events/{game_uuid}.log"));

    let mut external_server = ExternalServer::new(8017);
    external_server.start().await;
    external_server.api_state.write().await.store_live_activities = true;

    let mut server = ShlServer::new(8018);
    server.start(path, &external_server.get_url());
    external_server.add_game(Season::get_current(), GameType::Season, SeasonGame { 
        uuid: game_uuid.clone(), 
        homeTeamInfo: get_team_info("MIF", 0), 
        awayTeamInfo: get_team_info("MODO", 0), 
        startDateTime: Utc::now() - chrono::Duration::minutes(5),
        state: "pre-game".to_string(), 
        shootout: false,
        overtime: false, 
        seriesInfo: SeriesInfo { code: shl_server_rs::models::League::SHL },
    }).await;

    // notification user
    let req = AddUser { id: "user_id_SAIK_live_0".to_string(), teams: vec!["MODO".to_string()], apn_token: Some("apn_token_SAIK_0".to_string()), ios_version: None, app_version: None };
    server.retry_add_user(&req).await;

    // live activity user
    let req = AddUser { id: "user_id_SAIK_live_1".to_string(), teams: vec!["MODO".to_string()], apn_token: Some("apn_token_SAIK_1".to_string()), ios_version: None, app_version: None };
    server.retry_add_user(&req).await;
    let start_req = StartLiveActivity { user_id: req.id, token: req.apn_token.unwrap(), game_uuid: game_uuid.clone() };
    _ = server.start_live_acitivty(&start_req).await;

    // When - processing the events
    external_server.push_events(events).await;

    // Then - game should be finished, all events should be sent
    let game_details = server.retry_until_game_reaches(&game_uuid, &GameStatus::Finished, 500).await;

    assert!(game_details.game.overtime);
    assert!(!game_details.game.shootout);
    assert_eq!(game_details.events.len(), 121);

    let standings = server.get_api_standings(Season::Season2023).await?;
    assert_standing(&standings, "MODO", 1, 1, 2).await;
    assert_standing(&standings, "MIF", 2, 1, 1).await;

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

fn parse_sse_live_events(path: &str) -> Vec<SseEvent> {
    BufReader::new(File::open(path).expect("no such file")).lines()
        .map(|l| l.expect("Could not parse line"))
        .map(|l| serde_json::from_str::<SseEvent>(&l).expect("Could not decode json"))
        .collect()
}

fn assert_notifications(expected_titles: Vec<&str>, pushed_notifications: Vec<(String, ApnBody)> ) {
    let actual_titles: Vec<String> = pushed_notifications.iter()
        .map(|e| e.1.aps.alert.as_ref().map(|e| e.title.clone()).unwrap_or_default())
        .collect();

    assert_eq!(expected_titles, actual_titles);
}


fn assert_live_activities(expc_titles: Vec<&str>, live_activities: &[(String, ApnBody)] ) {
    let actual_titles: Vec<String> = live_activities.iter()
        .map(|e| e.1.aps.content_state.as_ref().and_then(|e| e.event.as_ref().map(|le| le.title.clone())).unwrap_or_default())
        .collect();
    let mut expected_titles = expc_titles.clone();
    for e in actual_titles {
        if let Some(expected) = expected_titles.first() {
            if  &e.as_str() == expected {
                expected_titles.remove(0);
            }
        }
    }
    assert!(expected_titles.is_empty(), "titles left {:?}", expected_titles);
}

async fn assert_standing(standings: &Standings, team: &str, rank: u8, gp: u16, points: u16) {
    let entry = standings.SHL.iter().find(|e| e.team_code == team).unwrap();
    assert_eq!(entry.rank, rank);
    assert_eq!(entry.gp, gp);
    assert_eq!(entry.points, points);
}

fn get_team_info(code: &str, score: i16) -> GameTeamInfo {
    GameTeamInfo { 
        code: code.to_string(), 
        score: StringOrNum::Number(score), 
        names: TeamNames { code: code.to_string(), long: "".to_string(), short: "".to_string() } 
    }
}