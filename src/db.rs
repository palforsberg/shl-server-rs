use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;
use tracing::log;
use std::collections::HashMap;
use std::fmt::Display;
use std::fs::File;
use std::time::{Instant, Duration, SystemTime};
use walkdir::WalkDir;

pub struct Db<K: Display, V: DeserializeOwned + Serialize> {
    pub name: String,
    pub key_type: std::marker::PhantomData<K>,
    pub value_type: std::marker::PhantomData<V>,
}

impl<K: Display, V: DeserializeOwned + Serialize> Db<K, V> {
    pub fn new(name: &str) -> Db<K, V> {
        Db {
            name: name.to_string(),
            key_type: std::marker::PhantomData,
            value_type: std::marker::PhantomData,
        }
    }

    pub fn read(&self, key: &K) -> Option<V> {
        let before = Instant::now();
        let path = self.get_path(&key.to_string());
        Db::<K, V>::read_file(&path)
    }

    pub fn read_all(&self) -> Vec<V> {
        let before = Instant::now();

        let path = format!("./db/{}", self.name);
        let result: Vec<V> = WalkDir::new(path).into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.metadata().ok().map(|e| e.is_file()).unwrap_or(false))
            .filter_map(|entry| Db::<K, V>::read_file(entry.path().to_str().unwrap()))
            .collect();

        log::info!("[DB] read all {} {} {:.0?}", self.name, result.len(), before.elapsed());
        result
    }

    pub fn read_raw(&self, key: &K) -> String {
        let path = self.get_path(&key.to_string());
        let data = std::fs::read_to_string(path);
        log::info!("[DB] Read raw from file {}", &key.to_string());
        data.unwrap_or_default()
    }

    pub fn write(&self, key: &K, obj: &V) -> std::io::Result<()> {
        let before = Instant::now();
        let json = serde_json::to_string(&obj)?;
        let path = std::path::PathBuf::from(self.get_path(&key.to_string()));
        std::fs::create_dir_all(path.parent().unwrap())?;
        let result = std::fs::write(path, json);
        
        match result {
            Ok(e) => {
                log::debug!("[DB] Wrote to file {}/{} {:.2?}", self.name, key, before.elapsed());
                Ok(e)
            },
            Err(e) => {
                log::debug!("[DB] Write failed {}/{} {}", self.name, key, e);
                Ok(())
            }
        }
    }

    pub fn is_stale(&self, key: &K, delta_s: Option<Duration>) -> bool {
        let path = self.get_path(&key.to_string());
        std::fs::metadata(path)
            .and_then(|e| e.modified())
            .map(|m| {
                if let Some(delta_s) = delta_s {
                    SystemTime::now().duration_since(m).unwrap() > delta_s
                } else {
                    false // if None and file exists => never stale
                }
            })
            .unwrap_or(true) // file doesn't exists => stale
    }

    fn read_file(path: &str) -> Option<V> {
        let before = Instant::now();
        let data = std::fs::read_to_string(path).ok()?;
        let res = match serde_json::from_str(&data) {
            Ok(e) => Some(e),
            Err(e) => {
                log::error!("[DB] Read failed {} {}", path, e);
                None
            }
        };
        log::debug!("[DB] Read from file {path} {:.2?}", before.elapsed());
        res
    }

    fn get_path(&self, key: &str) -> String {
        format!("./db/{}/{}", self.name, key)
    }
}
