use super::*;
use std::convert::TryFrom as _;

#[derive(Debug)]
pub enum Error {
    JsonParsingError(serde_json::error::Error),
    NetworkError(reqwest::Error),
    HttpErrorStatusCode(reqwest::StatusCode),
    ConversionError(core::num::TryFromIntError),
    MiscError(String),
}

impl From<serde_json::error::Error> for Error {
    fn from(err: serde_json::error::Error) -> Error {
        Error::JsonParsingError(err)
    }
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Error {
        Error::NetworkError(err)
    }
}

impl From<core::num::TryFromIntError> for Error {
    fn from(err: core::num::TryFromIntError) -> Error {
        Error::ConversionError(err)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error {
        Error::MiscError(format!("{:?}", err))
    }
}

pub type Result<T> = std::result::Result<T, Error>;

pub struct JiraConfig {
    pub base_url: String,
    pub username: String,
    pub password: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, Eq, PartialEq, PartialOrd, Ord, Debug)]
struct ResponseWithIssues {
    issues: Vec<Issue>,
}

#[derive(serde::Serialize, serde::Deserialize, Eq, PartialEq, PartialOrd, Ord, Debug)]
struct Issue {
    key: String,
}

#[derive(serde::Serialize, serde::Deserialize, Eq, PartialEq, Debug)]
struct ResponseWithWorklogs {
    worklogs: Vec<StoredWorklogEntry>,
}

#[derive(serde::Serialize, serde::Deserialize, Eq, PartialEq, Debug)]
struct StoredWorklogEntry {
    comment: String,
    started: DateTime,
    id: String,
    #[serde(rename = "timeSpentSeconds")]
    time_spent_seconds: u64,
    #[serde(rename = "issueId")]
    issue_id: String,
    author: WorklogAuthor,
}

#[derive(serde::Serialize, serde::Deserialize, Eq, PartialEq, PartialOrd, Ord, Debug)]
struct WorklogAuthor {
    name: String,
}

#[derive(serde::Serialize, serde::Deserialize, Eq, PartialEq, Debug)]
struct NewWorklogEntry {
    comment: String,
    #[serde(with = "my_date_format")]
    started: DateTime,
    #[serde(rename = "timeSpentSeconds")]
    time_spent_seconds: u64,
}

mod my_date_format {
    use super::*;
    use serde::Deserialize as _;

    const FORMAT: &str = "%Y-%m-%dT%H:%M:%S%.3f%z";

    pub fn serialize<S>(date: &DateTime, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let s = format!("{}", date.format(FORMAT));
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> std::result::Result<DateTime, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        DateTime::parse_from_str(&s, FORMAT).map_err(serde::de::Error::custom)
    }
}

async fn retrieve_json<T: for<'de> serde::Deserialize<'de>>(
    query_path: &str,
    client: &reqwest::Client,
    jira_config: &JiraConfig,
) -> Result<T> {
    let response = client
        .get(&format!("{}{}", jira_config.base_url, query_path))
        .basic_auth(&jira_config.username, jira_config.password.as_ref())
        .send()
        .await?;
    if !response.status().is_success() {
        return Err(Error::HttpErrorStatusCode(response.status()));
    }
    response.json::<T>().await.map_err(|err| err.into())
}

async fn post_worklog(
    issue_name: &str,
    new_worklog: &NewWorklogEntry,
    client: &reqwest::Client,
    jira_config: &JiraConfig,
) -> Result<()> {
    println!("POSTING ISSUE {} ({:?})", issue_name, new_worklog);
    let response = client
        .post(&format!(
            "{}/rest/api/2/issue/{}/worklog",
            jira_config.base_url, issue_name
        ))
        .basic_auth(&jira_config.username, jira_config.password.as_ref())
        .json(new_worklog)
        .send()
        .await?;
    if !response.status().is_success() {
        return Err(Error::HttpErrorStatusCode(response.status()));
    }
    Ok(())
}

async fn retrieve_issues_with_worklogs(
    day: &Date,
    client: &reqwest::Client,
    jira_config: &JiraConfig,
) -> Result<Vec<String>> {
    // TODO: add request filter such that not all fields of the Tickets are retrieved
    let uri = format!(
        "/rest/api/2/search?jql=worklogAuthor%3DcurrentUser()+AND+worklogDate%3D{}",
        day.format("%Y-%m-%d")
    );
    let issues = retrieve_json::<ResponseWithIssues>(&uri, &client, jira_config)
        .await?
        .issues
        .drain(..)
        .map(|issue| issue.key)
        .collect();
    Ok(issues)
}

async fn is_jira_issue(
    issue: &str,
    client: &reqwest::Client,
    jira_config: &JiraConfig,
) -> Result<bool> {
    let uri = format!("/rest/api/2/issue/{}?fields=id", issue);
    if let Err(e) = retrieve_json::<Issue>(&uri, client, jira_config).await {
        match e {
            Error::HttpErrorStatusCode(reqwest::StatusCode::NOT_FOUND) => return Ok(false),
            _ => return Err(e),
        }
    }
    Ok(true)
}

fn has_jira_key_structure(candidate: &str) -> bool {
    lazy_static::lazy_static! {
        static ref RE: regex::Regex =
            regex::Regex::new(r"^[^- ]+-[0-9]+$").expect("Erronuous expression for JIRA issue key");
    }
    RE.is_match(candidate)
}

