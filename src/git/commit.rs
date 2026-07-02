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

        commits.push(CommitInfo {
            oid: oid_str,
            short_id,
            message,
            author,
            time,
            refs: commit_refs,
        });
    }

    Ok(commits)
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
}
