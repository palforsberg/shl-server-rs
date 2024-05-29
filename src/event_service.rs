use std::{time::Duration, str::FromStr, fmt::Display};

use crate::{db::Db, rest_client::{self}, models_external::{event::{PlayByPlayType, Penalty, Shot, Goal, LiveEvent, EventType, PeriodType}, self}, models::{ParseStringError, Season, StringOrNum}, models_api::event::*};


impl FromStr for Player {
    type Err = ParseStringError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // 1 Johan Johansson Olsson => Player
        if s.is_empty() {
            return Err(ParseStringError)
        }
        let parts: Vec<&str> = s.split(' ').collect();
        let jersey = parts.first().cloned().unwrap_or_default().to_string().parse::<i32>().ok().unwrap_or_default();
        let first_name = parts.get(1).cloned().unwrap_or_default().to_string();
        let family_name = s.replace(format!("{jersey} {first_name} ").as_str(), "");
        if first_name.is_empty() && family_name.is_empty() {
            Err(ParseStringError)
        } else {
            Ok(Player { id: None, jersey, first_name, family_name })
        }
    }
}

impl GoalInfo {
    pub fn new(a: &Goal) -> GoalInfo {
        GoalInfo { 
            team: a.team.clone(),
            player: a.extra.scorerLong.parse().ok(),
            team_advantage: a.extra.teamAdvantage.clone(),
            home_team_result: a.extra.homeForward.to_num(),
            away_team_result: a.extra.homeAgainst.to_num(),
            location: Some(Location { x: a.location.x, y: a.location.y })
        }
    }
}


impl PenaltyInfo {
    pub fn new(description: &str, p: &Penalty) -> PenaltyInfo {
        let (player_info, penalty_info) = description.split_once(" utvisas ")
            .map(|e| (Some(e.0), Some(e.1)))
            .unwrap_or_else(|| (None, None));
        let (penalty, reason) = penalty_info.unwrap_or_default().split_once(',')
            .map(|e| (Some(e.0.to_string()), e.1.to_string()))
            .unwrap_or_else(|| (None, description.to_string()));

        let player = player_info.unwrap_or_default().parse::<Player>().ok();
        PenaltyInfo { 
            team: p.team.clone(), 
            player, 
            reason: reason.trim().to_string(), 
            penalty 
        }
    }
}

impl ShotInfo {
    pub fn new(info: &Shot) -> ShotInfo {
        ShotInfo { team: info.team.clone(), player: None, location: Some(Location { x: info.location.x, y: info.location.y }) }
    }
}


impl Display for ApiGameEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} {} :: {:?} • {}", self.info, self.description, self.status, self.gametime)
    }
}

impl models_external::event::PlayByPlay {
    fn to_type(&self) -> ApiEventType {
        match &self.class {
            PlayByPlayType::General(_) => ApiEventType::General,
            PlayByPlayType::Livefeed(_) => ApiEventType::General,
            PlayByPlayType::GoolkeeperEvent(_) => ApiEventType::General,

            PlayByPlayType::Goal(a) => ApiEventType::Goal(GoalInfo::new(a)),

            PlayByPlayType::Shot(a) =>          ApiEventType::Shot(ShotInfo::new(a)),
            PlayByPlayType::ShotBlocked(a) =>   ApiEventType::Shot(ShotInfo::new(a)),
            PlayByPlayType::ShotWide(a) =>      ApiEventType::Shot(ShotInfo::new(a)),
            PlayByPlayType::ShotIron(a) =>      ApiEventType::Shot(ShotInfo::new(a)),
            PlayByPlayType::PenaltyShot(a) =>   ApiEventType::Shot(ShotInfo::new(a)),
            PlayByPlayType::ShootoutPenaltyShot(a) => ApiEventType::Shot(ShotInfo::new(a)),

            PlayByPlayType::Penalty(a) => ApiEventType::Penalty(PenaltyInfo::new(&self.description, a)),

            PlayByPlayType::Timeout(_) => ApiEventType::Timeout,

            PlayByPlayType::Period(a) => match a.extra.gameStatus.as_str() {
                "Playing" => ApiEventType::PeriodStart,
                _ => ApiEventType::PeriodEnd,
            },
        }
    }
}

impl models_external::event::PlayByPlay {
    pub fn into_mapped_event(self, game_uuid: &str) -> ApiGameEvent {
        let info: ApiEventType = self.to_type();
        ApiGameEvent {
            game_uuid: game_uuid.to_string(),
            event_id: format!("{}", self.eventId),
            revision: self.revision,
            status: self.period.to_num().into(),
            gametime: self.gametime.clone(),
            description: self.description,
            info,
        }
    }
}


