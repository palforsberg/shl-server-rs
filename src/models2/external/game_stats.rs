use serde::{Serialize, Deserialize};

use crate::models::StringOrNum;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Period {
    pub label: String,
    pub value: StringOrNum,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Statistics {
    pub caption: String,
    pub homeTeamValue: i32,
    pub awayTeamValue: i32,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PeriodStatsBreakdown {
    pub period: Period,
    pub statistics: Vec<Statistics>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StatsRsp {
    pub period_stats_breakdown: Vec<PeriodStatsBreakdown>,
}