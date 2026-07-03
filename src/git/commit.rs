use std::collections::HashMap;

use chrono::Utc;
use git2::Repository;

use super::GitError;

#[derive(Clone, Debug)]
pub struct CommitInfo {
    pub oid: String,
    pub short_id: String,
    pub message: String,
    pub author: String,
    pub time: i64,
    pub refs: Vec<String>,
}

pub(super) fn load_commits(repo: &Repository, limit: usize) -> Result<Vec<CommitInfo>, GitError> {
    let refs = collect_refs(repo);

    let mut revwalk = repo.revwalk().map_err(GitError::Git2)?;
    revwalk.push_head().map_err(|e| {
        if e.code() == git2::ErrorCode::UnbornBranch {
            GitError::EmptyRepository
        } else {
            GitError::Git2(e)
        }
    })?;
    revwalk
        .set_sorting(git2::Sort::TIME)
        .map_err(GitError::Git2)?;

    let mut commits = Vec::new();
    for oid in revwalk.take(limit) {
        let oid = oid.map_err(GitError::Git2)?;
        commits.push(build_commit_info(repo, oid, &refs)?);
    }

    Ok(commits)
}

/// 指定パスが変更されたコミットのみを対象にコミット一覧を構築する。
///
/// `limit` は「マッチしたコミット数」の上限であり、走査するコミット総数の上限ではない。
pub(super) fn load_commits_for_path(
    repo: &Repository,
    limit: usize,
    path: &str,
) -> Result<Vec<CommitInfo>, GitError> {
    let refs = collect_refs(repo);

    let mut revwalk = repo.revwalk().map_err(GitError::Git2)?;
    revwalk.push_head().map_err(|e| {
        if e.code() == git2::ErrorCode::UnbornBranch {
            GitError::EmptyRepository
        } else {
            GitError::Git2(e)
        }
    })?;
    revwalk
        .set_sorting(git2::Sort::TIME)
        .map_err(GitError::Git2)?;

    let mut commits = Vec::new();
    for oid in revwalk {
        if commits.len() >= limit {
            break;
        }
        let oid = oid.map_err(GitError::Git2)?;
        let commit = repo.find_commit(oid).map_err(GitError::Git2)?;

        if !commit_touches_path(repo, &commit, path)? {
            continue;
        }

        commits.push(build_commit_info(repo, oid, &refs)?);
    }

    Ok(commits)
}

fn build_commit_info(
    repo: &Repository,
    oid: git2::Oid,
    refs: &HashMap<git2::Oid, Vec<String>>,
) -> Result<CommitInfo, GitError> {
    let commit = repo.find_commit(oid).map_err(GitError::Git2)?;

    let oid_str = oid.to_string();
    let short_id = oid_str[..7].to_string();
    let message = commit
        .summary()
        .unwrap_or("(メッセージなし)")
        .to_string();
    let author = commit.author().name().unwrap_or("Unknown").to_string();
    let time = commit.time().seconds();
    let commit_refs = refs.get(&oid).cloned().unwrap_or_default();

    Ok(CommitInfo {
        oid: oid_str,
        short_id,
        message,
        author,
        time,
        refs: commit_refs,
    })
}

fn commit_touches_path(
    repo: &Repository,
    commit: &git2::Commit,
    path: &str,
) -> Result<bool, GitError> {
    let new_tree = commit.tree().map_err(GitError::Git2)?;
    let old_tree = commit.parent(0).ok().and_then(|p| p.tree().ok());

    let mut opts = git2::DiffOptions::new();
    opts.pathspec(path);

    let diff = repo
        .diff_tree_to_tree(old_tree.as_ref(), Some(&new_tree), Some(&mut opts))
        .map_err(GitError::Git2)?;

    #[allow(clippy::len_zero)] // git2::Deltas の is_empty() は unstable feature (exact_size_is_empty)
    Ok(diff.deltas().len() > 0)
}

