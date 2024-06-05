use anyhow::anyhow;
use anyhow::Result;
use anyhow::{bail, Context};
use reqwest::{header, Client, ClientBuilder};
use serde::{Deserialize, Serialize};

use crate::configuration::Configuration;

pub struct GiteaClient {
    client: Client,
    configuration: &'static Configuration,
}

impl GiteaClient {
    pub fn new(config: &'static Configuration) -> Self {
        let mut headers = header::HeaderMap::new();
        headers.insert("Accept", "appliation/json".parse().unwrap());
        headers.insert("Authorization", format!("token {}", config.password).parse().unwrap());

        let client = ClientBuilder::new()
            .user_agent("altinn-all-apps-rs")
            .default_headers(headers)
            .build()
            .unwrap();
        GiteaClient {
            client,
            configuration: config,
        }
    }

    pub async fn get_orgs(&self) -> Result<Vec<GiteaOrganization>> {
        let mut result = Vec::<GiteaOrganization>::new();

        let mut page = 1;
        const PAGE_SIZE: usize = 50;

        loop {
            let mut url = self
                .configuration
                .base_url
                .join("/repos/api/v1/orgs")
                .context("Failed to build URL")?;
            url.query_pairs_mut()
                .append_pair("page", &page.to_string())
                .append_pair("limit", &PAGE_SIZE.to_string());

            let response = self.client.get(url).send().await;

            let response = response.context("Failed to fetch orgs - send request")?;

            let status = response.status();
            let body = response
                .text()
                .await
                .context("Failed to fetch orgs - reading body of request")?;

            if !status.is_success() {
                bail!(
                    "Failed to fetch orgs - invalid status - status={} content={}",
                    status,
                    body
                );
            }

            let response = serde_json::from_str::<Vec<GiteaOrganization>>(&body)
                .map_err(|err| anyhow!("Failed to parse orgs: {:?}\nBody={}", err, body))?;

            result.extend_from_slice(&response);

            if response.len() < PAGE_SIZE {
                break;
            }

            page += 1;
        }

        result.dedup_by(|a, b| a.name == b.name);
        result.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(result)
    }

    pub async fn get_repos(&self, org: &str) -> Result<Vec<GiteaRepo>> {
        let mut result = Vec::<GiteaRepo>::new();

        let mut page = 1;
        const PAGE_SIZE: usize = 50;

        loop {
            let mut url = self
                .configuration
                .base_url
                .join(&format!("/repos/api/v1/orgs/{}/repos", org))
                .context("Failed to build URL")?;

            url.query_pairs_mut()
                .append_pair("page", &page.to_string())
                .append_pair("limit", &PAGE_SIZE.to_string());

            let response = self.client.get(url).send().await;

            let response = response.context("Failed to fetch repos - send request")?;

            let status = response.status();
            let body = response
                .text()
                .await
                .context("Failed to fetch repos - reading body of request")?;

            if !status.is_success() {
                bail!(
                    "Failed to fetch repos - invalid status - status={} content={}",
                    status,
                    body
                );
            }

            let response = serde_json::from_str::<Vec<GiteaRepo>>(&body)
                .map_err(|err| anyhow!("Failed to parse repos: {:?}\nBody={}", err, body))?;

            result.extend_from_slice(&response);

            if response.len() < PAGE_SIZE {
                break;
            }

            page += 1;
        }

        result.dedup_by(|a, b| a.clone_url == b.clone_url);
        result.sort_by(|a, b| a.clone_url.cmp(&b.clone_url));

        Ok(result)
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GiteaOrganization {
    #[serde(rename = "id")]
    pub id: i64,

    #[serde(rename = "avatar_url")]
    pub avatar_url: Option<String>,

    #[serde(rename = "name")]
    pub name: Option<String>,

    #[serde(rename = "full_name")]
    pub full_name: Option<String>,

    #[serde(rename = "location")]
    pub location: Option<String>,

    #[serde(rename = "description")]
    pub description: Option<String>,

    #[serde(rename = "email")]
    pub email: Option<String>,

    #[serde(rename = "visibility")]
    pub visibility: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GiteaRepo {
    #[serde(rename = "id")]
    pub id: i64,

    #[serde(rename = "clone_url")]
    pub clone_url: String,

    #[serde(rename = "ssh_url")]
    pub ssh_url: Option<String>,

    #[serde(rename = "url")]
    pub url: Option<String>,

    #[serde(rename = "name")]
    pub name: Option<String>,

    #[serde(rename = "full_name")]
    pub full_name: Option<String>,

    #[serde(rename = "default_branch")]
    pub default_branch: Option<String>,

    #[serde(rename = "link")]
    pub link: Option<String>,

    #[serde(rename = "private")]
    pub private: Option<bool>,
}
