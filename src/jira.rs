use anyhow::Result;
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use ureq::{MiddlewareNext, Request, Response};

use crate::config::{load_app_config, AppConfig};

impl AppConfig {
    fn format_auth_header(&self) -> String {
        [
            "Basic",
            // basic auth value must be base64 encoded
            &base64::engine::general_purpose::STANDARD
                .encode([self.jira_username.to_owned(), self.jira_password.to_owned()].join(":")),
        ]
        .join(" ")
    }

    fn format_api_url(&self) -> String {
        [&self.jira_host, "/rest/api/latest"].join("")
    }
}

pub fn load_api_config() -> Result<JiraApiConfiguration> {
    let app_config = load_app_config()?;

    let api_url = app_config.format_api_url();
    let auth_header = app_config.format_auth_header();

    Ok(JiraApiConfiguration {
        auth_header,
        api_url,
    })
}

pub fn get_jira_account_id() -> Result<String> {
    let jira_config = load_api_config()?;
    let myself: MyselfResponse = build_agent()?
        .get(&(jira_config.api_url.to_owned() + "/myself"))
        .call()?
        .into_json()?;

    Ok(myself.account_id)
}

pub fn get_jira_issue(issue_key: &str) -> Result<IssueResponse> {
    let jira_config = load_api_config()?;
    let issue_path = [&jira_config.api_url, "issue", issue_key].join("/");

    let agent = build_agent()?;

    let issue: IssueResponse = agent.get(&issue_path).call().unwrap().into_json().unwrap();

    Ok(issue)
}

fn build_agent() -> Result<ureq::Agent> {
    let config = load_api_config()?;
    let auth_header = config.auth_header.clone();

    Ok(ureq::builder()
        .middleware(
            move |req: Request, next: MiddlewareNext| -> Result<Response, ureq::Error> {
                next.handle(req.set("Authorization", &auth_header))
            },
        )
        .build())
}

#[derive(Debug, Clone)]
pub struct JiraApiConfiguration {
    pub api_url: String,
    pub auth_header: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct IssueResponse {
    pub id: String,
    pub fields: IssueFieldsResponse,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MyselfResponse {
    pub account_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct IssueFieldsResponse {
    pub summary: String,
}
