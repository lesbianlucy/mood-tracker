#![allow(dead_code)]

use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use chrono::{DateTime, Local, Utc};
use git2::{IndexAddOption, Repository, Signature, StatusOptions};

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

        let repo = Repository::init(self.root())?;
        self.ensure_gitignore()?;
        let mut index = repo.index()?;
        index.add_all(["."].iter(), IndexAddOption::DEFAULT, None)?;
        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;
        let signature = Signature::now("kawaii-mood-bot", "moodbot@local")?;
        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            "chore: initial commit für den kawaii Moodtracker 💖",
            &tree,
            &[],
        )?;
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

    pub fn status(&self) -> Result<GitStatus, AppError> {
        let repo = Repository::discover(self.root())?;
        let head = repo.head().ok();
        let branch = head
            .as_ref()
            .and_then(|h| h.shorthand())
            .unwrap_or("detached")
            .to_string();
        let head_commit = head
            .and_then(|h| h.target())
            .and_then(|oid| repo.find_commit(oid).ok())
            .map(|commit| {
                let time = commit.time();
                let utc = DateTime::<Utc>::from_timestamp(time.seconds(), 0)
                    .unwrap_or_else(|| DateTime::<Utc>::from_timestamp(0, 0).unwrap());
                let timestamp = utc
                    .with_timezone(&Local)
                    .format("%d.%m.%Y %H:%M")
                    .to_string();
                GitCommitInfo {
                    hash: commit.id().to_string(),
                    message: commit
                        .message()
                        .unwrap_or("ohne Nachricht")
                        .trim()
                        .to_string(),
                    timestamp,
                }
            });

        let mut opts = StatusOptions::new();
        opts.include_untracked(true)
            .include_ignored(false)
            .include_unmodified(false)
            .pathspec("ai");
        let statuses = repo.statuses(Some(&mut opts))?;
        let pending_ai_changes = statuses.iter().any(|entry| {
            entry
                .path()
                .map(|path| path.starts_with("ai"))
                .unwrap_or(false)
        });

        Ok(GitStatus {
            branch,
            head: head_commit,
            pending_ai_changes,
        })
    }

    fn ensure_gitignore(&self) -> Result<(), AppError> {
        let path = self.root().join(".gitignore");
        if path.exists() {
            return Ok(());
        }
        fs::write(
            &path,
            "/target\n/.env\n/node_modules\n/.idea\n/static/app.css\n/ai/cache\n",
        )?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct GitStatus {
    pub branch: String,
    pub head: Option<GitCommitInfo>,
    pub pending_ai_changes: bool,
}

#[derive(Debug, Clone)]
pub struct GitCommitInfo {
    pub hash: String,
    pub message: String,
    pub timestamp: String,
}
