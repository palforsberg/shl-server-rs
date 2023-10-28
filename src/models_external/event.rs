use std::fmt::Display;

use serde::{Serialize, Deserialize};

use crate::{models::StringOrNum, models_api::{report::GameStatus, event::{ApiGameEvent, GoalInfo, ApiEventType, Player, ShotInfo, PenaltyInfo}}};



#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameReport {
    pub gameUuid: String,

    pub gameTime: String,
    pub statusString: String,
    pub gameState: String,
    pub period: StringOrNum,

    pub homeTeamId: Option<String>,
    pub awayTeamId: Option<String>,
    pub homeTeamScore: StringOrNum,
    pub awayTeamScore: StringOrNum,
    pub revision: u16,
}

impl GameReport {
    pub fn get_status(&self) -> GameStatus {
        GameStatus::get_from(&self.gameState, self.period.to_num())
    }
}

impl GameStatus {
    pub fn get_from(game_state: &str, period: i16) -> GameStatus {
        match game_state {
            "NotStarted" => GameStatus::Coming,
            "GameEnded" => GameStatus::Finished,
            "Intermission" => GameStatus::Intermission,
            "PeriodBreak" => GameStatus::Intermission,
            "ShootOut" => GameStatus::Shootout,
            "OverTime" => GameStatus::Overtime,
            "Ongoing" => period.into(),
            _ => GameStatus::Coming,
        }
    }
}

