use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use clap::{arg, Parser};
use config::Config;
use reqwest::Url;

/// Utility for cloning all Altinn apps
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Where to put the cloned repos
    #[arg(short, long)]
    dir: Option<PathBuf>,

    /// Base url for the Altinn instance
    #[arg(long = "url")]
    base_url: Option<String>,

    /// Username for authentication
    #[arg(short, long)]
    username: Option<String>,

    /// Password for authentication (token from Gitea)
    #[arg(short, long)]
    password: Option<String>,
}

pub struct Configuration {
    pub dir: PathBuf,
    pub base_url: Url,
    pub username: String,
    pub password: String,
}

impl Configuration {
    pub fn new() -> Result<&'static Self> {
        let args = Args::try_parse()?;

        let settings = Config::builder()
            .add_source(config::File::with_name("config").required(false))
            .add_source(config::Environment::with_prefix("ALTINN").ignore_empty(true))
            .build()
            .context("Failed to build configuration")?;

        let dir = args
            .dir
            .or(settings.get::<PathBuf>("dir").ok())
            .unwrap_or(PathBuf::from("./repos"));

        let base_url = args
            .base_url
            .or(settings.get::<String>("url").ok())
            .unwrap_or("https://altinn.studio".to_string());

        let base_url = Url::parse(&base_url).context("Failed to parse base url")?;

        let username = args
            .username
            .or(settings.get::<String>("username").ok())
            .ok_or_else(|| {
                anyhow!("Username is required - must be configured either as an argument or in a config file")
            })?;

        let password = args
            .password
            .or(settings.get::<String>("password").ok())
            .ok_or_else(|| {
                anyhow!("Password is required - must be configured either as an argument or in a config file")
            })?;

        let config = Configuration {
            dir,
            base_url,
            username,
            password,
        };

        Ok(Box::leak(Box::new(config)))
    }
}
