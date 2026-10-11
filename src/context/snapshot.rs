use super::{gitlog, processes, tree};
use crate::config::Context;
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

impl ContextSnapshot {
    /// Built lazily: the offline path never pays for tree walks or subprocess probes.
    pub fn enrich(&self, config: &Context) -> EnrichedContext {
        let tree = config.include_fs.then(|| {
            tree::walk(
                &self.cwd,
                &tree::TreeBudget {
                    depth: config.max_depth,
                    files: config.max_files,
                },
            )
        });
        let git = if config.include_git {
            gitlog::collect(&self.cwd)
        } else {
            gitlog::GitDetail::default()
        };
        let processes = if config.include_kube || config.include_docker {
            processes::collect(&self.cwd)
        } else {
            processes::ProcessContext::default()
        };
        EnrichedContext {
            tree: tree.unwrap_or_default(),
            git,
            processes,
        }
    }
}

pub struct EnrichedContext {
    pub tree: Vec<String>,
    pub git: gitlog::GitDetail,
    pub processes: processes::ProcessContext,
}
