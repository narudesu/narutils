use std::{
    fs::{self, File},
    io::{BufReader, BufWriter, Write},
};

use anyhow::{bail, Result};
use clap::{Args, Parser, Subcommand};
use config::{load_app_config, AppConfigError};
use jira::get_jira_issue;
use serde::{Deserialize, Serialize};
mod config;
mod jira;

fn main() {
    let opts = AppCliOptions::parse();

    match opts.subcommand {
        AppCliSubcommand::FormatCommit(args) => {
            run_command_format_commit(args).expect("command failed")
        }
        AppCliSubcommand::StartIssue(args) => {
            run_command_activate_issue(args).expect("command failed")
        }
        AppCliSubcommand::Config => run_command_config().expect("command failed"),
        AppCliSubcommand::GetActiveIssue => run_command_get_active_issue().expect("command failed"),
    };
}

#[derive(Parser)]
struct AppCliOptions {
    #[command(subcommand)]
    subcommand: AppCliSubcommand,
}

#[derive(Subcommand, Debug)]
enum AppCliSubcommand {
    FormatCommit(FormatCommitArgs),
    StartIssue(StartIssueArgs),
    GetActiveIssue,
    Config,
}

#[derive(Args, Debug)]
pub struct FormatCommitArgs {
    jira_issue: Option<String>,
}

#[derive(Args, Debug)]
pub struct StartIssueArgs {
    jira_issue: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ActiveIssueConfig {
    active_issue_key: String,
}

fn run_command_config() -> Result<()> {
    let result = load_app_config();

    match result {
        Ok(config) => {
            dbg!(config);
        }
        Err(x) => match x.downcast_ref() {
            Some(AppConfigError::FileNotFound) => {
                println!(
                    "To configure the application, please create a file .narutils/config.json and fill it with values."
                );
            }
            None => {
                bail!(x);
            }
        },
    };

    Ok(())
}

fn run_command_get_active_issue() -> Result<()> {
    let issue_key = load_active_issue_config().map(|x| x.active_issue_key).ok();

    match issue_key {
        None => {
            println!("No active issue selected.")
        }
        Some(issue_key) => {
            let issue = get_jira_issue(&issue_key)?;
            let summary = &issue.fields.summary;
            let issue_url = load_app_config()?.format_jira_issue_url(&issue_key);

            println!("issue_key: {issue_key}\nsummary: {summary}\nurl: {issue_url}")
        }
    }

    Ok(())
}

fn run_command_activate_issue(args: StartIssueArgs) -> std::io::Result<()> {
    let issue_key = parse_issue_key(&args.jira_issue);

    fs::create_dir_all(".narutils")?;

    let file = File::create(".narutils/active_issue.json")?;

    let mut writer = BufWriter::new(file);

    let config = ActiveIssueConfig {
        active_issue_key: issue_key.to_owned(),
    };

    serde_json::to_writer_pretty(&mut writer, &config)?;

    writer.flush()?;

    Ok(())
}

fn load_active_issue_config() -> Result<ActiveIssueConfig> {
    let file = File::open(".narutils/active_issue.json")?;
    let reader = BufReader::new(file);
    let config: ActiveIssueConfig = serde_json::from_reader(reader)?;

    Ok(config)
}

fn run_command_format_commit(args: FormatCommitArgs) -> Result<()> {
    let issue_key = match &args.jira_issue {
        Some(jira_issue) => parse_issue_key(jira_issue).to_owned(),
        None => load_active_issue_config()?.active_issue_key,
    };
    let issue = get_jira_issue(&issue_key)?;

    println!("fix: {}", issue.fields.summary);

    Ok(())
}

fn parse_issue_key(input: &str) -> &str {
    regex::Regex::new(r"TTM-\d{1,6}")
        .unwrap()
        .find(input)
        .expect("could not parse issue key")
        .as_str()
}
