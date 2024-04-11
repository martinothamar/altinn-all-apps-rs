use std::collections::HashMap;

use anyhow::anyhow;
use anyhow::Result;
use anyhow::{bail, Context};
use reqwest::{header, Client, ClientBuilder, Url};
use serde::{Deserialize, Serialize};

const CDN_URL: &'static str = "https://altinncdn.no/";

pub struct CdnClient {
    client: Client,
}

impl CdnClient {
    pub fn new() -> Self {
        let mut headers = header::HeaderMap::new();
        headers.insert("Accept", "appliation/json".parse().unwrap());

        let client = ClientBuilder::new()
            .user_agent("altinn-all-apps-rs")
            .default_headers(headers)
            .build()
            .unwrap();
        CdnClient { client }
    }

    pub async fn get_orgs(&self) -> Result<CdnOrganizations> {
        let url = Url::parse(CDN_URL)?
            .join("/orgs/altinn-orgs.json")
            .context("Failed to build URL")?;

        let response = self.client.get(url).send().await;

        let response = response.context("Failed to fetch orgs - send request")?;

        if !response.status().is_success() {
            bail!("Failed to fetch orgs - invalid status={}", response.status());
        }

        let body = response
            .text()
            .await
            .context("Failed to fetch orgs - reading body of request")?;

        let response = serde_json::from_str::<CdnOrganizations>(&body)
            .map_err(|err| anyhow!("Failed to parse organizations: {:?}\nBody={}", err, body))?;

        Ok(response)
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CdnOrganizations {
    pub orgs: HashMap<String, CdnOrganization>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CdnOrganization {
    pub name: Name,
    pub logo: Option<String>,
    pub orgnr: String,
    pub homepage: String,
    pub environments: Vec<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Name {
    pub en: String,
    pub nb: String,
    pub nn: String,
}