impl From<i16> for GameStatus {
    fn from(value: i16) -> Self {
        match value {
            1 => GameStatus::Period1,
            2 => GameStatus::Period2,
            3 => GameStatus::Period3,
            4..=10 => GameStatus::Overtime,
            99 => GameStatus::Shootout,
            _ => GameStatus::Period1,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct General {
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Location {
    pub x: f32,
    pub y: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Shot {
    pub team: String,
    pub location: Location,
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GoalExtra {

    pub scorerLong: String,
    pub teamAdvantage: String,
    pub homeAgainst: StringOrNum,
    pub homeForward: StringOrNum,
    pub assist: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Goal {
    pub team: String,
    pub location: Location,
    pub extra: GoalExtra,
}



#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PeriodExtra {
    pub gameStatus: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Period {
    pub extra: PeriodExtra,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Penalty {
    pub team: String,
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlayByPlay {
    pub eventId: i32,
    pub revision: u16,
    pub hash: String,
    pub period: StringOrNum,
    pub gametime: String,
    pub description: String,

    #[serde(flatten)]
    pub class: PlayByPlayType,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "class")]
pub enum PlayByPlayType {
    Period(Period),
    Goal(Goal),
    Penalty(Penalty),
    
    PenaltyShot(Shot),
    Shot(Shot),
    ShotBlocked(Shot),
    ShotIron(Shot),
    ShotWide(Shot),
    ShootoutPenaltyShot(Shot),

    General(General),
    Timeout(General),
    GoolkeeperEvent(General),
    #[serde(rename = "Livefeed_SHL")]
    Livefeed(General),
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Action {
    pub actions: Vec<PlayByPlay>,
    pub gameUuid: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct SseEvent {
    pub gameReport: Option<GameReport>,
    pub playByPlay: Option<Action>,
    pub gameTime: Option<SseGameTime>,
    pub liveEvent: Option<LiveEvent>,
    pub teamStatistics: Option<TeamStatistics>,
    pub liveState: Option<LiveStateEvent>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LiveStateEvent {
    pub gameUuid: String,
    pub liveState: LiveState,
    pub previousLiveState: LiveState,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all="lowercase")]
pub enum LiveState {
    Unknown,
    Ongoing,
    Intermission,
    Decided,
    Overtime,
    Shootout,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameTime {
    pub gameUuid: String,
    pub period: StringOrNum,
    pub periodTime: String,
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SseGameTime {
    pub gameUuid: String,
    pub period: Option<StringOrNum>,
    pub periodTime: Option<String>,
}

impl From<SseGameTime> for Option<GameTime> {
    fn from(value: SseGameTime) -> Self {
        match (value.period, value.periodTime) {
            (Some(period), Some(periodTime)) => Some(GameTime { gameUuid: value.gameUuid, period, periodTime }),
            _ => None
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LiveEventTeam {
    pub teamId: String,
    pub score: StringOrNum,
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EventTeam {
    pub teamId: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag="type", rename_all="lowercase")]

pub enum EventType {
    Shot(ShotType),
    Period(PeriodType),
    Goal(ShotType),
    Penalty(PenaltyType),
    Goalkeeper(GoalkeeperType),
    #[serde(other)]
    Unknown,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ShotType {
    pub time: String,
    pub gameState: String,
    pub goalStatus: Option<String>,
    pub homeTeam: LiveEventTeam,
    pub awayTeam: LiveEventTeam,
    pub eventTeam: EventTeam,
    pub revision: u16,
    pub player: EventPlayer,
}

impl LiveEventTeam {
    pub fn get_team_id(&self) -> String {
        match self.teamId.as_str() {
            _ => self.teamId.clone(),
        }
    }
}

impl EventTeam {
    pub fn get_team_id(&self) -> String {
        match self.teamId.as_str() {
            _ => self.teamId.clone(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PeriodType {
    pub started: bool,
    pub finished: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PenaltyType {
    pub time: String,
    pub gameState: String,
    pub homeTeam: LiveEventTeam,
    pub awayTeam: LiveEventTeam,
    pub eventTeam: EventTeam,
    pub revision: u16,
    pub player: Option<EventPlayer>,
    pub offence: String,
    pub variant: Description,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GoalkeeperType {
    pub time: String,
    pub gameState: String,
}

impl PenaltyType {
    pub fn get_offence(&self) -> String {
        match self.offence.as_str() {
            "HOOK" => "Hooking".to_string(),
            "TRIP" => "Tripping".to_string(),
            "INTRF" => "Interference".to_string(),
            "UN-SP" => "Unsportsmanlike".to_string(),
            "ROUGH" => "Roughing".to_string(),
            "HI-ST" => "High-Sticking".to_string(),
            "TOO-M" => "Too many players".to_string(),
            "HOLD" => "Holding".to_string(),
            "CROSS" => "Crosschecking".to_string(),
            "GK-INTRF" => "Goalkeeper Interference".to_string(),
            "BOARD" => "Boarding".to_string(),
            "DELAY" => "Delay the game".to_string(),
            "KNEE" => "Kneeing".to_string(),
            "SLASH" => "Slashing".to_string(),
            _ => self.offence.to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Description {
    pub description: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EventPlayer {
    pub playerId: StringOrNum,
    pub familyName: String,
    pub firstName: String,
    pub jerseyToday: StringOrNum,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LiveEvent {
    pub gameUuid: String,
    pub eventId: Option<StringOrNum>,
    pub period: StringOrNum,
    #[serde(flatten)]
    pub eventType: Option<EventType>,
}
impl Display for LiveEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.get_event_type() {
            EventType::Period(_) => write!(f, "{} {}", self.get_event_type(), self.period.to_str()),
            _ => write!(f, "{}", self.get_event_type())
        }
    }
}
impl Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventType::Goal(e) => write!(f, "Goal for {} :: {} {} - {} {} :: {}", e.eventTeam.get_team_id(), e.homeTeam.get_team_id(), e.homeTeam.score.to_str(), e.awayTeam.score.to_str(), e.awayTeam.get_team_id(), e.time),
            EventType::Penalty(e) => write!(f, "Penalty for {} :: {} {} - {} {} :: {}", e.eventTeam.get_team_id(), e.homeTeam.get_team_id(), e.homeTeam.score.to_str(), e.awayTeam.score.to_str(), e.awayTeam.get_team_id(), e.time),
            EventType::Shot(e) => write!(f, "Shot for {} :: {} {} - {} {} :: {}", e.eventTeam.get_team_id(), e.homeTeam.get_team_id(), e.homeTeam.score.to_str(), e.awayTeam.score.to_str(), e.awayTeam.get_team_id(), e.time),
            EventType::Period(_) => write!(f, "Period"),
            EventType::Goalkeeper(e) => write!(f, "Goalkeeper :: {} {}", e.gameState, e.time),
            EventType::Unknown => write!(f, "Unknown"),
        }
    }
}
impl LiveEvent {
    pub fn get_event_type(&self) -> &EventType {
        if let Some(e) = &self.eventType {
            e
        } else {
            &EventType::Unknown
        }
    }
    pub fn get_event_id(&self) -> String {
        match self.get_event_type() {
            EventType::Period(e) => {
                if e.finished {
                    format!("PeriodEnd {}", self.period.to_str())
                } else {
                    format!("PeriodStart {}", self.period.to_str())
                }
            },
            _ => self.eventId.as_ref().map(|e| e.to_str()).unwrap_or("eventId".to_string()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TeamStatistics {
    pub gameUuid: String,
    pub teamId: String,
    pub statistics: Vec<PeriodStats>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PeriodStats {
    pub period: StringOrNum,
    pub parsedTotalStatistics: Vec<StatsValue>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StatsValue {
    pub caption: String,
    pub value: Option<StringOrNum>,
}

impl TeamStatistics {
    #![allow(unused)]
    pub fn get_score(&self) -> Option<i16> {
        self.statistics.iter().find(|e| e.period.to_num() == 0)
            .and_then(|e| e.parsedTotalStatistics.iter().find(|a| a.caption == "G"))
            .and_then(|e| e.value.as_ref().map(|a| a.to_num()))
    }
    pub fn get_opponent_score(&self) -> Option<i16> {
        self.statistics.iter().find(|e| e.period.to_num() == 0)
            .and_then(|e| e.parsedTotalStatistics.iter().find(|a| a.caption == "GA"))
            .and_then(|e| e.value.as_ref().map(|a| a.to_num()))
    }
    pub fn get_team_id(&self) -> String {
        match self.teamId.as_str() {
            _ => self.teamId.clone(),
        }
    }
}

impl From<LiveEvent> for ApiGameEvent {
    fn from(value: LiveEvent) -> Self {
        let revision = match &value.get_event_type() {
            EventType::Goal(e) => e.revision,
            EventType::Penalty(e) => e.revision,
            EventType::Shot(e) => e.revision,
            EventType::Period(_) => 1,
            EventType::Goalkeeper(_) => 1,
            EventType::Unknown => 1,
        };
        let status = match &value.get_event_type() {
            EventType::Goal(e) => GameStatus::get_from(e.gameState.as_str(), value.period.to_num()),
            EventType::Penalty(e) => GameStatus::get_from(e.gameState.as_str(), value.period.to_num()),
            EventType::Shot(e) => GameStatus::get_from(e.gameState.as_str(), value.period.to_num()),
            EventType::Goalkeeper(e) => GameStatus::get_from(e.gameState.as_str(), value.period.to_num()),
            EventType::Period(_) => GameStatus::get_from("Ongoing", value.period.to_num()),
            EventType::Unknown => GameStatus::Coming,
        };

        let gametime = match &value.get_event_type() {
            EventType::Goal(e) => e.time.clone(),
            EventType::Penalty(e) => e.time.clone(),
            EventType::Shot(e) => e.time.clone(),
            EventType::Goalkeeper(e) => e.time.clone(),
            EventType::Period(_) => "00:00".to_string(),
            EventType::Unknown => "00:00".to_string(),
        };
        ApiGameEvent {
            game_uuid: value.gameUuid.clone(),
            event_id: value.get_event_id(),
            revision,
            status,
            gametime,
            description: "".to_string(),
            info: match value.get_event_type() {
                EventType::Goal(e) => ApiEventType::Goal(GoalInfo { 
                    team: e.eventTeam.get_team_id(), 
                    player: Some(Player { first_name: e.player.firstName.clone(), family_name: e.player.familyName.clone(), jersey: e.player.jerseyToday.to_num() as i32 }), 
                    team_advantage: e.goalStatus.clone().unwrap_or("EQ".to_string()), 
                    home_team_result: e.homeTeam.score.to_num(), 
                    away_team_result: e.awayTeam.score.to_num(), 
                    location: crate::models_api::event::Location { x: 0.0, y: 0.0 } }),
                EventType::Shot(e) => ApiEventType::Shot(ShotInfo { 
                    team: e.eventTeam.get_team_id(), 
                    location: crate::models_api::event::Location { x: 0.0, y: 0.0 } 
                }),
                EventType::Penalty(e) => ApiEventType::Penalty(PenaltyInfo { 
                    team: e.eventTeam.get_team_id(), 
                    player: e.player.as_ref().map(|p| Player { first_name: p.firstName.clone(), family_name: p.familyName.clone(), jersey: p.jerseyToday.to_num() as i32 }), 
                    reason: e.get_offence(), 
                    penalty: Some(e.variant.description.clone()) }),
                EventType::Period(e) => {
                    if e.finished {
                        ApiEventType::PeriodEnd
                    } else {
                        ApiEventType::PeriodStart
                    }
                },
                _ => ApiEventType::General,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{models_external::event::{EventType, LiveState}, models_api::event::{ApiGameEvent, ApiEventType}};

    use super::{SseEvent, LiveEvent};

    #[test]
    fn sse_event_parsing_shot() {
        let json = r#"{"liveEvent":{"gameUuid":"qcz-3SBK10tiR7","gameSourceId":"20230914-SAIK-MODO","gameId":18001,"eventId":58,"eventUuid":"2c02e3e7-b361-517c-a451-cb6cad916236","round":0,"gameType":"Elitserien","arena":"SkellefteÃ¥ Kraft Arena","attendance":0,"startDateAndTime":"2023-09-14T19:00:00","period":2,"time":"07:29","gameState":"Ongoing","revision":1,"type":"shot","realWorldTime":"2023-09-14T19:57:54.757835","updatedTime":"2023-09-14T19:57:49.391","homeTeam":{"teamId":"SAIK","teamName":"SkellefteÃ¥ AIK","teamCode":"SKE","score":0},"awayTeam":{"teamId":"MODO","teamName":"MoDo Hockey","teamCode":"MoDo","score":1},"eventTeam":{"teamId":"SAIK","place":"home","teamCode":"SKE","teamName":"SkellefteÃ¥ AIK"},"player":{"playerId":"4446","firstName":"Anton","familyName":"Olsson","jerseyToday":"56"},"locationX":163,"locationY":-9,"goalSection":0,"isPenaltyShot":false,"source":"statnet-xml-parser"}}"#;
        let event: SseEvent = serde_json::from_str(json).expect("should pass");
        assert!(event.liveEvent.is_some());
        let live_event = event.liveEvent.unwrap();
        assert_eq!(live_event.period.to_str(), "2");
        match live_event.clone().get_event_type() {
            EventType::Shot(e) => {
                assert_eq!(e.time, "07:29");
                assert_eq!(e.eventTeam.teamId, "SAIK");
                assert_eq!(e.player.firstName, "Anton");
                assert_eq!(e.player.familyName, "Olsson");
                assert_eq!(e.player.jerseyToday.to_str(), "56");
                assert_eq!(e.player.playerId.to_str(), "4446");
            }
            _ => panic!("should be EventType::Shot"),
        }
        let api_event: ApiGameEvent = live_event.into();
        assert_eq!(api_event.gametime, "07:29");
        match api_event.info {
            ApiEventType::Shot(e) => {
                assert_eq!(e.team, "SAIK");
            },
            _ => panic!("Should be ApiEventType::Shot"),
        }
    }

    #[test]
    fn sse_event_parsing_goal() {
        let json = r#"{"liveEvent":{"gameUuid":"qcz-3SBK10tiR7","gameSourceId":"20230914-SAIK-MODO","gameId":18001,"eventId":57,"eventUuid":"a72082cf-4e80-5eab-8d03-e64d03b25ddc","round":0,"gameType":"Elitserien","arena":"SkellefteÃ¥ Kraft Arena","attendance":0,"startDateAndTime":"2023-09-14T19:00:00","period":2,"time":"07:00","gameState":"Ongoing","revision":1,"type":"goal","realWorldTime":"2023-09-14T19:56:49.92593","updatedTime":"2023-09-14T19:57:17.469","homeTeam":{"teamId":"SAIK","teamName":"SkellefteÃ¥ AIK","teamCode":"SKE","score":0},"awayTeam":{"teamId":"MODO","teamName":"MoDo Hockey","teamCode":"MoDo","score":1},"eventTeam":{"teamId":"MODO","place":"away","teamCode":"MoDo","teamName":"MoDo Hockey"},"player":{"playerId":"3219","firstName":"Kristians","familyName":"Rubins","jerseyToday":"33","statistics":[{"key":"G","value":"1"},{"key":"A","value":"0"}]},"locationX":71,"locationY":-81,"homeGoals":0,"awayGoals":1,"goalSection":6,"isPenaltyShot":false,"isEmptyNetGoal":false,"pop":[{"playerId":"4925","firstName":"Riley","familyName":"Woods","jerseyToday":"17"},{"playerId":"5727","firstName":"Josh","familyName":"Dickinson","jerseyToday":"23"},{"playerId":"4024","firstName":"Niklas","familyName":"Folin","jerseyToday":"27"},{"playerId":"4548","firstName":"Mikkel","familyName":"Aagaard","jerseyToday":"29"},{"playerId":"6134","firstName":"Lassi","familyName":"Lehtinen","jerseyToday":"30"},{"playerId":"3219","firstName":"Kristians","familyName":"Rubins","jerseyToday":"33"}],"nep":[{"playerId":"1646","firstName":"Petter","familyName":"Granberg","jerseyToday":"8"},{"playerId":"3274","firstName":"Max","familyName":"Lindholm","jerseyToday":"11"},{"playerId":"1552","firstName":"Oscar","familyName":"Lindberg","jerseyToday":"24"},{"playerId":"2758","firstName":"Linus","familyName":"SÃ¶derstrÃ¶m","jerseyToday":"32"},{"playerId":"2217","firstName":"Arvid","familyName":"Lundberg","jerseyToday":"52"},{"playerId":"2337","firstName":"Jonathan","familyName":"Pudas","jerseyToday":"64"}],"assists":{"first":{"playerId":"4548","firstName":"Mikkel","familyName":"Aagaard","jerseyToday":"29","statistics":[{"key":"G","value":"0"},{"key":"A","value":"1"}]}},"goalStatus":"EQ","source":"statnet-xml-parser"}}"#;
        let event: SseEvent = serde_json::from_str(json).expect("should pass");
        assert!(event.liveEvent.is_some());
        let live_event = event.liveEvent.unwrap();
        assert_eq!(live_event.period.to_str(), "2");
        match live_event.clone().get_event_type() {
            EventType::Goal(e) => {
                assert_eq!(e.time, "07:00");
                assert_eq!(e.eventTeam.teamId, "MODO");
                assert_eq!(e.player.firstName, "Kristians");
                assert_eq!(e.player.familyName, "Rubins");
                assert_eq!(e.player.jerseyToday.to_str(), "33");
                assert_eq!(e.player.playerId.to_str(), "3219");
                assert_eq!(e.homeTeam.score.to_num(), 0);
                assert_eq!(e.awayTeam.score.to_num(), 1);
            }
            _ => panic!("should be EventType::Goal"),
        }
        let api_event: ApiGameEvent = live_event.into();
        assert_eq!(api_event.gametime, "07:00");
        match api_event.info {
            ApiEventType::Goal(e) => {
                assert_eq!(e.team, "MODO");
                assert_eq!(e.player.as_ref().unwrap().first_name, "Kristians");
                assert_eq!(e.player.as_ref().unwrap().jersey, 33);
                assert_eq!(e.home_team_result, 0);
                assert_eq!(e.away_team_result, 1);
            },
            _ => panic!("Should be ApiEventType::Goal"),
        }
    }

    #[test]
    fn sse_event_parsing_period() {
        let json = r#"{"liveEvent":{"gameUuid":"qcz-3SBK10tiR7","gameSourceId":"20230914-SAIK-MODO","gameId":18001,"period":2,"started":true,"startedAt":"2023-09-14T19:46:45.026Z","finished":false,"realWorldTime":"2023-09-14T19:46:45.026Z","type":"period","source":"statnet-xml-parser"}}"#;
        let event: SseEvent = serde_json::from_str(json).expect("should pass");
        assert!(event.liveEvent.is_some());
        let live_event = event.liveEvent.unwrap();
        assert_eq!(live_event.period.to_str(), "2");
        match live_event.clone().get_event_type() {
            EventType::Period(e) => {
                assert!(e.started);
                assert!(!e.finished);
            }
            _ => panic!("should be EventType::Period, was {:?}", live_event.get_event_type()),
        }
        let api_event: ApiGameEvent = live_event.into();
        assert_eq!(api_event.gametime, "00:00");
        match api_event.info {
            ApiEventType::PeriodStart => {

            },
            _ => panic!("Should be ApiEventType::PeriodStart"),
        }
    }
    

    #[test]
    fn sse_event_parsing_penalty() {
        let json = r#"{"liveEvent":{"gameSourceId":"20230914-LIF-OHK","gameId":18003,"eventId":44,"eventUuid":"7c4a1beb-bede-5b15-b27e-b226c849baf6","round":0,"gameType":"Elitserien","arena":"Tegera Arena","attendance":0,"startDateAndTime":"2023-09-14T19:00:00","period":2,"time":"06:27","gameState":"Ongoing","revision":1,"type":"penalty","realWorldTime":"2023-09-14T19:59:35.323384","updatedTime":"2023-09-14T19:59:47.113","homeTeam":{"teamId":"LIF","teamName":"Leksands IF","teamCode":"LIF","score":2},"awayTeam":{"teamId":"OHK","teamName":"Örebro Hockey","teamCode":"ÖRE","score":0},"eventTeam":{"teamId":"LIF","place":"home","teamCode":"LIF","teamName":"Leksands IF"},"player":{"playerId":"4701","firstName":"Arvid","familyName":"Eljas","jerseyToday":"24"},"variant":{"shortName":"Minor","minorTime":"2","doubleMinorTime":"0","benchTime":"0","majorTime":"0","misconductTime":"0","gMTime":"0","mPTime":"0","description":"2 min"},"offence":"HOOK","didRenderInPenaltyShot":false,"gameUuid":"qcz-3SBMPOvMq"}}"#;
        let event: SseEvent = serde_json::from_str(json).expect("should pass");
        assert!(event.liveEvent.is_some());
        let live_event = event.liveEvent.unwrap();
        assert_eq!(live_event.period.to_str(), "2");
        match live_event.clone().get_event_type() {
            EventType::Penalty(e) => {
                let p = e.player.as_ref().unwrap();
                assert_eq!(p.firstName, "Arvid");
                assert_eq!(p.familyName, "Eljas");
                assert_eq!(p.playerId.to_str(), "4701");
                assert_eq!(p.jerseyToday.to_str(), "24");
                assert_eq!(e.offence, "HOOK");
                assert_eq!(e.variant.description, "2 min");
            },
            _ => panic!("not penalty"),
        }
        let api_event: ApiGameEvent = live_event.into();
        assert_eq!(api_event.gametime, "06:27");
        match api_event.info {
            ApiEventType::Penalty(e) => {
                assert_eq!(e.team, "LIF");
                assert_eq!(e.player.as_ref().unwrap().first_name, "Arvid");
                assert_eq!(e.player.as_ref().unwrap().jersey, 24);
                assert_eq!(e.reason, "Hooking");
                assert_eq!(e.penalty, Some("2 min".to_string()));
            },
            _ => panic!("Should be ApiEventType::Penalty"),
        }
    }

    #[test]
    fn sse_event_parsing_goalkeeper_event() {
        let json = r#"{"liveEvent":{"gameSourceId":"20230914-LIF-OHK","gameId":18003,"eventId":110,"eventUuid":"dc0ba33b-5199-5100-a348-e9afc38fe282","round":0,"gameType":"Elitserien","arena":"Tegera Arena","attendance":0,"startDateAndTime":"2023-09-14T19:00:00","period":3,"time":"20:00","gameState":"GameEnded","revision":3,"type":"goalkeeper","realWorldTime":"2023-09-14T21:33:22.748853","updatedTime":"2023-09-14T21:33:15.578","homeTeam":{"teamId":"LIF","teamName":"Leksands IF","teamCode":"LIF","score":3},"awayTeam":{"teamId":"OHK","teamName":"Örebro Hockey","teamCode":"ÖRE","score":5},"eventTeam":{"teamId":"OHK","place":"away","teamCode":"ÖRE","teamName":"Örebro Hockey"},"player":{"playerId":"920","firstName":"Jhonas","familyName":"Enroth","jerseyToday":"1"},"isEntering":false,"gameUuid":"qcz-3SBMPOvMq"}}"#;
        let event: SseEvent = serde_json::from_str(json).expect("should pass");
        assert!(event.liveEvent.is_some());
        let live_event = event.liveEvent.unwrap();
        assert_eq!(&live_event.eventId.clone().unwrap().to_str(), "110");
        assert_eq!(&live_event.period.to_num(), &3);
        match live_event.clone().get_event_type() {
            EventType::Goalkeeper(e) => {
                assert_eq!(e.gameState, "GameEnded");
            },
            _ => panic!("not penalty"),
        }
        let api_event: ApiGameEvent = live_event.into();
        assert_eq!(api_event.gametime, "20:00");
        match api_event.info {
            ApiEventType::General => {},
            _ => panic!("Should be ApiEventType::GameEnd, was {:?}", api_event.info),
        }
    }      

    #[test]
    fn sse_event_parsing_team_statistics() {
        let json = r#"{"teamStatistics":{"gameUuid":"qcz-3SBLgaZcu","source":"statnet-xml-parser","gameId":18002,"teamId":"FHC","teamCode":"FHC","teamName":"FrÃ¶lunda HC","place":"away","statistics":[{"period":0,"parsedTotalStatistics":[{"caption":"G","value":1},{"caption":"PIM","value":10},{"caption":"FOW","value":16},{"caption":"SOG","value":13},{"caption":"SPG","value":11},{"caption":"PPSOG","value":1},{"caption":"Saves","value":33},{"caption":"GA","value":4},{"caption":"SavesPerShot","value":0},{"caption":"PP_perc","value":0},{"caption":"SH_perc","value":0},{"caption":"PPG","value":0},{"caption":"SHGA","value":1},{"caption":"SHG","value":0},{"caption":"PPGA","value":2},{"caption":"NumPP","value":1},{"caption":"NumSH","value":5},{"caption":"Hits","value":15}]},{"period":1,"parsedTotalStatistics":[{"caption":"G","value":0},{"caption":"PIM","value":2},{"caption":"FOW","value":4},{"caption":"SOG","value":6},{"caption":"SPG","value":6},{"caption":"PPSOG","value":1},{"caption":"Saves","value":9},{"caption":"GA","value":3},{"caption":"SavesPerShot","value":0},{"caption":"PP_perc","value":0},{"caption":"SH_perc","value":0},{"caption":"PPG","value":0},{"caption":"SHGA","value":1},{"caption":"SHG","value":0},{"caption":"PPGA","value":1},{"caption":"NumPP","value":1},{"caption":"NumSH","value":1},{"caption":"Hits","value":6}]},{"period":2,"parsedTotalStatistics":[{"caption":"G","value":1},{"caption":"PIM","value":4},{"caption":"FOW","value":4},{"caption":"SOG","value":6},{"caption":"SPG","value":4},{"caption":"PPSOG","value":0},{"caption":"Saves","value":13},{"caption":"GA","value":0},{"caption":"SavesPerShot","value":1},{"caption":"PP_perc","value":null},{"caption":"SH_perc","value":1},{"caption":"PPG","value":0},{"caption":"SHGA","value":0},{"caption":"SHG","value":0},{"caption":"PPGA","value":0},{"caption":"NumPP","value":0},{"caption":"NumSH","value":2},{"caption":"Hits","value":6}]},{"period":3,"parsedTotalStatistics":[{"caption":"G","value":0},{"caption":"PIM","value":4},{"caption":"FOW","value":8},{"caption":"SOG","value":1},{"caption":"SPG","value":1},{"caption":"PPSOG","value":0},{"caption":"Saves","value":11},{"caption":"GA","value":1},{"caption":"SavesPerShot","value":0},{"caption":"PP_perc","value":null},{"caption":"SH_perc","value":0},{"caption":"PPG","value":0},{"caption":"SHGA","value":0},{"caption":"SHG","value":0},{"caption":"PPGA","value":1},{"caption":"NumPP","value":0},{"caption":"NumSH","value":2},{"caption":"Hits","value":3}]}]}}"#;
        let event: SseEvent = serde_json::from_str(json).expect("should pass");
        assert!(event.teamStatistics.is_some());
        let stats = event.teamStatistics.unwrap();
        assert_eq!(stats.get_score(), Some(1));
        assert_eq!(stats.get_opponent_score(), Some(4));
    }      

    #[test]
    fn test_without_type() {
        let json = r#"[{
            "gameSourceId": "20230914-SAIK-MODO",
            "gameId": 18001,
            "period": 99,
            "started": true,
            "startedAt": "2023-09-14T21:19:47.092Z",
            "finished": false,
            "realWorldTime": "2023-09-14T21:19:47.092Z",
            "gameUuid": "qcz-3SBK10tiR7"
        }]"#;
    
        let event: Vec<LiveEvent> = serde_json::from_str(json).expect("should pass");
        assert_eq!(event.len(), 1);
    }


    #[test]
    fn test_with_unknown_type() {
        let json = r#"[{
            "gameSourceId": "20230914-SAIK-MODO",
            "gameId": 18001,
            "type": "timeout",
            "period": 99,
            "started": true,
            "startedAt": "2023-09-14T21:19:47.092Z",
            "finished": false,
            "realWorldTime": "2023-09-14T21:19:47.092Z",
            "gameUuid": "qcz-3SBK10tiR7"
        }]"#;
    
        let event: Vec<LiveEvent> = serde_json::from_str(json).expect("should pass");
        assert_eq!(event.len(), 1);
        assert!(matches!(event.get(0).as_ref().unwrap().eventType.as_ref().unwrap(), EventType::Unknown));
    }

    #[test]
    fn test_with_live_state() {
        let json = r#"{"liveState":{"gameUuid":"qcz-3SCTnS581","gameSourceId":"20230916-LHF-TIK","updated":true,"liveState":"ongoing","previousLiveState":"unknown","source":"statnet-xml-parser"}}"#;

        let event: SseEvent = serde_json::from_str(json).expect("should pass");
        assert!(event.liveState.is_some());
        assert!(matches!(event.liveState.expect("is some").liveState, LiveState::Ongoing));
    }
}