fn add_period_events(raw_events: Vec<LiveEvent>) -> Vec<LiveEvent> {
    if raw_events.is_empty() {
        return raw_events
    }

    let mut result: Vec<LiveEvent> = Vec::new();
    let mut last_period = 1;

    result.push(LiveEvent { 
        gameUuid: raw_events.first().map(|e| e.gameUuid.clone()).unwrap_or("game_uuid".to_string()), 
        eventId: None, 
        period: StringOrNum::Number(1), 
        eventType: Some(EventType::Period(PeriodType { started: true, finished: false})) 
    });

    for e in raw_events.iter() {
        if matches!(e.get_event_type(), EventType::Goal(_) | EventType::Penalty(_) | EventType::Shot(_)) && 
            e.period.to_num() != last_period {
            result.push(LiveEvent { 
                gameUuid: e.gameUuid.to_string(), 
                eventId: None, 
                period: StringOrNum::Number(last_period), 
                eventType: Some(EventType::Period(PeriodType { started: true, finished: true })) 
            });
            result.push(LiveEvent { 
                gameUuid: e.gameUuid.to_string(), 
                eventId: None, 
                period: e.period.clone(),
                eventType: Some(EventType::Period(PeriodType { started: true, finished: false })) 
            });
            last_period = e.period.to_num();
        }
        if !matches!(e.get_event_type(), EventType::Period(_)) {
            result.push(e.clone());
        }
    }

    result
}

pub struct EventService;
impl EventService {
 
    pub async fn update(season: &Season, game_uuid: &str, throttle_s: Option<Duration>) -> Option<Vec<ApiGameEvent>> {
        match season {
            Season::Season2023 => EventService::update_2023_season(game_uuid, throttle_s).await,
            _ => EventService::update_older_season(game_uuid, throttle_s).await,
        }
    }

    async fn update_2023_season(game_uuid: &str, throttle_s: Option<Duration>) -> Option<Vec<ApiGameEvent>> {
        let db_raw: Db<String, Vec<LiveEvent>> = Db::new("v2_events_raw_2023");

        let raw_events = if !db_raw.is_stale(&game_uuid.to_string(), throttle_s) {
            db_raw.read(&game_uuid.to_string()).unwrap_or_default()
        } else {
            let mut raw_events = rest_client::get_events_2023(game_uuid).await.unwrap_or_default();
            raw_events.reverse();
            let result = add_period_events(raw_events);
            _ = db_raw.write(&game_uuid.to_string(), &result);
            result
        };

        Some(raw_events.into_iter().map(|e| e.into()).rev().collect())
    }
    

    async fn update_older_season(game_uuid: &str, throttle_s: Option<Duration>) -> Option<Vec<ApiGameEvent>> {
        let db_raw: Db<String, Vec<models_external::event::PlayByPlay>> = Db::new("v2_events_raw");

        let raw_events = if !db_raw.is_stale(&game_uuid.to_string(), throttle_s) {
            db_raw.read(&game_uuid.to_string()).unwrap_or_default()
        } else {
            let raw_events = rest_client::get_events(game_uuid).await.unwrap_or_default();
            _ = db_raw.write(&game_uuid.to_string(), &raw_events);
            raw_events
        };

        Some(raw_events.into_iter().map(|e| e.into_mapped_event(game_uuid)).collect())
    }


    pub fn store_raw(game_uuid: &str, event: &LiveEvent) -> bool {
        let db = Db::<String, Vec<LiveEvent>>::new("v2_events_raw_2023");
        let mut events = db.read(&game_uuid.to_string()).unwrap_or_default();
        let new_event;
        if let Some(pos) = events.iter().position(|e| e.get_event_id() == event.get_event_id()) {
            events[pos] = event.clone();
            new_event = false;
        } else {
            events.push(event.clone());
            new_event = true;
        }
        _ = db.write(&game_uuid.to_string(), &events);
        new_event
    }

    pub fn store_raws(game_uuid: &str, events: &Vec<LiveEvent>) -> Vec<LiveEvent> {
        let db = Db::<String, Vec<LiveEvent>>::new("v2_events_raw_2023");
        let mut stored_events = db.read(&game_uuid.to_string()).unwrap_or_default();
        let mut new_event = Vec::new();
        for event in events {
            if let Some(pos) = stored_events.iter().position(|e| e.get_event_id() == event.get_event_id()) {
                stored_events[pos] = event.clone();
            } else {
                stored_events.push(event.clone());
                new_event.push(event.clone());
            }
        }
        _ = db.write(&game_uuid.to_string(), &stored_events);
        new_event
    }

    pub fn store_older_raw(game_uuid: &str, event: &models_external::event::PlayByPlay) -> bool {
        let db = Db::<String, Vec<models_external::event::PlayByPlay>>::new("v2_events_raw");
        let mut events = db.read(&game_uuid.to_string()).unwrap_or_default();
        let new_event;
        if let Some(pos) = events.iter().position(|e| e.eventId == event.eventId) {
            events[pos] = event.clone();
            new_event = false;
        } else {
            events.push(event.clone());
            new_event = true;
        }
        _ = db.write(&game_uuid.to_string(), &events);
        new_event
    }

