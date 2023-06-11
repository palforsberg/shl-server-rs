use std::{time::Duration, str::FromStr, fmt::{Display}};

use serde::{Deserialize, Serialize};

use crate::{db::Db, rest_client::{self}, models2::external::{event::{PlayByPlayType, Penalty, Shot, Goal}, self}, game_report_service::{GameStatus}, models::ParseStringError};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Player {
    pub first_name: String,
    pub family_name: String,
    pub jersey: String,
}
impl FromStr for Player {
    type Err = ParseStringError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // 1 Johan Johansson Olsson => Player
        if s.is_empty() {
            return Err(ParseStringError)
        }
        let parts: Vec<&str> = s.split(' ').collect();
        let jersey = parts.first().cloned().unwrap_or_default().to_string(); 
        let first_name = parts.get(1).cloned().unwrap_or_default().to_string();
        let family_name = s.replace(format!("{jersey} {first_name} ").as_str(), "");
        if jersey.is_empty() && first_name.is_empty() && family_name.is_empty() {
            Err(ParseStringError)
        } else {
            Ok(Player { jersey, first_name, family_name })
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Location {
    x: f32,
    y: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]

pub struct GoalInfo {
    pub team: String,
    pub player: Option<Player>,
    pub team_advantage: String,
    pub assist: Option<String>,
    pub home_team_result: i16,
    pub away_team_result: i16,
    pub location: Location,
}

impl GoalInfo {
    pub fn new(a: &Goal) -> GoalInfo {
        GoalInfo { 
            team: a.team.clone(),
            player: a.extra.scorerLong.parse().ok(),
            team_advantage: a.extra.teamAdvantage.clone(),
            assist: Some(a.extra.assist.clone()),
            home_team_result: a.extra.homeForward.to_num(),
            away_team_result: a.extra.homeAgainst.to_num(),
            location: Location { x: a.location.x, y: a.location.y }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]

pub struct PenaltyInfo {
    pub team: String,
    pub player: Option<Player>,
    pub reason: String,
    pub penalty: Option<String>,
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


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ShotInfo {
    pub team: String,
    pub location: Location,
}
impl ShotInfo {
    pub fn new(info: &Shot) -> ShotInfo {
        ShotInfo { team: info.team.clone(), location: Location { x: info.location.x, y: info.location.y } }
    }
}
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct GameEndInfo {
    pub winner: Option<String>,
}

#[derive(PartialEq)]
pub enum ApiEventTypeLevel {
    Low, // only websocket
    Medium, // live activity, show in UI
    High // alert
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "type", content = "info")]
pub enum ApiEventType {
    Goal(GoalInfo),
    PeriodEnd,
    PeriodStart,
    GameEnd(GameEndInfo),
    GameStart,
    Penalty(PenaltyInfo),
    Shot(ShotInfo),
    Timeout,
    General,
}
impl ApiEventType {
    pub fn get_level(&self) -> ApiEventTypeLevel {
        match self {
            Self::Goal(_) => ApiEventTypeLevel::High,
            Self::GameStart => ApiEventTypeLevel::High,
            Self::GameEnd(_) => ApiEventTypeLevel::High,
            Self::Penalty(_) => ApiEventTypeLevel::Medium,
            Self::PeriodStart => ApiEventTypeLevel::Medium,
            Self::PeriodEnd => ApiEventTypeLevel::Medium,
            Self::Timeout => ApiEventTypeLevel::Medium,
            Self::Shot(_) => ApiEventTypeLevel::Low,
            Self::General => ApiEventTypeLevel::Low,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApiGameEvent {
    pub game_uuid: String,
    pub event_id: String,
    pub revision: u16,
    pub status: GameStatus,
    pub gametime: String,
    pub description: String,
    #[serde(flatten)]
    pub info: ApiEventType,
}

impl Display for ApiGameEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} {} :: {:?} • {}", self.info, self.description, self.status, self.gametime)
    }
}

impl external::event::PlayByPlay {
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

impl external::event::PlayByPlay {
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

pub struct EventService;
impl EventService {
 
    pub async fn update(game_uuid: &str, throttle_s: Option<Duration>) -> Option<Vec<ApiGameEvent>> {
        let db_raw: Db<String, Vec<external::event::PlayByPlay>> = Db::new("v2_events_raw");
        // let db: Db<String, Vec<ApiGameEvent>> = Db::new("v2_events_2");

        
        let raw_events = if !db_raw.is_stale(&game_uuid.to_string(), throttle_s) {
            db_raw.read(&game_uuid.to_string()).unwrap_or_default()
        } else {
            rest_client::get_events(game_uuid).await.unwrap_or_default()
        };
        _ = db_raw.write(&game_uuid.to_string(), &raw_events);

        Some(raw_events.into_iter().map(|e| e.into_mapped_event(game_uuid)).collect())
    }

    pub fn store_raw(game_uuid: &str, event: &external::event::PlayByPlay) -> bool {
        let db = Db::<String, Vec<external::event::PlayByPlay>>::new("v2_events_raw");
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

    pub fn store(game_uuid: &str, event: &ApiGameEvent) -> bool {
        let db = Db::<String, Vec<ApiGameEvent>>::new("v2_events_2");
        let mut events: Vec<ApiGameEvent> = db.read(&game_uuid.to_string()).unwrap_or_default();
        let new_event;
        if let Some(pos) = events.iter().position(|e| e.event_id == event.event_id) {
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
        let db = Db::<String, Vec<external::event::PlayByPlay>>::new("v2_events_raw");
        db.read(&game_uuid.to_string()).unwrap_or_default()
            .into_iter().map(|e| e.into_mapped_event(game_uuid))
            .collect()
    }

}

#[cfg(test)]
mod tests {
    use crate::models2::external::event::Penalty;

    use super::{Player, PenaltyInfo};

    #[test]
    fn parse_player() {
        let player_res = "1 Mats Olle Matsson".parse::<Player>();
        assert!(player_res.is_ok());
        let player = player_res.unwrap();
        assert_eq!(player.first_name, "Mats");
        assert_eq!(player.family_name, "Olle Matsson");
        assert_eq!(player.jersey, "1");
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
}