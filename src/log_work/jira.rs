//use serde::{Deserialize, Serialize};
//use hyper::body::HttpBody as _;
//use tokio::io::AsyncWriteExt as _;
//use chrono::Utc as _;
use super::*;

#[derive(Debug)]
pub enum Error {
    JsonParsingError(serde_json::error::Error),
    JiraQueryError(reqwest::Error),
}

impl From<serde_json::error::Error> for Error {
    fn from(err: serde_json::error::Error) -> Error { Error::JsonParsingError(err) }
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Error { Error::JiraQueryError(err) }
}

pub type Result<T> = std::result::Result<T, Error>;

pub struct JiraConfig {
    pub base_url: String,
    pub username: String,
    pub password: Option<String>,
}

#[derive(Serialize, Deserialize, Eq, PartialEq, PartialOrd, Ord, Debug)]
struct ResponseWithIssues {
    issues: Vec<Issue>,
}

#[derive(Serialize, Deserialize, Eq, PartialEq, PartialOrd, Ord, Debug)]
struct Issue {
    key: String,
}


#[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
struct ResponseWithWorklogs {
    worklogs: Vec<StoredWorklogEntry>,
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
struct StoredWorklogEntry {
    comment: String,
    started: DateTime,
    timeSpentSeconds: u64,
    id: String,
    issueId: String,
    author: WorklogAuthor,
}

#[derive(Serialize, Deserialize, Eq, PartialEq, PartialOrd, Ord, Debug)]
struct WorklogAuthor {
    name: String,
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
struct NewWorklogEntry {
    comment: String,
    started: DateTime,
    timeSpentSeconds: u64,
}

async fn retrieve_json<T: for<'de> serde::Deserialize<'de>>(
    queryPath: &str,
    client: &reqwest::Client,
    jira_config: &JiraConfig)
-> Result<T>
{
        client
            .get(&format!("{}{}", jira_config.base_url, queryPath))
            .basic_auth(&jira_config.username, jira_config.password.as_ref())
            .send().await?
            .json::<T>().await
            .map_err(|err| err.into())
}

async fn retrieve_keys(
    day: Date,
    jira_config: &JiraConfig)
-> Result<Vec<String>>
{
    let client = reqwest::Client::new();

    // TODO: add request filter such that not all fields of the Tickets are retrieved
    let uri = format!("/rest/api/2/search?jql=worklogAuthor%3DcurrentUser()+AND+worklogDate%3D{}",
                      day.format("%Y-%m-%d"));
    let issues = retrieve_json::<ResponseWithIssues>(&uri, &client, jira_config)
        .await?
        .issues.drain(..).map(|issue| issue.key).collect();

    for ref issue in &issues {
        let uri = format!("/rest/api/2/issue/{}/worklog", issue);
        let worklogs = retrieve_json::<ResponseWithWorklogs>(&uri, &client, jira_config)
            .await?;
        let worklogs: std::vec::Vec<_> = worklogs
            .worklogs
            .iter()
            .filter(|&entry| entry.author.name == jira_config.username)
            .collect();
        println!("Got logs: {:?}", worklogs);
    }
    Ok(issues)
}

async fn set_logging_for_complete_day(
    day: &work_day::WorkDay,
    jira_config: &JiraConfig)
    -> Result<()>
{
    let issues = retrieve_keys(day.date, jira_config).await?;
    println!("Issues: {:?}", issues);
    Ok(())
}


pub fn update_logging_for_day(
    day: &work_day::WorkDay,
    jira_config: &JiraConfig)
    -> Result<()>
{
    let runtime = tokio::runtime::Runtime::new().expect("Failed to instantiate tokio runtime");
    runtime.block_on(set_logging_for_complete_day(day, jira_config))
}
