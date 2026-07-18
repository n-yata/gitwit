use git2::Repository;

use super::GitError;

/// 全リモートに対して fetch を実行する。
/// 認証は SSH エージェント / 既存 Git 認証（credential helper）に委譲する。
pub(super) fn fetch_all_remotes(repo: &Repository) -> Result<(), GitError> {
    let remote_names = repo.remotes().map_err(GitError::Git2)?;

    for remote_name in remote_names.iter().flatten() {
        let mut remote = repo.find_remote(remote_name).map_err(GitError::Git2)?;

        let mut callbacks = git2::RemoteCallbacks::new();
        callbacks.credentials(|url, username_from_url, allowed_types| {
            if allowed_types.contains(git2::CredentialType::SSH_KEY) {
                let user = username_from_url.unwrap_or("git");
                if let Ok(cred) = git2::Cred::ssh_key_from_agent(user) {
                    return Ok(cred);
                }
            }
            if allowed_types.contains(git2::CredentialType::DEFAULT) {
                if let Ok(cred) = git2::Cred::default() {
                    return Ok(cred);
                }
            }
            if allowed_types.contains(git2::CredentialType::USER_PASS_PLAINTEXT) {
                let config = repo.config()?;
                if let Ok(cred) = git2::Cred::credential_helper(&config, url, username_from_url) {
                    return Ok(cred);
                }
            }
            Err(git2::Error::from_str("利用可能な認証方式がありません"))
        });

        let mut opts = git2::FetchOptions::new();
        opts.remote_callbacks(callbacks);
        opts.prune(git2::FetchPrune::On);

        let refspecs: Vec<String> = Vec::new();
        remote
            .fetch(&refspecs, Some(&mut opts), None)
            .map_err(GitError::Git2)?;
    }
    Ok(())
}

/// `refs/remotes/*` のブランチ名を列挙する（例: "origin/main"）。
/// HEAD シンボリック参照（例: "origin/HEAD"）は除外する。
pub(super) fn list_remote_branches(repo: &Repository) -> Result<Vec<String>, GitError> {
    let branches = repo
        .branches(Some(git2::BranchType::Remote))
        .map_err(GitError::Git2)?;

    let mut names = Vec::new();
    for branch in branches {
        let (branch, _) = branch.map_err(GitError::Git2)?;
        if let Some(name) = branch.name().map_err(GitError::Git2)? {
            if name.ends_with("/HEAD") {
                continue;
            }
            names.push(name.to_string());
        }
    }
    names.sort();
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    fn init_dir(name: &str) -> PathBuf {
        let tmp = std::env::temp_dir().join(format!(
            "gitwit-remote-test-{}-{}",
            std::process::id(),
            name
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        tmp
    }

    fn commit_file(repo: &Repository, path: &str, content: &str, message: &str) {
        let full_path = repo.workdir().unwrap().join(path);
        std::fs::create_dir_all(full_path.parent().unwrap()).unwrap();
        std::fs::write(&full_path, content).unwrap();

        let mut index = repo.index().unwrap();
        index.add_path(Path::new(path)).unwrap();
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

    fn file_url(path: &Path) -> String {
        format!("file:///{}", path.to_string_lossy().replace('\\', "/"))
    }

    /// リモート役（実体を持つ通常リポジトリ）とローカル役（origin を張って clone 相当の
    /// remote 設定をした空リポジトリ）を作り、ローカル役を返す。
    fn setup_remote_and_local(name: &str) -> (PathBuf, PathBuf) {
        let remote_dir = init_dir(&format!("{}-remote", name));
        let remote_repo = Repository::init(&remote_dir).unwrap();
        commit_file(&remote_repo, "a.txt", "1", "initial");

        let local_dir = init_dir(&format!("{}-local", name));
        let local_repo = Repository::init(&local_dir).unwrap();
        local_repo.remote("origin", &file_url(&remote_dir)).unwrap();

        (remote_dir, local_dir)
    }

    #[test]
    fn fetch_and_list_remote_branches_reflects_remote_default_branch() {
        let (_remote_dir, local_dir) = setup_remote_and_local("fetch-basic");
        let local_repo = Repository::open(&local_dir).unwrap();

        fetch_all_remotes(&local_repo).unwrap();
        let names = list_remote_branches(&local_repo).unwrap();

        assert!(
            names.iter().any(|n| n.starts_with("origin/")),
            "expected an origin/* branch, got: {:?}",
            names
        );

        let _ = std::fs::remove_dir_all(&local_dir);
    }

    #[test]
    fn fetch_picks_up_newly_created_remote_branch() {
        let (remote_dir, local_dir) = setup_remote_and_local("fetch-new-branch");
        let local_repo = Repository::open(&local_dir).unwrap();

        let remote_repo = Repository::open(&remote_dir).unwrap();
        let head_commit = remote_repo.head().unwrap().peel_to_commit().unwrap();
        remote_repo
            .branch("feature-x", &head_commit, false)
            .unwrap();

        fetch_all_remotes(&local_repo).unwrap();
        let names = list_remote_branches(&local_repo).unwrap();

        assert!(
            names.iter().any(|n| n == "origin/feature-x"),
            "expected origin/feature-x in {:?}",
            names
        );

        let _ = std::fs::remove_dir_all(&local_dir);
    }

    #[test]
    fn list_remote_branches_excludes_symbolic_head() {
        let (_remote_dir, local_dir) = setup_remote_and_local("exclude-head");
        let local_repo = Repository::open(&local_dir).unwrap();

        fetch_all_remotes(&local_repo).unwrap();
        let names = list_remote_branches(&local_repo).unwrap();

        assert!(!names.iter().any(|n| n.ends_with("/HEAD")));

        let _ = std::fs::remove_dir_all(&local_dir);
    }

    #[test]
    fn fetch_all_remotes_returns_err_for_unreachable_remote() {
        let local_dir = init_dir("fetch-unreachable-local");
        let local_repo = Repository::init(&local_dir).unwrap();
        let missing_remote = init_dir("fetch-unreachable-missing").join("does-not-exist-as-a-repo");
        local_repo
            .remote("origin", &file_url(&missing_remote))
            .unwrap();

        let result = fetch_all_remotes(&local_repo);

        assert!(
            result.is_err(),
            "expected fetch against a nonexistent local path to fail"
        );

        let _ = std::fs::remove_dir_all(&local_dir);
    }
}
