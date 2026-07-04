use git2::{Repository, Tree};

use super::GitError;

#[derive(Clone, Debug)]
pub enum FileStatus {
    Added,
    Modified,
    Deleted,
    Renamed { old_path: String },
}

#[derive(Clone, Debug)]
pub struct DiffFile {
    pub path: String,
    pub status: FileStatus,
    pub is_binary: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiffLineKind {
    Added,
    Deleted,
    Context,
}

#[derive(Clone, Debug)]
pub struct DiffLine {
    pub kind: DiffLineKind,
    pub content: String,
}

#[derive(Clone, Debug)]
pub struct DiffHunk {
    pub header: String,
    pub lines: Vec<DiffLine>,
}

fn commit_tree<'a>(repo: &'a Repository, oid_str: &str) -> Result<Tree<'a>, GitError> {
    let oid = git2::Oid::from_str(oid_str).map_err(GitError::Git2)?;
    let commit = repo.find_commit(oid).map_err(GitError::Git2)?;
    commit.tree().map_err(GitError::Git2)
}

fn parent_tree<'a>(repo: &'a Repository, oid_str: &str) -> Result<Option<Tree<'a>>, GitError> {
    let oid = git2::Oid::from_str(oid_str).map_err(GitError::Git2)?;
    let commit = repo.find_commit(oid).map_err(GitError::Git2)?;
    Ok(commit.parent(0).ok().and_then(|p| p.tree().ok()))
}

fn diff_tree_to_files(
    repo: &Repository,
    old_tree: Option<&Tree>,
    new_tree: &Tree,
) -> Result<Vec<DiffFile>, GitError> {
    let diff = repo
        .diff_tree_to_tree(old_tree, Some(new_tree), None)
        .map_err(GitError::Git2)?;

    let mut files: Vec<DiffFile> = Vec::new();
    diff.foreach(
        &mut |delta, _progress| {
            let is_binary = delta.new_file().is_binary() || delta.old_file().is_binary();
            let status = match delta.status() {
                git2::Delta::Added => FileStatus::Added,
                git2::Delta::Deleted => FileStatus::Deleted,
                git2::Delta::Renamed => {
                    let old_path = delta
                        .old_file()
                        .path()
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_default();
                    FileStatus::Renamed { old_path }
                }
                _ => FileStatus::Modified,
            };
            let path = delta
                .new_file()
                .path()
                .or_else(|| delta.old_file().path())
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
            files.push(DiffFile {
                path,
                status,
                is_binary,
            });
            true
        },
        None,
        None,
        None,
    )
    .map_err(GitError::Git2)?;

    Ok(files)
}

fn diff_tree_to_hunks(
    repo: &Repository,
    old_tree: Option<&Tree>,
    new_tree: &Tree,
    file_path: &str,
) -> Result<Vec<DiffHunk>, GitError> {
    let mut opts = git2::DiffOptions::new();
    opts.pathspec(file_path);

    let diff = repo
        .diff_tree_to_tree(old_tree, Some(new_tree), Some(&mut opts))
        .map_err(GitError::Git2)?;

    #[allow(clippy::len_zero)]
    // git2::Deltas の is_empty() は unstable feature (exact_size_is_empty)
    if diff.deltas().len() == 0 {
        return Ok(Vec::new());
    }

    let patch = git2::Patch::from_diff(&diff, 0).map_err(GitError::Git2)?;
    let Some(patch) = patch else {
        return Ok(Vec::new());
    };

    let mut hunks: Vec<DiffHunk> = Vec::new();
    for hunk_idx in 0..patch.num_hunks() {
        let (hunk, line_count) = patch.hunk(hunk_idx).map_err(GitError::Git2)?;
        let header = String::from_utf8_lossy(hunk.header())
            .trim_end()
            .to_string();
        let mut diff_lines: Vec<DiffLine> = Vec::new();

        for line_idx in 0..line_count {
            let line = patch
                .line_in_hunk(hunk_idx, line_idx)
                .map_err(GitError::Git2)?;
            let content = String::from_utf8_lossy(line.content())
                .trim_end_matches('\n')
                .trim_end_matches('\r')
                .to_string();
            let kind = match line.origin() {
                '+' => DiffLineKind::Added,
                '-' => DiffLineKind::Deleted,
                _ => DiffLineKind::Context,
            };
            diff_lines.push(DiffLine { kind, content });
        }

        hunks.push(DiffHunk {
            header,
            lines: diff_lines,
        });
    }

    Ok(hunks)
}

