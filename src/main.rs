#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unreachable_code)]

use std::io::ErrorKind;

use anyhow::anyhow;
use anyhow::{Context, Result};
use cdn_client::CdnClient;
use configuration::Configuration;
use futures::future::join_all;
use spmc::Receiver;
use tokio::fs;
use tokio::task::spawn_blocking;
use tokio::task::JoinError;

use crate::git_client::GitClient;
use crate::gitea_client::GiteaClient;
use crate::gitea_client::GiteaRepo;
use crate::ui::Ui;

mod cdn_client;
mod configuration;
mod git_client;
mod gitea_client;
mod ui;

#[tokio::main]
async fn main() -> Result<()> {
    let config = Configuration::new()?;

    init(config).await?;

    println!("Cloning into: {}", config.dir.display());
    println!("--------------------------------------------------");

    let gitea_client = GiteaClient::new(config);
    let cdn_client = CdnClient::new();

    let orgs = cdn_client.get_orgs().await?;

    let repos = gitea_client.get_repos("ttd").await?;

    let cpus_to_use = num_cpus::get().min(8);

    let (mut tx, rx) = spmc::channel::<GiteaRepo>();

    let (ui, ui_thread) = Ui::new();

    let mut threads = Vec::with_capacity(cpus_to_use);
    for id in 0..cpus_to_use {
        let rx = rx.clone();
        let ui = ui.clone();

        let thread = spawn_blocking(move || thread(id, rx, config, ui));

        threads.push(thread);
    }

    for repo in repos.into_iter() {
        tx.send(repo).context("Failed to queue repo")?;
    }

    drop(tx);

    let results = join_all(threads)
        .await
        .into_iter()
        .collect::<Result<Vec<_>, JoinError>>()?;

    let repo_count = results.iter().sum::<u64>();

    drop(ui);

    ui_thread.await.context("Failed to wait for UI thread")?;

    println!("--------------------------------------------------");
    println!("Cloned {} repos", repo_count);

    Ok(())
}

async fn init(config: &Configuration) -> Result<()> {
    if is_root::is_root() {
        return Err(anyhow!("Can't run as root, it's safest to run as a normal user"));
    }

    let dir = &config.dir;
    match dir.metadata() {
        Ok(metadata) => match metadata {
            _ if !metadata.is_dir() => return Err(anyhow!("Can only clone repos into a directory")),
            _ if metadata.permissions().readonly() => return Err(anyhow!("Can't clone repos into a read-only dir")),
            _ => {}
        },
        Err(e) if e.kind() == ErrorKind::NotFound => {
            fs::create_dir_all(&dir)
                .await
                .context("Failed to create dir to place repos")?;
        }
        Err(e) => return Err(e).context("Failed to get directory metadata"),
    };

    if fs::read_dir(dir)
        .await
        .context("Failed to read directory")?
        .next_entry()
        .await
        .context("Failed to read directory")?
        .is_some()
    {
        return Err(anyhow!("Directory is not empty"));
    }

    // Since checking for folder write permissions is kind of complicated apparantly,
    // we'll just try to write a file there and see if it works
    let uuid = uuid::Uuid::new_v4();
    let canary = dir.join(format!(".canary.{}.txt", uuid));
    match fs::write(&canary, "canary").await {
        Ok(_) => fs::remove_file(&canary).await.context("Failed to remove canary file")?,
        Err(e) => return Err(e).context("Does not have write permissions to the directory"),
    }

    Ok(())
}

fn thread(id: usize, rx: Receiver<GiteaRepo>, config: &Configuration, ui: Ui) -> u64 {
    let mut count = 0;

    while let Ok(repo) = rx.recv() {
        match GitClient::clone(&repo.clone_url, config, &ui) {
            Ok(_) => {}
            Err(err) => {
                panic!("Thread {} - Failed to clone {}: {}", id, repo.clone_url, err);
                break;
            }
        }
        count += 1;

        if count == 4 {
            break;
        }
    }

    count
}
