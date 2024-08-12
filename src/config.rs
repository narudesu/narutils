use std::{fs::File, io::BufReader, path::Path};

use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};

pub fn load_app_config() -> Result<AppConfig> {
    let path = Path::new(".narutils/config.json");

    if !path.exists() {
        bail!(AppConfigError::FileNotFound);
    }

    let reader = BufReader::new(File::open(path)?);

    let config: AppConfig = serde_json::from_reader(reader)
        .map_err(|err| anyhow!(err).context("Failed to parse file."))?;

    Ok(config)
}

#[derive(Debug)]
pub enum AppConfigError {
    FileNotFound,
}

impl std::fmt::Display for AppConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FileNotFound => write!(f, "File not found."),
        }
    }
}
impl std::error::Error for AppConfigError {}

#[derive(Serialize, Deserialize, Debug)]
pub struct AppConfig {
    pub jira_host: String,
    pub jira_username: String,
    pub jira_password: String,
}
