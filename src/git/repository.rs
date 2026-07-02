use std::path::Path;

use super::{
    commit::{load_commits, CommitInfo},
    diff::{load_diff_files, load_diff_hunks, DiffFile, DiffHunk},
    GitError,
};

pub struct GitRepository {
    inner: git2::Repository,
}

impl GitRepository {
    pub fn open(path: &Path) -> Result<Self, GitError> {
        let repo = git2::Repository::open(path).map_err(|e| {
            GitError::NotARepository(path.to_string_lossy().to_string() + ": " + e.message())
        })?;
        Ok(Self { inner: repo })
    }

    pub fn load_commits(&self, limit: usize) -> Result<Vec<CommitInfo>, GitError> {
        load_commits(&self.inner, limit)
    }

    pub fn load_diff_files(&self, oid_str: &str) -> Result<Vec<DiffFile>, GitError> {
        load_diff_files(&self.inner, oid_str)
    }

    pub fn load_diff_hunks(&self, oid_str: &str, file_path: &str) -> Result<Vec<DiffHunk>, GitError> {
        load_diff_hunks(&self.inner, oid_str, file_path)
    }
}
