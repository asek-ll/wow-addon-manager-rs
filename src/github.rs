use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Debug, Clone)]
pub struct GitHubClient {
    client: Client,
    token: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Commit {
    pub sha: String,
    pub commit: CommitInfo,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CommitInfo {
    pub message: String,
}

impl GitHubClient {
    pub fn new(token: Option<String>) -> Self {
        let mut client_builder = Client::builder();

        // Add user agent
        client_builder = client_builder.user_agent("wow-addon-manager-rs/0.1.0");

        // Add token if provided
        if let Some(token) = &token {
            client_builder = client_builder.default_headers({
                let mut headers = reqwest::header::HeaderMap::new();
                let auth_value = format!("Bearer {}", token);
                headers.insert(
                    reqwest::header::AUTHORIZATION,
                    reqwest::header::HeaderValue::from_str(&auth_value).unwrap(),
                );
                headers
            });
        }

        Self {
            client: client_builder.build().expect("Failed to build HTTP client"),
            token,
        }
    }

    pub async fn get_latest_commit(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<String, Box<dyn Error>> {
        let url = format!("https://api.github.com/repos/{}/{}/commits", owner, repo);
        let response = self.client.get(&url).send().await?;

        if response.status().is_success() {
            let commits: Vec<Commit> = response.json().await?;
            if let Some(commit) = commits.first() {
                Ok(commit.sha.clone())
            } else {
                Err("No commits found".into())
            }
        } else {
            Err(format!("GitHub API error: {}", response.status()).into())
        }
    }

    pub async fn download_repository(
        &self,
        owner: &str,
        repo: &str,
        ref_name: &str,
    ) -> Result<Vec<u8>, Box<dyn Error>> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/tarball/{}",
            owner, repo, ref_name
        );

        let response = self.client.get(&url).send().await?;

        if response.status().is_success() {
            let data = response.bytes().await?;
            Ok(data.to_vec())
        } else {
            Err(format!("Failed to download repository: {}", response.status()).into())
        }
    }
}
