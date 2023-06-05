use tracing::log;

use crate::{db::Db, user_service::User};

pub struct Migrate {
}
impl Migrate {
    #![allow(unused)]
    pub fn migrate_users() {
        let old_db: Db<String, Vec<User>> = Db::new("v1_users");
        let new_db: Db<String, User> = Db::new("v2_user");
        let all_old_users = old_db.read(&"all".to_string()).expect("Must read old users");
        log::info!("[MIGRATE] Migrating {} users", all_old_users.len());

        for u in all_old_users {
            new_db.write(&u.id, &u).expect("Must write user entry");
        }
    }
}