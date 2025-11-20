use std::{env, fs, io, str::FromStr};

pub struct Config {
    pub token: String,
    pub squawk_response_chance: f32,
    pub squawk_response_cooldown: u64,
    pub fuk_u_chance: f32,
}

impl Config {
    pub fn init() -> anyhow::Result<Self> {
        Ok(Config {
            token: var_or_secret("DISCORD_TOKEN", "DISCRD_TOKEN_SECRET_PATH")?,
            squawk_response_chance: var_or_default("SQUAWK_RESPONSE_CHANCE", || 0.001)?,
            fuk_u_chance: var_or_default("FUK_U_CHANCE", || 0.001)?,
            squawk_response_cooldown: var_or_default("SQUAWK_RESPONSE_COOLDOWN", || 604800)?
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
