use std::{env, fs, io, str::FromStr, sync::Arc};

use serde::de::DeserializeOwned;
use serenity::prelude::TypeMapKey;

pub struct Config {
    pub token: String,
    pub squawk_response_chance: f32,
    pub squawk_cooldown: i64,
    pub fuk_u_chance: f32,

    pub max_random_squawk_time: i64,
    pub random_squawk_channels: Vec<u64>,
    pub squawk_blacklist_channels: Vec<u64>

}

impl TypeMapKey for Config {
    type Value = Arc<Config>;
}

impl Config {
    pub fn init() -> anyhow::Result<Self> {
        Ok(Config {
            token: var_or_secret("DISCORD_TOKEN", "DISCRD_TOKEN_SECRET_PATH")?,
            squawk_response_chance: var_or_default("SQUAWK_RESPONSE_CHANCE", || 0.001)?,
            fuk_u_chance: var_or_default("FUK_U_CHANCE", || 0.001)?,
            squawk_cooldown: var_or_default("SQUAWK_COOLDOWN", || 604800)?,
            max_random_squawk_time: var_or_default("MAX_RANDOM_SQUAWK_TIME", || 2629746)?,
            random_squawk_channels: var_list("RANDOM_SQUAWK_CHANNELS")?,
            squawk_blacklist_channels: var_list("SQUAWK_BLACKLIST_CHANNELS")?
        })
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ConfigError {
    #[error("missing environment variable {0}")]
    MissingVar(String),
    #[error("failed to parse environmnet variable {0}")]
    BadVar(String),

    #[error("failed to parse secret file {0}")]
    BadSecret(String),
    #[error("failed to read secret file: {0}")]
    SecretIO(#[from] io::Error),
}

fn var_list<T: DeserializeOwned>(var: &str) -> Result<Vec<T>, ConfigError> {
    if let Ok(s) = env::var(var) {
        match serde_json::from_str(&s) {
            Ok(l) => Ok(l),
            Err(_) => Err(ConfigError::BadVar(var.to_string()))
        }
    } else {
        Ok(Vec::new())
    }
}

fn var_or_secret<T: FromStr>(var: &str, secret_var: &str) -> Result<T, ConfigError> {
    if let Ok(s) = env::var(var) {
        if let Ok(t) = T::from_str(&s) {
            return Ok(t);
        } else {
            return Err(ConfigError::BadVar(var.to_string()))
        }
    }

    if let Ok(s) = env::var(secret_var) {
        match fs::read_to_string(&s) {
            Ok(file) => { 
                if let Ok(t) = T::from_str(&file) {
                    return Ok(t);
                } else {
                    return Err(ConfigError::BadSecret(s))
                }
            },
            Err(e) => return Err(ConfigError::SecretIO(e))
        }
    }

    Err(ConfigError::MissingVar(format!("{var} or {secret_var}")))
}

fn var_or_default<T: FromStr>(var: &str, default: fn() -> T) -> Result<T, ConfigError> {
    if let Ok(s) = env::var(var) {
        if let Ok(t) = T::from_str(&s) {
            return Ok(t);
        } else {
            return Err(ConfigError::BadVar(var.to_string()));
        }
    }

    Ok(default())
}
