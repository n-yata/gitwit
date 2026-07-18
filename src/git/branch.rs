use git2::Repository;

use super::GitError;

pub(super) fn list_local_branches(repo: &Repository) -> Result<Vec<String>, GitError> {
    let branches = repo
        .branches(Some(git2::BranchType::Local))
        .map_err(GitError::Git2)?;

    let mut names = Vec::new();
    for branch in branches {
        let (branch, _) = branch.map_err(GitError::Git2)?;
        if let Some(name) = branch.name().map_err(GitError::Git2)? {
            names.push(name.to_string());
        }
    }
    names.sort();
    Ok(names)
}

pub(super) fn current_branch_name(repo: &Repository) -> Result<Option<String>, GitError> {
    let head = match repo.head() {
        Ok(head) => head,
        Err(e) if e.code() == git2::ErrorCode::UnbornBranch => return Ok(None),
        Err(e) => return Err(GitError::Git2(e)),
    };

    if !head.is_branch() {
        return Ok(None);
    }

    Ok(head.shorthand().map(|s| s.to_string()))
}

pub(super) fn checkout_branch(repo: &Repository, branch_name: &str) -> Result<(), GitError> {
    let refname = format!("refs/heads/{}", branch_name);
    let obj = repo.revparse_single(&refname).map_err(GitError::Git2)?;

    let mut checkout_builder = git2::build::CheckoutBuilder::new();
    checkout_builder.safe();

    repo.checkout_tree(&obj, Some(&mut checkout_builder))
        .map_err(|e| {
            if e.code() == git2::ErrorCode::Conflict {
                GitError::CheckoutConflict
            } else {
                GitError::Git2(e)
            }
        })?;

    repo.set_head(&refname).map_err(GitError::Git2)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn init_repo(name: &str) -> std::path::PathBuf {
        let tmp = std::env::temp_dir().join(format!(
            "gitwit-branch-test-{}-{}",
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

    #[test]
    fn list_local_branches_returns_names_sorted() {
        let tmp = init_repo("list-sorted");
        let repo = Repository::init(&tmp).unwrap();
        commit_file(&repo, "a.txt", "1", "initial");
        repo.branch(
            "zeta",
            &repo.head().unwrap().peel_to_commit().unwrap(),
            false,
        )
        .unwrap();
        repo.branch(
            "alpha",
            &repo.head().unwrap().peel_to_commit().unwrap(),
            false,
        )
        .unwrap();

        let names = list_local_branches(&repo).unwrap();

        // 初期ブランチ名は環境依存(main/master)のため、作成した2ブランチが正しい順で
        // 含まれていることのみを検証する。
        let alpha_idx = names.iter().position(|n| n == "alpha").unwrap();
        let zeta_idx = names.iter().position(|n| n == "zeta").unwrap();
        assert!(alpha_idx < zeta_idx);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn current_branch_name_returns_branch_on_normal_head() {
        let tmp = init_repo("current-normal");
        let repo = Repository::init(&tmp).unwrap();
        commit_file(&repo, "a.txt", "1", "initial");
        repo.branch(
            "feature-x",
            &repo.head().unwrap().peel_to_commit().unwrap(),
            false,
        )
        .unwrap();
        repo.set_head("refs/heads/feature-x").unwrap();

        let name = current_branch_name(&repo).unwrap();

        assert_eq!(name.as_deref(), Some("feature-x"));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn current_branch_name_returns_none_on_detached_head() {
        let tmp = init_repo("current-detached");
        let repo = Repository::init(&tmp).unwrap();
        commit_file(&repo, "a.txt", "1", "initial");
        let oid = repo.head().unwrap().peel_to_commit().unwrap().id();
        repo.set_head_detached(oid).unwrap();

        let name = current_branch_name(&repo).unwrap();

        assert_eq!(name, None);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn checkout_branch_switches_head_to_target() {
        let tmp = init_repo("checkout-success");
        let repo = Repository::init(&tmp).unwrap();
        commit_file(&repo, "a.txt", "1", "initial");
        let base_commit = repo.head().unwrap().peel_to_commit().unwrap();
        repo.branch("feature-y", &base_commit, false).unwrap();
        commit_file(&repo, "a.txt", "2", "on original branch");

        checkout_branch(&repo, "feature-y").unwrap();

        let head = repo.head().unwrap();
        assert_eq!(head.shorthand(), Some("feature-y"));
        let content = std::fs::read_to_string(tmp.join("a.txt")).unwrap();
        assert_eq!(content, "1");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn checkout_branch_returns_conflict_and_keeps_head_on_uncommitted_conflict() {
        let tmp = init_repo("checkout-conflict");
        let repo = Repository::init(&tmp).unwrap();
        commit_file(&repo, "a.txt", "base", "initial");
        let base_commit = repo.head().unwrap().peel_to_commit().unwrap();
        repo.branch("feature-z", &base_commit, false).unwrap();
        commit_file(&repo, "a.txt", "on-original", "diverge on original branch");

        // switch to feature-z branch's content differs from working tree's uncommitted edit
        std::fs::write(tmp.join("a.txt"), "uncommitted-local-edit").unwrap();

        let original_head_name = repo.head().unwrap().shorthand().unwrap().to_string();
        let result = checkout_branch(&repo, "feature-z");

        assert!(matches!(result, Err(GitError::CheckoutConflict)));
        assert_eq!(
            repo.head().unwrap().shorthand(),
            Some(original_head_name.as_str())
        );
        let content = std::fs::read_to_string(tmp.join("a.txt")).unwrap();
        assert_eq!(content, "uncommitted-local-edit");

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
