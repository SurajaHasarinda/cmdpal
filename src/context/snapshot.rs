use serde::Serialize;
use std::{collections::HashSet, path::PathBuf};

#[derive(Serialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum ShellType {
    Bash,
    Zsh,
    Fish,
    Pwsh,
    Unknown,
}

#[derive(Serialize, Clone, Default)]
pub struct GitContext {
    pub branch: Option<String>,
    pub changes: usize,
    pub ahead: u32,
    pub behind: u32,
    pub remotes: Vec<String>,
    pub branches: Vec<String>,
}

#[derive(Serialize, Clone)]
pub struct KubeContext {
    pub context: Option<String>,
    pub namespace: Option<String>,
    pub cluster: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct ContextSnapshot {
    pub shell: ShellType,
    pub cwd: PathBuf,
    pub cwd_entries: Vec<String>,
    pub git: Option<GitContext>,
    pub kube: Option<KubeContext>,
    pub tags: HashSet<String>,
}
