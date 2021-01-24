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
    pub username: Option<String>,
    pub password: Option<String>,
}

#[derive(Serialize, Deserialize, Eq, PartialEq, PartialOrd, Ord, Debug)]
struct Response {
    issues: Vec<Issue>,
}

#[derive(Serialize, Deserialize, Eq, PartialEq, PartialOrd, Ord, Debug)]
struct Issue {
    key: String,
}


//#[derive(Serialize, Deserialize, Eq, PartialEq, PartialOrd, Ord, Debug)]
//struct WorklogEntry {
//    comment: String,
//    started: chrono::Utc,
//    timeSpentSeconds: u64,
//}

async fn retrieve_keys(
    day: Date,
    jira_config: &JiraConfig)
-> Result<Vec<String>>
{
    let uri = format!("{}{}/search?jql=worklogAuthor%3DcurrentUser()+AND+worklogDate%3D{}",
                      jira_config.base_url, "/rest/api/2", day.format("%Y-%m-%d")); //"2021-01-13");
    println!("Querying: {}", uri);
    let mut builder = reqwest::Client::new()
        .get(&uri);
    if let Some(ref username) = &jira_config.username {
        builder =
            builder.basic_auth(username, jira_config.password.as_ref());
    }
    builder
        .send().await?
        .json::<Response>().await
        .map(|mut response| response.issues.drain(..).map(|issue| issue.key).collect())
        .map_err(|err| err.into())
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
