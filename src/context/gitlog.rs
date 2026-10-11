use git2::{Repository, StatusOptions};
use std::path::Path;

const MAX_COMMITS: usize = 15;
const MAX_CHANGES: usize = 40;

#[derive(Default)]
pub struct GitDetail {
    pub commits: Vec<String>,
    pub changed_paths: Vec<String>,
    pub is_repo: bool,
}

pub fn collect(cwd: &Path) -> GitDetail {
    let Ok(repo) = Repository::discover(cwd) else {
        return GitDetail::default();
    };
    GitDetail {
        commits: commits(&repo),
        changed_paths: changed_paths(&repo),
        is_repo: true,
    }
}

fn commits(repo: &Repository) -> Vec<String> {
    let Ok(mut walk) = repo.revwalk() else {
        return Vec::new();
    };
    if walk.push_head().is_err() {
        return Vec::new();
    }
    walk.filter_map(Result::ok)
        .take(MAX_COMMITS)
        .filter_map(|oid| {
            let commit = repo.find_commit(oid).ok()?;
            let summary = commit.summary().unwrap_or("").replace(['\n', '\r'], " ");
            Some(format!("{} {}", &commit.id().to_string()[..7], summary))
        })
        .collect()
}

fn changed_paths(repo: &Repository) -> Vec<String> {
    let mut options = StatusOptions::new();
    options.include_untracked(true);
    let Ok(statuses) = repo.statuses(Some(&mut options)) else {
        return Vec::new();
    };
    statuses
        .iter()
        .take(MAX_CHANGES)
        .filter_map(|entry| entry.path().map(String::from))
        .map(|path| path.replace(['\n', '\r'], " "))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo_with_history() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        let sig = git2::Signature::now("t", "t@e.com").unwrap();
        let mut parent: Option<git2::Oid> = None;
        for index in 0..3 {
            let path = format!("f{index}.txt");
            std::fs::write(dir.path().join(&path), "x").unwrap();
            let mut index_tree = repo.index().unwrap();
            index_tree.add_path(Path::new(&path)).unwrap();
            index_tree.write().unwrap();
            let tree = repo.find_tree(index_tree.write_tree().unwrap()).unwrap();
            let previous: Vec<git2::Commit> = parent
                .iter()
                .filter_map(|oid| repo.find_commit(*oid).ok())
                .collect();
            let refs: Vec<&git2::Commit> = previous.iter().collect();
            parent = Some(
                repo.commit(
                    Some("HEAD"),
                    &sig,
                    &sig,
                    &format!("commit {index}"),
                    &tree,
                    &refs,
                )
                .unwrap(),
            );
        }
        std::fs::write(dir.path().join("dirty.txt"), "y").unwrap();
        dir
    }

    #[test]
    fn collects_commits_and_dirty_paths() {
        let dir = repo_with_history();
        let detail = collect(dir.path());
        assert!(detail.is_repo);
        assert_eq!(detail.commits.len(), 3);
        assert!(detail.commits[0].contains("commit 2"), "newest first");
        assert!(detail.changed_paths.contains(&"dirty.txt".to_string()));
    }

    #[test]
    fn outside_a_repo_is_not_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let detail = collect(dir.path());
        assert!(!detail.is_repo);
        assert!(detail.commits.is_empty());
    }
}