pub(super) fn load_diff_files(repo: &Repository, oid_str: &str) -> Result<Vec<DiffFile>, GitError> {
    let new_tree = commit_tree(repo, oid_str)?;
    let old_tree = parent_tree(repo, oid_str)?;
    diff_tree_to_files(repo, old_tree.as_ref(), &new_tree)
}

pub(super) fn load_diff_hunks(
    repo: &Repository,
    oid_str: &str,
    file_path: &str,
) -> Result<Vec<DiffHunk>, GitError> {
    let new_tree = commit_tree(repo, oid_str)?;
    let old_tree = parent_tree(repo, oid_str)?;
    diff_tree_to_hunks(repo, old_tree.as_ref(), &new_tree, file_path)
}

pub(super) fn load_diff_files_between(
    repo: &Repository,
    base_oid_str: &str,
    target_oid_str: &str,
) -> Result<Vec<DiffFile>, GitError> {
    let old_tree = commit_tree(repo, base_oid_str)?;
    let new_tree = commit_tree(repo, target_oid_str)?;
    diff_tree_to_files(repo, Some(&old_tree), &new_tree)
}

pub(super) fn load_diff_hunks_between(
    repo: &Repository,
    base_oid_str: &str,
    target_oid_str: &str,
    file_path: &str,
) -> Result<Vec<DiffHunk>, GitError> {
    let old_tree = commit_tree(repo, base_oid_str)?;
    let new_tree = commit_tree(repo, target_oid_str)?;
    diff_tree_to_hunks(repo, Some(&old_tree), &new_tree, file_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn commit_file(repo: &Repository, path: &str, content: &str, message: &str) -> String {
        let full_path = repo.workdir().unwrap().join(path);
        std::fs::create_dir_all(full_path.parent().unwrap()).unwrap();
        std::fs::write(&full_path, content).unwrap();

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
        let oid = repo
            .commit(Some("HEAD"), &sig, &sig, message, &tree, &parent_refs)
            .unwrap();
        oid.to_string()
    }

    fn init_repo(name: &str) -> (std::path::PathBuf, Repository) {
        let tmp =
            std::env::temp_dir().join(format!("gitwit-diff-test-{}-{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let repo = Repository::init(&tmp).unwrap();
        (tmp, repo)
    }

    #[test]
    fn load_diff_files_between_returns_cumulative_changes() {
        let (tmp, repo) = init_repo("files-between");

        let c1 = commit_file(&repo, "a.txt", "1", "add a");
        commit_file(&repo, "a.txt", "2", "update a");
        let c3 = commit_file(&repo, "b.txt", "1", "add b");

        let files = load_diff_files_between(&repo, &c1, &c3).unwrap();
        let mut paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
        paths.sort();

        assert_eq!(paths, vec!["a.txt", "b.txt"]);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn load_diff_hunks_between_returns_accumulated_line_changes() {
        let (tmp, repo) = init_repo("hunks-between");

        let c1 = commit_file(&repo, "a.txt", "line1\n", "add a");
        commit_file(&repo, "a.txt", "line1\nline2\n", "update a to line2");
        let c3 = commit_file(&repo, "a.txt", "line1\nline2\nline3\n", "update a to line3");

        let hunks = load_diff_hunks_between(&repo, &c1, &c3, "a.txt").unwrap();
        let added_lines: Vec<&str> = hunks
            .iter()
            .flat_map(|h| &h.lines)
            .filter(|l| l.kind == DiffLineKind::Added)
            .map(|l| l.content.as_str())
            .collect();

        assert_eq!(added_lines, vec!["line2", "line3"]);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn load_diff_files_matches_parent_diff_when_base_is_parent() {
        let (tmp, repo) = init_repo("files-parent-equivalence");

        commit_file(&repo, "a.txt", "1", "add a");
        let c2 = commit_file(&repo, "a.txt", "2", "update a");

        let single = load_diff_files(&repo, &c2).unwrap();
        let c1 = repo
            .find_commit(git2::Oid::from_str(&c2).unwrap())
            .unwrap()
            .parent(0)
            .unwrap()
            .id()
            .to_string();
        let between = load_diff_files_between(&repo, &c1, &c2).unwrap();

        assert_eq!(single.len(), between.len());
        assert_eq!(single[0].path, between[0].path);

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
