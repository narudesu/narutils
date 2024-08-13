use std::{
    fs::{self, File},
    io::{BufReader, BufWriter, Write},
};

use anyhow::{bail, Context, Result};
use clap::{Args, CommandFactory, Parser, Subcommand};
use config::{load_app_config, AppConfigError};
use jira::get_jira_issue;
use promkit::preset::listbox::Listbox;
use serde::{Deserialize, Serialize};
use tempo::{fetch_today_tempo_worklog, track_time, WorklogResponse};
mod config;
mod jira;
mod tempo;

fn main() {
    let opts = AppCliOptions::parse();

    match opts.subcommand {
        AppCliSubcommand::FormatCommit(args) => {
            run_command_format_commit(args).expect("command failed")
        }
        AppCliSubcommand::ActivateIssue(args) => {
            run_command_activate_issue(args).expect("command failed")
        }
        AppCliSubcommand::Config => run_command_config().expect("command failed"),
        AppCliSubcommand::GetActiveIssue => run_command_get_active_issue().expect("command failed"),
        AppCliSubcommand::PrintTempoWorklog => {
            run_command_print_tempo_worklog().expect("command failed")
        }
        AppCliSubcommand::TrackTime => run_command_track_time().expect("command failed"),
        AppCliSubcommand::Completions { shell } => {
            shell.generate(&mut AppCliOptions::command(), &mut std::io::stdout())
        }
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
    ActivateIssue(ActivateIssueArgs),
    GetActiveIssue,
    PrintTempoWorklog,
    TrackTime,
    Config,
    Completions {
        #[arg(value_enum)]
        shell: clap_complete_command::Shell,
    },
}

#[derive(Args, Debug)]
pub struct FormatCommitArgs {
    jira_issue: Option<String>,
}

#[derive(Args, Debug)]
pub struct ActivateIssueArgs {
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

fn run_command_activate_issue(args: ActivateIssueArgs) -> std::io::Result<()> {
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

fn run_command_print_tempo_worklog() -> Result<()> {
    let worklog = fetch_today_tempo_worklog()?;

    let billable_hours = worklog.get_billable_hours();

    let started_work_at = worklog.results.first().map(|x| x.start_time.clone());
    let last_entry = &worklog.results.last();

    println!("Today worked hours: {billable_hours}");

    if let Some(started_work_at) = started_work_at {
        println!("Started work at: {started_work_at}");
    }

    if let Some(start_end) = last_entry.map(WorklogResponse::parse_start_end) {
        let (start, end) = start_end?;

        println!("Last entry: {start:?} - {end:?}");
    }

    Ok(())
}

fn run_command_track_time() -> Result<()> {
    let issue_key = load_active_issue_config()
        .map(|x| x.active_issue_key)
        .ok()
        .context("no active issue")?;

    run_command_print_tempo_worklog()?;

    let minutes = Listbox::new([15, 30, 45, 60])
        .title("How many minutes do you want to track?")
        .listbox_lines(5)
        .prompt()?
        .run()?
        .parse::<i64>()?;

    let worklog = fetch_today_tempo_worklog()?;

    let last_entry_end = worklog
        .results
        .last()
        .context("last result not found")?
        .parse_start_end()
        .context("parsing failed")?
        .1;

    track_time(&issue_key, &last_entry_end, i32::try_from(minutes * 60)?)?;

    println!("Time tracked.");

    Ok(())
}