async fn do_update_logging_for_days(
    days: &std::vec::Vec<&work_day::WorkDay>,
    jira_config: &JiraConfig,
) -> Result<()> {
    let client = reqwest::Client::new();

    let mut issues_with_old_logs = std::collections::BTreeSet::new();
    println!(
        "Retrieving issues with logs on one of the {} day(s)",
        days.len()
    );
    for ref day in days {
        let mut issues = retrieve_issues_with_worklogs(&day.date, &client, jira_config).await?;
        issues_with_old_logs.extend(issues.drain(..));
    }

    let relevant_days: std::collections::HashSet<_> =
        days.iter().map(|ref day| day.date.clone()).collect();

    let my_logs = {
        let mut my_logs = std::vec::Vec::new();
        for ref issue in &issues_with_old_logs {
            let uri = format!("/rest/api/2/issue/{}/worklog", issue);
            let mut worklogs =
                retrieve_json::<ResponseWithWorklogs>(&uri, &client, jira_config).await?;
            let mut worklogs: std::vec::Vec<_> = worklogs
                .worklogs
                .drain(..)
                .filter(|entry| {
                    entry.author.name == jira_config.username
                        && relevant_days.contains(&entry.started.date())
                })
                .collect();
            my_logs.append(&mut worklogs);
        }
        my_logs
    };
    println!(
        "Found {} old log entries of user {}",
        my_logs.len(),
        jira_config.username
    );

    // println!("Would delete: {:?}", my_logs.iter().map(|ref log| format!("{}_{}", log.issue_id, log.id)).collect::<std::vec::Vec<_>>());
    for ref worklog in my_logs {
        println!("Do you want to delete old worklog entry for issue={} start_time='{}' duration={}(secs)? (yN): ",
                 worklog.issue_id, worklog.started, worklog.time_spent_seconds);
        // this blocks on purpose (see documentation of tokio::io::stdin())
        let mut buf = String::new();
        std::io::stdin().read_line(&mut buf)?;
        if buf.as_str() == "y\n" {
            // TODO: delete the entry
            let uri = format!(
                "{}/rest/api/2/issue/{}/worklog/{}",
                jira_config.base_url, worklog.issue_id, worklog.id
            );
            let response = client
                .delete(uri.as_str())
                .basic_auth(&jira_config.username, jira_config.password.as_ref())
                .send()
                .await
                .map_err(|err| {
                    println!("ERR: {:?}", err);
                    err
                })?;
            if !response.status().is_success() {
                return Err(Error::HttpErrorStatusCode(response.status()));
            }
        } else {
            println!("ignoring old entry");
        }
    }

    let possible_issue_names: std::collections::BTreeSet<_> = days
        .iter()
        .flat_map(|day| day.entries.iter())
        .map(|entry| &entry.key)
        .filter(|issue_name| has_jira_key_structure(issue_name.as_str()))
        .map(|issue_name| issue_name)
        .collect();

    // don't verify issue names that we've already seen
    let issues_with_old_logs = issues_with_old_logs.iter().collect();
    let mut confirmed_issues = &issues_with_old_logs & &possible_issue_names;
    let possible_issue_names = &possible_issue_names - &issues_with_old_logs;

    // check whether the given issue names exist
    let mut unknown_issues = std::collections::BTreeSet::new();
    for issue in possible_issue_names {
        match is_jira_issue(issue, &client, jira_config).await {
            Err(e) => {
                println!("Error while verifying issue='{}': {:?}", issue, e);
                unknown_issues.insert(issue);
            }
            Ok(success) => {
                if success {
                    confirmed_issues.insert(issue);
                } else {
                    unknown_issues.insert(issue);
                }
            }
        }
    }

    // perform the worklogs
    let mut transmitted = std::vec::Vec::new();
    let mut without_issue = std::vec::Vec::new();
    let mut with_transmission_error = std::vec::Vec::new();
    for day in days {
        for entry in &day.entries {
            if confirmed_issues.contains(&entry.key) && !entry.duration.is_zero() {
                let new_worklog = NewWorklogEntry {
                    comment: itertools::join(&entry.sub_keys, " "),
                    started: day.date.and_time(entry.start_ts),
                    time_spent_seconds: u64::try_from(entry.duration.num_seconds())?,
                };
                match post_worklog(entry.key.as_str(), &new_worklog, &client, jira_config).await {
                    Ok(()) => transmitted.push(entry.clone()),
                    Err(e) => {
                        println!("Error transmitting {:?}: {:?}", entry, e);
                        with_transmission_error.push(entry.clone());
                    }
                }
            } else {
                without_issue.push(entry.clone());
            }
        }
    }
    println!("Added {} worklog entries, ignored {} because of they were not correct, and {} transmission errors",
             transmitted.len(), without_issue.len(), with_transmission_error.len());

    Ok(())
}

pub fn update_logging_for_days(
    days: &std::vec::Vec<&work_day::WorkDay>,
    jira_config: &JiraConfig,
) -> Result<()> {
    let runtime = tokio::runtime::Runtime::new().expect("Failed to instantiate tokio runtime");
    runtime.block_on(do_update_logging_for_days(days, jira_config))
}
