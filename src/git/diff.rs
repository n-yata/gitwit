use git2::Repository;

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

pub(super) fn load_diff_files(repo: &Repository, oid_str: &str) -> Result<Vec<DiffFile>, GitError> {
    let oid = git2::Oid::from_str(oid_str).map_err(GitError::Git2)?;
    let commit = repo.find_commit(oid).map_err(GitError::Git2)?;
    let new_tree = commit.tree().map_err(GitError::Git2)?;
    let old_tree = commit
        .parent(0)
        .ok()
        .and_then(|p| p.tree().ok());

    let diff = repo
        .diff_tree_to_tree(old_tree.as_ref(), Some(&new_tree), None)
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

pub(super) fn load_diff_hunks(
    repo: &Repository,
    oid_str: &str,
    file_path: &str,
) -> Result<Vec<DiffHunk>, GitError> {
    let oid = git2::Oid::from_str(oid_str).map_err(GitError::Git2)?;
    let commit = repo.find_commit(oid).map_err(GitError::Git2)?;
    let new_tree = commit.tree().map_err(GitError::Git2)?;
    let old_tree = commit
        .parent(0)
        .ok()
        .and_then(|p| p.tree().ok());

    let mut opts = git2::DiffOptions::new();
    opts.pathspec(file_path);

    let diff = repo
        .diff_tree_to_tree(old_tree.as_ref(), Some(&new_tree), Some(&mut opts))
        .map_err(GitError::Git2)?;

    #[allow(clippy::len_zero)] // git2::Deltas の is_empty() は unstable feature (exact_size_is_empty)
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
