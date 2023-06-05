/*
type StatsColumn = {
    name: string,
    type: string,
}

type GoalkeeperStats = {
    info: PlayerInfo
    GA: number,
    NR: number,
    SOGA: number,
    SPGA: number,
    SVS: number,
    'SVS%': number,
}

type PlayerInfo = {
    playerId: number
    teamId: string
    period: number
}

type PlayerStats = {
    info: PlayerInfo
    '+/-': number,
    A: number,
    FOL: number,
    FOPerc: number,
    FOW: number,
    G: number,
    Hits: number,
    NR: number,
    PIM: number,
    POS: string,
    PPG: number,
    PPSOG: number,
    SOG: number,
    SW: number,
    TOI: string,
}
type Player = {
    firstName: string,
    lastName: string,
}

type PlayerStatsResponse = {
    dataColumns: StatsColumn[],
    gkDataColumns: StatsColumn[],
    gkStats: { awayTeamValue: GoalkeeperStats[], homeTeamValue: GoalkeeperStats[] },
    goalkeepers: { awayTeamValue: Record<number, Player>, homeTeamValue: Record<number, Player> },
    stats: { awayTeamValue: PlayerStats[], homeTeamValue: PlayerStats[] },
    players: { awayTeamValue: Record<number, Player>, homeTeamValue: Record<number, Player> },
}
 */

use std::collections::HashMap;

use serde::{Serialize, Deserialize};

use crate::models::StringOrNum;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PlayerName {
    pub firstName: String,
    pub lastName: String,
}

 #[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlayerInfo {
    pub playerId: i32,
    pub teamId: String,
    pub period: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GoalkeeperStats {
    pub info: PlayerInfo,
    pub GA: i32,
    pub NR: i32,
    pub SOGA: i32,
    pub SPGA: i32,
    pub SVS: i32,
    #[serde(rename = "SVS%")]
    pub SVS_perc: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlayerStats {
    pub info: PlayerInfo,
    #[serde(rename = "+/-")]
    pub plus_minus: i32,
    pub A: i32,
    pub FOL: i32,
    pub FOPerc: f32,
    pub FOW: i32,
    pub G: i32,
    pub Hits: i32,
    pub NR: i32,
    pub PIM: i32,
    pub POS: StringOrNum,
    pub PPG: i32,
    pub PPSOG: i32,
    pub SOG: i32,
    pub SW: i32,
    pub TOI: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct EachTeamStats<T: Default> {
    #[serde(default)]
    pub homeTeamValue: T,
    #[serde(default)]
    pub awayTeamValue: T
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StatsColumn {
    pub name: String,
    #[serde(rename = "type")]
    pub stats_type: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PlayerStatsRsp {
    pub dataColumns: Vec<StatsColumn>,
    #[serde(default)]
    pub gkDataColumns: Vec<StatsColumn>,
    #[serde(default)]
    pub gkStats: EachTeamStats<Vec<GoalkeeperStats>>,
    #[serde(default)]
    pub stats: EachTeamStats<Vec<PlayerStats>>,
    #[serde(default)]
    pub goalkeepers: EachTeamStats<HashMap<i32, PlayerName>>,
    #[serde(default)]
    pub players: EachTeamStats<HashMap<i32, PlayerName>>,
}