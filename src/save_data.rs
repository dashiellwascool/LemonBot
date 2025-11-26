use std::{fs::File, sync::{Arc}};

use anyhow::Result;
use chrono::{DateTime, Utc};
use ron::de::from_reader;
use serde::{Deserialize, Serialize};
use serenity::prelude::TypeMapKey;
use tokio::sync::RwLock;

const PATH: &str = "./data.ron";

#[derive(Serialize, Deserialize, Default)]
pub struct SaveData {
    pub squawk_cooldown: DateTime<Utc>,
    pub next_random_squawk: DateTime<Utc>
}

impl TypeMapKey for SaveData {
    type Value = Arc<RwLock<SaveData>>;
}

impl SaveData {
    pub fn load_or_default() -> Result<Self> {
        let file = match File::open(PATH) {
            Ok(f) => f,
            Err(_) => {
                return Ok(SaveData::default());
            }
        };

        Ok(from_reader(file)?)
    }

    pub fn save(&self) -> Result<()> {
        let f = File::options()
            .create(true)
            .truncate(true)
            .write(true)
            .open(PATH)?;

        ron::Options::default().to_io_writer(f, &self)?;

        Ok(())
    }
}