    pub fn read(game_uuid: &str) -> Vec<ApiGameEvent> {
        let db = Db::<String, Vec<models_external::event::LiveEvent>>::new("v2_events_raw_2023");
        db.read(&game_uuid.to_string()).unwrap_or_default()
            .into_iter().map(|e| e.into())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::{models_external::event::{Penalty, LiveEvent, EventType, ShotType, EventTeam}, models::StringOrNum};

    use super::{Player, PenaltyInfo, add_period_events};

    #[test]
    fn parse_player() {
        let player_res = "1 Mats Olle Matsson".parse::<Player>();
        assert!(player_res.is_ok());
        let player = player_res.unwrap();
        assert_eq!(player.first_name, "Mats");
        assert_eq!(player.family_name, "Olle Matsson");
        assert_eq!(player.jersey, 1);
    }

    #[test]
    fn parse_player_err() {
        let player_res = "".parse::<Player>();
        assert!(player_res.is_err());
    }

    #[test]
    fn parse_penalty_info() {
        let info = PenaltyInfo::new("1 Olle Olsson utvisas 5min, roughing", &Penalty { team: "LHF".to_string() });
        assert_eq!(info.penalty.unwrap(), "5min");
        assert_eq!(info.reason, "roughing");
        assert_eq!(info.player.unwrap().first_name, "Olle");
        assert_eq!(info.team, "LHF");
    }


    #[test]
    fn parse_penalty_info2() {
        let info = PenaltyInfo::new("Too many players on ice", &Penalty { team: "LHF".to_string() });
        assert_eq!(info.penalty, None);
        assert_eq!(info.reason, "Too many players on ice");
        assert_eq!(info.player, None);
        assert_eq!(info.team, "LHF");
    }

    #[test]
    fn test_add_period_events() {
        let events = vec![get_event(1), get_event(3), get_event(4), get_event(4), get_event(99), get_event(99)];
        let result = add_period_events(events);
        assert_eq!(result.len(), 6 + 7);

        let p1 = result[0].clone();
        assert!(matches!(p1.eventType.as_ref().expect("msg"), EventType::Period(_)));
        assert_eq!(p1.period.to_num(), 1);
        assert_period(&p1, true, false);

        let p1 = result[2].clone();
        assert!(matches!(p1.eventType.as_ref().expect("msg"), EventType::Period(_)));
        assert_eq!(p1.period.to_num(), 1);
        assert_period(&p1, true, true);

        let p1 = result[3].clone();
        assert!(matches!(p1.eventType.as_ref().expect("msg"), EventType::Period(_)));
        assert_eq!(p1.period.to_num(), 3);
        assert_period(&p1, true, false);

        let p1 = result[5].clone();
        assert!(matches!(p1.eventType.as_ref().expect("msg"), EventType::Period(_)));
        assert_eq!(p1.period.to_num(), 3);
        assert_period(&p1, true, true);

        let p1 = result[6].clone();
        assert!(matches!(p1.eventType.as_ref().expect("msg"), EventType::Period(_)));
        assert_eq!(p1.period.to_num(), 4);
        assert_period(&p1, true, false);

        let p1 = result[9].clone();
        assert!(matches!(p1.eventType.as_ref().expect("msg"), EventType::Period(_)));
        assert_eq!(p1.period.to_num(), 4);
        assert_period(&p1, true, true);

        let p1 = result[10].clone();
        assert!(matches!(p1.eventType.as_ref().expect("msg"), EventType::Period(_)));
        assert_eq!(p1.period.to_num(), 99);
        assert_period(&p1, true, false);
    }

    fn assert_period(period: &LiveEvent, started: bool, finished: bool) {
        match period.get_event_type() {
            EventType::Period(e) => {
                assert_eq!(e.finished, finished);
                assert_eq!(e.started, started);
            },
            _ => panic!(""),
        }
    }

    fn get_event(period: i16) -> LiveEvent {
        LiveEvent { gameUuid: "u".to_string(), eventId: Some(StringOrNum::Number(1)), period: StringOrNum::Number(period), eventType: Some(EventType::Goal(ShotType { 
            time: "00:00".to_string(), gameState: "Ongoing".to_string(), goalStatus: None, 
            homeTeam: crate::models_external::event::LiveEventTeam { teamId: "SAIK".to_string(), score: StringOrNum::Number(1) }, 
            awayTeam: crate::models_external::event::LiveEventTeam { teamId: "SAIK".to_string(), score: StringOrNum::Number(1) }, 
            eventTeam: EventTeam { teamId: "SAIK".to_string() }, 
            revision: 1, 
            player: crate::models_external::event::EventPlayer { playerId: StringOrNum::Number(1), familyName: "Ole".to_string(), firstName: "ole".to_string(), jerseyToday: StringOrNum::Number(1) } 
        }))}
    } 
}