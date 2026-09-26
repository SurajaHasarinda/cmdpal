use super::snapshot::{GitContext, KubeContext, ShellType};
use git2::{Branch, BranchType, Repository, StatusOptions};
use serde::Deserialize;
use std::{
    env, fs,
    path::{Path, PathBuf},
};

const MAX_ENTRIES: usize = 100;
const MAX_BRANCHES: usize = 20;

pub fn detect_shell() -> ShellType {
    // PSModulePath is set system-wide on Windows, so it only signals "a PowerShell-family host is likely";
    // cmd.exe is out of scope for v1.
    if cfg!(windows) {
        return if env::var_os("PSModulePath").is_some() {
            ShellType::Pwsh
        } else {
            ShellType::Unknown
        };
    }
    let shell = env::var("SHELL").unwrap_or_default();
    match Path::new(&shell).file_name().and_then(|name| name.to_str()) {
        Some("bash") => ShellType::Bash,
        Some("zsh") => ShellType::Zsh,
        Some("fish") => ShellType::Fish,
        _ => ShellType::Unknown,
    }
}

pub fn list_entries(cwd: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(cwd) else {
        return Vec::new();
    };
    entries
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| !name.starts_with('.'))
        .take(MAX_ENTRIES)
        .collect()
}

pub fn detect_git(cwd: &Path) -> Option<GitContext> {
    let repo = Repository::discover(cwd).ok()?;
    let (ahead, behind) = ahead_behind(&repo).unwrap_or_default();
    Some(GitContext {
        branch: repo
            .head()
            .ok()
            .and_then(|head| head.shorthand().map(String::from)),
        changes: count_changes(&repo),
        ahead,
        behind,
        remotes: repo
            .remotes()
            .ok()?
            .iter()
            .flatten()
            .map(String::from)
            .collect(),
        branches: local_branches(&repo),
    })
}

fn count_changes(repo: &Repository) -> usize {
    let mut options = StatusOptions::new();
    options.include_untracked(true);
    repo.statuses(Some(&mut options))
        .map_or(0, |statuses| statuses.len())
}

fn ahead_behind(repo: &Repository) -> Option<(u32, u32)> {
    let head = repo.head().ok()?;
    let local = head.target()?;
    let upstream = Branch::wrap(head).upstream().ok()?;
    let remote = upstream.get().target()?;
    let (ahead, behind) = repo.graph_ahead_behind(local, remote).ok()?;
    Some((ahead as u32, behind as u32))
}

fn local_branches(repo: &Repository) -> Vec<String> {
    let Ok(branches) = repo.branches(Some(BranchType::Local)) else {
        return Vec::new();
    };
    branches
        .flatten()
        .filter_map(|(branch, _)| branch.name().ok().flatten().map(String::from))
        .take(MAX_BRANCHES)
        .collect()
}

#[derive(Deserialize)]
struct KubeConfig {
    #[serde(rename = "current-context")]
    current_context: Option<String>,
    #[serde(default)]
    contexts: Vec<NamedContext>,
}

#[derive(Deserialize)]
struct NamedContext {
    name: String,
    context: ContextEntry,
}

#[derive(Deserialize)]
struct ContextEntry {
    cluster: Option<String>,
    namespace: Option<String>,
}

pub fn detect_kube() -> Option<KubeContext> {
    let text = fs::read_to_string(kubeconfig_path()?).ok()?;
    let config: KubeConfig = serde_yaml::from_str(&text).ok()?;
    let current = config.current_context?;
    let entry = config
        .contexts
        .into_iter()
        .find(|named| named.name == current)?
        .context;
    Some(KubeContext {
        context: Some(current),
        namespace: entry.namespace,
        cluster: entry.cluster,
    })
}

fn kubeconfig_path() -> Option<PathBuf> {
    match env::var_os("KUBECONFIG") {
        Some(value) => env::split_paths(&value).next(),
        None => dirs::home_dir().map(|home| home.join(".kube").join("config")),
    }
}
