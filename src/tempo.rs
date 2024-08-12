use anyhow::{Context, Result};
use chrono::{Local, NaiveTime, TimeDelta};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use ureq::{MiddlewareNext, Request, Response};

use crate::{
    config::load_app_config,
    jira::{get_jira_account_id, get_jira_issue},
};

pub fn track_time(issue_key: &str, start_time: &NaiveTime, time_spent_seconds: i32) -> Result<()> {
    let config = load_app_config()?
        .tempo
        .context(AppTempoError::NotConfigured)?;

    let agent = build_agent()?;

    let issue = get_jira_issue(issue_key)?;
    let account_id = get_jira_account_id()?;

    let today_iso_date = Local::now().format("%Y-%m-%d").to_string();

    let request = CreateWorklogRequest {
        start_date: today_iso_date,
        start_time: start_time.format("%H:%M:%S").to_string(),
        time_spent_seconds,
        issue_id: issue.id.parse::<i32>()?,
        author_account_id: account_id,
    };
    let response = agent
        .post(&[&config.api_url, "worklogs"].join("/"))
        .send_json(request)?;

    dbg!(response);

    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateWorklogRequest {
    author_account_id: String,
    issue_id: i32,
    start_date: String,
    start_time: String,
    time_spent_seconds: i32,
}

// see docs at https://apidocs.tempo.io/#tag/Worklogs/operation/getWorklogsByUser
pub fn fetch_today_tempo_worklog() -> Result<UserWorklogsResponse> {
    let config = load_app_config()?
        .tempo
        .context(AppTempoError::NotConfigured)?;

    let account_id = get_jira_account_id()?;
    dbg!(&account_id);

    let agent = build_agent()?;

    let today_iso_date = Local::now().format("%Y-%m-%d").to_string();

    let response: UserWorklogsResponse = agent
        .get(&[&config.api_url, "worklogs", "user", &account_id].join("/"))
        .query("from", &today_iso_date)
        .query("to", &today_iso_date)
        .call()?
        .into_json()?;

    Ok(response)
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct WorklogResponse {
    pub billable_seconds: i32,
    pub _start_date: String,
    pub start_time: String,
    pub _tempo_worklog_id: i32,
}

impl WorklogResponse {
    pub fn parse_start_end(&self) -> Result<(NaiveTime, NaiveTime)> {
        let start = NaiveTime::parse_from_str(&self.start_time, "%H:%M:%S")?;
        let end = start
            .overflowing_add_signed(TimeDelta::seconds(self.billable_seconds.into()))
            .0;

        Ok((start, end))
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UserWorklogsResponse {
    pub results: Vec<WorklogResponse>,
}

impl UserWorklogsResponse {
    pub fn get_billable_hours(&self) -> f64 {
        let billable_seconds: f64 = self
            .results
            .iter()
            .map(|x| f64::from(x.billable_seconds))
            .sum();

        billable_seconds / 3600.0
    }
}

fn build_agent() -> Result<ureq::Agent> {
    let config = load_app_config()?
        .tempo
        .context(AppTempoError::NotConfigured)?;
    let auth_header = ["Bearer", &config.token].join(" ");

    Ok(ureq::builder()
        .middleware(
            move |req: Request, next: MiddlewareNext| -> Result<Response, ureq::Error> {
                next.handle(req.set("Authorization", &auth_header))
            },
        )
        .build())
}

#[derive(Error, Debug)]
pub enum AppTempoError {
    #[error("app config file could not be found")]
    NotConfigured,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TempoConfiguration {
    pub token: String,
    pub api_url: String,
    pub project_id: String,
}
