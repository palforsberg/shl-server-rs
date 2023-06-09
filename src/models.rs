use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::str::FromStr;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum League {
    SHL,
    HA,
}
impl League {
    pub fn get_all() -> Vec<League> {
        vec![League::SHL, League::HA]
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum GameType {
    Season,
    PlayOff,
    Demotion,
}

impl GameType {
    pub fn get_all() -> Vec<GameType> {
        vec![GameType::Season, GameType::PlayOff, GameType::Demotion]
    }
}

impl FromStr for GameType {
    type Err = ParseStringError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Season" => Ok(GameType::Season),
            "PlayOff" => Ok(GameType::PlayOff),
            "Demotion" => Ok(GameType::Demotion),
            _ => Err(ParseStringError)
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash, PartialEq, Eq)]
pub enum Season {
    Season2023,
    Season2022,
    Season2021,
    Season2020,
    Season2019,
    Season2018,
}

impl Season {
    pub fn get_current() -> Season {
        Season::Season2022
    }
    pub fn is_current(&self) -> bool {
        self == &Season::get_current()
    }
    pub fn get_all() -> Vec<Season> {
        vec![Season::Season2018, Season::Season2019, Season::Season2020, Season::Season2021, Season::Season2022, Season::Season2023]
    }
}
impl FromStr for Season {
    type Err = ParseStringError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "2023" => Ok(Season::Season2023),
            "Season2023" => Ok(Season::Season2023),
            "2022" => Ok(Season::Season2022),
            "Season2022" => Ok(Season::Season2022),
            "2021" => Ok(Season::Season2021),
            "Season2021" => Ok(Season::Season2021),
            "2020" => Ok(Season::Season2020),
            "Season2020" => Ok(Season::Season2020),
            "2019" => Ok(Season::Season2019),
            "Season2019" => Ok(Season::Season2019),
            "2018" => Ok(Season::Season2018),
            "Season2018" => Ok(Season::Season2018),
            _ => Err(ParseStringError)
        }
    }
}
impl Display for Season {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}


#[derive(Debug, PartialEq, Eq)]
pub struct ParseStringError;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum StringOrNum {
    String(String),
    Arr(Vec<String>),
    Number(i16),
}

impl StringOrNum {
    pub fn to_num(&self) -> i16 {
        match self {
            StringOrNum::String(str) => str.parse::<i16>().unwrap_or(0),
            StringOrNum::Number(n) => *n,
            StringOrNum::Arr(a) => a.get(0).map(|e| e.parse::<i16>().ok()).unwrap_or(None).unwrap_or(0),
        }
    }

    pub fn to_str(&self) -> String {
        match self {
            StringOrNum::String(str) => str.to_owned(),
            StringOrNum::Number(n) => n.to_string(),
            StringOrNum::Arr(a) => a.join("-"),
        }
    }
}

#[derive(Clone)]
pub struct SeasonKey(pub Season, pub League, pub GameType);

impl std::fmt::Display for SeasonKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}/{:?}/{:?}", self.0, self.1, self.2)
    }
}
