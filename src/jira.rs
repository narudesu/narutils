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

pub fn get_jira_issue(issue_key: &str) -> Result<IssueResponse> {
    let jira_config = load_api_config()?;
    let issue_path = [&jira_config.api_url, "issue", issue_key].join("/");

    let agent = build_agent();

    let issue: IssueResponse = agent.get(&issue_path).call().unwrap().into_json().unwrap();

    Ok(issue)
}

#[allow(clippy::result_large_err)]
fn jira_middleware(req: Request, next: MiddlewareNext) -> Result<Response, ureq::Error> {
    let jira_config = load_api_config().unwrap();

    next.handle(req.set("Authorization", &jira_config.auth_header))
}

fn build_agent() -> ureq::Agent {
    ureq::builder().middleware(jira_middleware).build()
}

#[derive(Debug)]
pub struct JiraApiConfiguration {
    pub api_url: String,
    pub auth_header: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct IssueResponse {
    pub fields: IssueFieldsResponse,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct IssueFieldsResponse {
    pub summary: String,
}
