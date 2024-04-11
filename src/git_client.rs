use std::cell::RefCell;
use std::path::Path;
use std::path::PathBuf;

use crate::configuration::Configuration;
use crate::ui::Ui;
use anyhow::anyhow;
use anyhow::bail;
use anyhow::Context;
use anyhow::Result;
use git2::build::CheckoutBuilder;
use git2::build::RepoBuilder;
use git2::Cred;
use git2::CredentialType;
use git2::FetchOptions;
use git2::RemoteCallbacks;

pub struct GitClient;

impl GitClient {
    fn assert_invariants() -> Result<()> {
        let version = git2::Version::get();
        if !version.threads() {
            return Err(anyhow!("libgit2 was not built with thread support"));
        }
        Ok(())
    }

    pub fn clone(url: &str, config: &Configuration, ui: &Ui) -> Result<()> {
        Self::assert_invariants().context("Failed to clone")?;

        let (org, repo_name) = match url.rsplitn(3, '/').collect::<Vec<_>>()[..] {
            [repo_name, org, _] => (org, repo_name),
            _ => bail!("Invalid git url"),
        };

        let (repo_name, _) = repo_name.rsplit_once('.').context("Invalid git url")?;

        let repo_dir = config.dir.join(org).join(repo_name);

        Self::clone_core(url, &repo_dir, ui, config)
    }

    fn clone_core(url: &str, repo_dir: &PathBuf, ui: &Ui, config: &Configuration) -> Result<()> {
        let state = RefCell::new(State {
            indexed_objects: 0,
            total_objects: 0,
            current_checkout: 0,
            total_checkout: 0,
        });

        {
            let state = state.borrow();
            state.update(ui, url);
        }

        let mut cb = RemoteCallbacks::new();
        cb.transfer_progress(|stats| {
            let mut state = state.borrow_mut();

            state.total_objects = stats.total_objects() as u64;
            state.indexed_objects = stats.indexed_objects() as u64;
            state.update(ui, url);

            true
        });
        cb.credentials(|_, _, allowed_types| {
            if !allowed_types.contains(CredentialType::USER_PASS_PLAINTEXT) {
                return Err(git2::Error::from_str("User/pass credentials not supported by remote"));
            }
            Cred::userpass_plaintext(&config.username, &config.password)
        });

        let mut co = CheckoutBuilder::new();
        co.progress(|path, cur, total| {
            let mut state = state.borrow_mut();

            state.current_checkout = cur as u64;
            state.total_checkout = total as u64;

            state.update(ui, url);
        });

        let mut fo = FetchOptions::new();
        fo.remote_callbacks(cb);
        RepoBuilder::new()
            .fetch_options(fo)
            .with_checkout(co)
            .clone(url, Path::new(repo_dir))
            .context("Failed to clone repo")?;

        Ok(())
    }
}

struct State {
    indexed_objects: u64,
    total_objects: u64,

    current_checkout: u64,
    total_checkout: u64,
}

impl State {
    fn update(&self, ui: &Ui, url: &str) {
        assert!(self.indexed_objects <= self.total_objects);
        assert!(self.current_checkout <= self.total_checkout);

        let fetch_current = match self.total_objects {
            0 => 0.0,
            _ => ((self.indexed_objects as f64 / self.total_objects as f64) * 100.0) / 2.0,
        };
        let checkout_current = match self.total_checkout {
            0 => 0.0,
            _ => ((self.current_checkout as f64 / self.total_checkout as f64) * 100.0) / 2.0,
        };

        ui.update(url, (fetch_current + checkout_current) as u64, 100);
    }
}