fn collect_refs(repo: &Repository) -> HashMap<git2::Oid, Vec<String>> {
    let mut result: HashMap<git2::Oid, Vec<String>> = HashMap::new();
    let Ok(references) = repo.references() else {
        return result;
    };
    for reference in references.flatten() {
        let Some(target) = reference.resolve().ok().and_then(|r| r.target()) else {
            continue;
        };
        let name = reference
            .shorthand()
            .unwrap_or_else(|| reference.name().unwrap_or(""))
            .to_string();
        if !name.is_empty() {
            result.entry(target).or_default().push(name);
        }
    }
    result
}

pub fn format_relative_time(unix_time: i64) -> String {
    let now = Utc::now().timestamp();
    let diff = now - unix_time;

    if diff < 60 {
        "たった今".to_string()
    } else if diff < 60 * 60 {
        format!("{}分前", diff / 60)
    } else if diff < 60 * 60 * 24 {
        format!("{}時間前", diff / (60 * 60))
    } else if diff < 60 * 60 * 24 * 30 {
        format!("{}日前", diff / (60 * 60 * 24))
    } else {
        let dt = chrono::DateTime::from_timestamp(unix_time, 0)
            .unwrap_or_default()
            .with_timezone(&chrono::Local);
        dt.format("%Y-%m-%d").to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_relative_time_just_now() {
        let now = Utc::now().timestamp();
        assert_eq!(format_relative_time(now - 30), "たった今");
    }

    #[test]
    fn format_relative_time_minutes() {
        let now = Utc::now().timestamp();
        assert_eq!(format_relative_time(now - 20 * 60), "20分前");
    }

    #[test]
    fn format_relative_time_hours() {
        let now = Utc::now().timestamp();
        assert_eq!(format_relative_time(now - 5 * 60 * 60), "5時間前");
    }

    #[test]
    fn format_relative_time_days() {
        let now = Utc::now().timestamp();
        assert_eq!(format_relative_time(now - 3 * 24 * 60 * 60), "3日前");
    }

    #[test]
    fn format_relative_time_old_date() {
        // 60日前は YYYY-MM-DD 形式
        let now = Utc::now().timestamp();
        let result = format_relative_time(now - 60 * 24 * 60 * 60);
        assert!(result.contains('-'), "Should be date format, got: {}", result);
    }

    fn commit_file(repo: &Repository, path: &str, message: &str) {
        let full_path = repo.workdir().unwrap().join(path);
        std::fs::create_dir_all(full_path.parent().unwrap()).unwrap();
        std::fs::write(&full_path, message).unwrap();

        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new(path)).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let sig = git2::Signature::now("tester", "tester@example.com").unwrap();
        let parents: Vec<git2::Commit> = repo
            .head()
            .ok()
            .and_then(|h| h.peel_to_commit().ok())
            .into_iter()
            .collect();
        let parent_refs: Vec<&git2::Commit> = parents.iter().collect();
        repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parent_refs)
            .unwrap();
    }

    #[test]
    fn load_commits_for_path_filters_by_directory() {
        let tmp = std::env::temp_dir().join(format!(
            "gitwit-commit-test-dir-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let repo = Repository::init(&tmp).unwrap();

        commit_file(&repo, "src/main.rs", "add main.rs");
        commit_file(&repo, "docs/readme.md", "add readme");
        commit_file(&repo, "src/lib.rs", "add lib.rs");

        let commits = load_commits_for_path(&repo, 10, "src").unwrap();

        assert_eq!(commits.len(), 2);
        assert!(commits.iter().all(|c| c.message != "add readme"));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn load_commits_for_path_filters_by_file() {
        let tmp = std::env::temp_dir().join(format!(
            "gitwit-commit-test-file-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let repo = Repository::init(&tmp).unwrap();

        commit_file(&repo, "src/main.rs", "add main.rs");
        commit_file(&repo, "src/lib.rs", "add lib.rs");
        commit_file(&repo, "src/main.rs", "update main.rs");

        let commits = load_commits_for_path(&repo, 10, "src/main.rs").unwrap();

        assert_eq!(commits.len(), 2);
        assert!(commits.iter().all(|c| c.message != "add lib.rs"));

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
