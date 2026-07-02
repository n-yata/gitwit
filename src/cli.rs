use std::path::{Path, PathBuf};

use crate::git::GitError;

/// CLI引数から解決された起動対象。
pub struct CliTarget {
    pub repo_root: PathBuf,
    pub file_filter: Option<String>,
}

/// Explorer 等から渡されたファイル/フォルダパスを、リポジトリルートと
/// (ファイル指定時のみ)そのファイルの相対パスに解決する。
pub fn resolve_target(raw_path: &Path) -> Result<CliTarget, GitError> {
    let canonical = raw_path
        .canonicalize()
        .map_err(|e| GitError::NotARepository(format!("{}: {}", raw_path.display(), e)))?;

    let is_file = canonical.is_file();
    let search_start: &Path = if is_file {
        canonical.parent().unwrap_or(&canonical)
    } else {
        &canonical
    };

    let git_repo = git2::Repository::discover(search_start).map_err(GitError::from)?;
    let workdir = git_repo
        .workdir()
        .ok_or_else(|| {
            GitError::NotARepository("bareリポジトリには対応していません".to_string())
        })?
        .canonicalize()
        .map_err(|e| GitError::NotARepository(e.to_string()))?;

    let file_filter = if is_file {
        let relative = canonical.strip_prefix(&workdir).map_err(|_| {
            GitError::NotARepository(format!(
                "{}: リポジトリ({})の外にあるファイルです",
                canonical.display(),
                workdir.display()
            ))
        })?;
        Some(relative.to_string_lossy().replace('\\', "/"))
    } else {
        None
    };

    Ok(CliTarget {
        repo_root: workdir,
        file_filter,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn init_repo_with_commit(dir: &Path, file_name: &str) {
        let repo = git2::Repository::init(dir).unwrap();
        fs::write(dir.join(file_name), "content").unwrap();

        let mut index = repo.index().unwrap();
        index.add_path(Path::new(file_name)).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let sig = git2::Signature::now("tester", "tester@example.com").unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
            .unwrap();
    }

    #[test]
    fn resolve_target_with_file_computes_relative_path() {
        let tmp = std::env::temp_dir().join(format!("gitwit-cli-test-file-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join("src")).unwrap();
        init_repo_with_commit(&tmp, "src/main.rs");

        let target = resolve_target(&tmp.join("src").join("main.rs")).unwrap();

        assert_eq!(target.file_filter.as_deref(), Some("src/main.rs"));
        assert_eq!(
            target.repo_root.canonicalize().unwrap(),
            tmp.canonicalize().unwrap()
        );

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn resolve_target_with_folder_has_no_filter() {
        let tmp = std::env::temp_dir().join(format!("gitwit-cli-test-dir-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        init_repo_with_commit(&tmp, "readme.txt");

        let target = resolve_target(&tmp).unwrap();

        assert!(target.file_filter.is_none());

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn resolve_target_outside_repository_is_error() {
        let tmp = std::env::temp_dir().join(format!("gitwit-cli-test-norepo-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let result = resolve_target(&tmp);

        assert!(result.is_err());

        let _ = fs::remove_dir_all(&tmp);
    }
}
