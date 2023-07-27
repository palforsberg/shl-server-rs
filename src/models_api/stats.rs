use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApiGameTeamStats {
    pub g: i32,
    pub sog: i32,
    pub pim: i32,
    pub fow: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApiGameStats {
    pub home: ApiGameTeamStats,
    pub away: ApiGameTeamStats,
}
