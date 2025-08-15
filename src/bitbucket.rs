use reqwest::{Client, Error as ReqwestError};
use serde_json::Value;
use std::error::Error as StdError;
use std::fmt;

/// The base URL for the Bitbucket API.
pub const API_URL: &str = "https://api.bitbucket.org/2.0/repositories";

/// Represents errors that can occur while interacting with the Bitbucket API.
#[derive(Debug)]
pub struct CustomError(Box<dyn StdError>);

/// Authorization data structure for connecting to the Bitbucket API
pub struct Bitbucket {
    bitbucket_username: String,
    bitbucket_app_password: String,
    bitbucket_workspace: String,
    bitbucket_repository: String,
    client: Client
}

impl fmt::Display for CustomError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Custom Error: {}", self.0)
    }
}

impl StdError for CustomError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(&*self.0)
    }
}

impl From<ReqwestError> for CustomError {
    fn from(err: ReqwestError) -> Self {
        CustomError(Box::new(err))
    }
}

impl Bitbucket {
    /// Creates a new `Bitbucket` instance with the specified token.
    ///
    /// # Arguments
    ///
    /// * `token` - A personal access token for authenticating with the Bitbucket API use Bearer Authentication
    ///
    /// # Returns
    ///
    /// A new `Bitbucket` instance.
    pub fn new(bitbucket_username: String,
                bitbucket_app_password: String,
                bitbucket_workspace: String,
                bitbucket_repository: String) -> Self {
        let client = Client::new();
        Self {  bitbucket_username, bitbucket_app_password, bitbucket_workspace, bitbucket_repository, client }
    }

    /// Sends an HTTP GET request to the specified URL with the configured token.
    ///
    /// # Arguments
    ///
    /// * `url` - The URL to send the request to.
    ///
    /// # Returns
    ///
    /// A Result containing the response body as a string if the request was successful,
    /// or an error if the request failed.
    pub async fn send_http_request(&self, url: &str) -> Result<String, CustomError> {
        let username = &self.bitbucket_username;
        let password = &self.bitbucket_app_password;

        let response = self
            .client
            .get(url)
            .basic_auth(username, Some(password))
            .header("User-Agent", "Rust")
            .header("Accept", "application/json")
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            return Err(CustomError(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Request failed with status code: {}", status),
            ))));
        }

        let json_string = response.text().await?;
        Ok(json_string)
    }

    /// Retrieves the difference between two branches from the Bitbucket API.
    ///
    /// # Arguments
    ///
    /// * `feature_branch` - The name of the feature branch.
    /// * `compare_branch` - The name of the branch to compare against.
    ///
    /// # Returns
    ///
    /// A Result containing a vector of strings representing the differences
    /// between the two branches, or an error if the operation failed.
    pub async fn get_diff(
        &self,
        feature_branch: &str,
        compare_branch: &str,
    ) -> Result<Vec<String>, CustomError> {
        let feature_branch_commit_id = self.get_latest_commit_id(feature_branch).await?;
        let compare_branch_commit_id = self.get_latest_commit_id(compare_branch).await?;

        let url = format!(
            "{}/{}/{}/diffstat/{}..{}",
            API_URL, self.bitbucket_workspace, self.bitbucket_repository, feature_branch_commit_id, compare_branch_commit_id
        );

        let json_string = self.send_http_request(&url).await?;

        let diff_stats: Value = serde_json::from_str(&json_string).map_err(|e| CustomError(Box::new(e)))?;

        self.get_git_diff_response(diff_stats).await
    }

    /// Parses the JSON response from the Bitbucket API and extracts the differences.
    ///
    /// # Arguments
    ///
    /// * `diff_stats` - The JSON response containing the diff stats.
    ///
    /// # Returns
    ///
    /// A Result containing a vector of strings representing the differences
    /// between the two branches, or an error if the operation failed.
    pub async fn get_git_diff_response(
        &self,
        diff_stats: Value,
    ) -> Result<Vec<String>, CustomError> {
        let mut diff_output: Vec<String> = Vec::new();

        if let Some(values) = diff_stats.get("values").and_then(|v| v.as_array()) {
            for diff in values {
                let status = match diff["status"].as_str() {
                    Some("added") => "A",
                    Some("removed") => "D",
                    Some("modified") => "M",
                    Some("renamed") => "R",
                    Some("merge conflict") => "M",
                    Some("remote deleted") => "D",
                    Some("Unknown") => "?",
                    _ => "?",
                };

                if let (Some(old_file), Some(new_file)) = (diff["old"].as_object(), diff["new"].as_object()) {
                    if diff["status"] == "R" {
                        diff_output.push(format!("{}       {}       {}", status, old_file["path"].as_str().unwrap_or_default(), new_file["path"].as_str().unwrap_or_default()));
                    } else {
                        diff_output.push(format!("{}       {}", status, new_file["path"].as_str().unwrap_or_default()));
                    }
                } else if let Some(old_file) = diff["old"].as_object() {
                    diff_output.push(format!("{}       {}", status, old_file["path"].as_str().unwrap_or_default()));
                } else if let Some(new_file) = diff["new"].as_object() {
                    diff_output.push(format!("{}       {}", status, new_file["path"].as_str().unwrap_or_default()));
                }
            }
        }

        Ok(diff_output)
    }

    /// Retrieves the ID of the latest commit on the specified branch.
    ///
    /// # Arguments
    ///
    /// * `branch` - The name of the branch.
    ///
    /// # Returns
    ///
    /// A Result containing the commit ID if successful, or an error if the operation failed.
    pub async fn get_latest_commit_id(&self, branch: &str) -> Result<String, CustomError> {
        let url = format!("{}/{}/{}/commits/{}", API_URL, self.bitbucket_workspace, self.bitbucket_repository, branch);

        let json_string = self.send_http_request(&url).await?;
        let json: Value = serde_json::from_str(&json_string)
            .map_err(|e| CustomError(Box::new(e)))?;

        let commit_id = match json["values"][0]["hash"].as_str() {
            Some(commit_id) => commit_id.to_string(),
            None => {
                return Err(CustomError(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Commit ID not found",
                ))));
            }
        };
        Ok(commit_id)
    }
}
