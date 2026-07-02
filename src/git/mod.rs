pub mod commit;
pub mod diff;
pub mod repository;

pub use commit::{format_relative_time, CommitInfo};
pub use diff::{DiffFile, DiffHunk, DiffLineKind, FileStatus};
pub use repository::GitRepository;

use std::fmt;

#[derive(Debug)]
pub enum GitError {
    Git2(git2::Error),
    NotARepository(String),
    EmptyRepository,
}

impl fmt::Display for GitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GitError::Git2(e) => write!(f, "{}", e.message()),
            GitError::NotARepository(path) => {
                write!(f, "Git リポジトリが見つかりません: {}", path)
            }
            GitError::EmptyRepository => write!(f, "コミットがありません"),
        }
    }
}

impl std::error::Error for GitError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            GitError::Git2(e) => Some(e),
            _ => None,
        }
    }
}

impl From<git2::Error> for GitError {
    fn from(e: git2::Error) -> Self {
        use git2::ErrorClass;
        match e.class() {
            ErrorClass::Repository | ErrorClass::Os => {
                GitError::NotARepository(e.message().to_string())
            }
            _ => GitError::Git2(e),
        }
    }
}
