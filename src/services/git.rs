#![allow(dead_code)]

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use git2::{IndexAddOption, Repository, Signature};

use crate::error::AppError;

#[derive(Clone)]
pub struct GitService {
    repo_root: Arc<PathBuf>,
}

impl GitService {
    pub fn new(root: PathBuf) -> Self {
        Self {
            repo_root: Arc::new(root),
        }
    }

    fn root(&self) -> &Path {
        &self.repo_root
    }

    pub fn init_repo_if_needed(&self) -> Result<(), AppError> {
        if self.root().join(".git").exists() {
            return Ok(());
        }

        Repository::init(self.root())?;
        Ok(())
    }

    pub fn commit_ai_changes(&self, message: &str) -> Result<(), AppError> {
        let repo = Repository::discover(self.root())?;
        let mut index = repo.index()?;
        index.add_all(["ai"].iter(), IndexAddOption::DEFAULT, None)?;
        if index.is_empty() {
            return Ok(());
        }
        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;
        let signature = Signature::now("kawaii-mood-bot", "moodbot@local")?;

        let parent_commits = repo
            .head()
            .ok()
            .and_then(|head| head.target())
            .and_then(|oid| repo.find_commit(oid).ok())
            .map(|commit| vec![commit])
            .unwrap_or_default();

        let parent_refs = parent_commits.iter().collect::<Vec<_>>();
        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &parent_refs,
        )?;

        Ok(())
    }
}
