use serde::{Deserialize, Serialize};

use crate::db::Db;


#[derive(Serialize, Deserialize, Clone)]
pub struct Status {
    msg: String,
    lvl: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ApiStatusRsp {
    status: Option<Status>,
}

pub struct StatusService {

}
impl StatusService {
    pub fn read_raw() -> String {
        let db: Db<String, Status> = Db::new("v2_status");
        db.read_raw(&"key".to_string())
    }
}