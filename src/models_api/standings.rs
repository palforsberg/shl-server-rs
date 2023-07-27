use serde::{Serialize, Deserialize};

use crate::models::League;

pub type TeamCode = String;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Standings {
    pub SHL: Vec<Standing>,
    pub HA: Vec<Standing>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Standing {
    pub team_code: TeamCode,
    pub rank: u8,
    
    pub gp: u16,
    pub points: u16,
    pub diff: i16,
    pub league: League,
}
