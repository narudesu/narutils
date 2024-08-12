use crate::tempo::TempoConfiguration;
use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};
use std::{fs::File, io::BufReader, path::Path};
use thiserror::Error;

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

#[derive(Error, Debug)]
pub enum AppConfigError {
    #[error("app config file could not be found")]
    FileNotFound,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AppConfig {
    pub jira_host: String,
    pub jira_username: String,
    pub jira_password: String,
    pub tempo: Option<TempoConfiguration>,
}

impl AppConfig {
    pub fn format_jira_issue_url(&self, issue_key: &str) -> String {
        [&self.jira_host, "browse", issue_key].join("/")
    }
}
