//! Ground-truth ranking report: `cargo test accuracy -- --nocapture`.

use super::Searcher;
use crate::{
    context::{ContextSnapshot, ShellType},
    knowledge::load_builtin,
    modes::ActiveMode,
};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::LazyLock,
};

// (mode id, query, expected command id)
const CASES: &[(&str, &str, &str)] = &[
    // ---- linux ----
    ("linux", "delete directory", "rm-rf"),
    ("linux", "remove folder", "rm-rf"),
    ("linux", "list files in directory", "ls-list"),
    ("linux", "show hidden files", "ls-list"),
    ("linux", "show directory tree", "ls-tree"),
    ("linux", "create directory", "mkdir-new"),
    ("linux", "new folder", "mkdir-new"),
    ("linux", "delete file", "rm-file"),
    ("linux", "copy file", "cp-file"),
    ("linux", "move file", "mv-move"),
    ("linux", "rename file", "mv-rename"),
    ("linux", "go to parent directory", "cd-up"),
    ("linux", "find file by name", "find-name"),
    ("linux", "find large files", "find-large"),
    ("linux", "find recently modified files", "find-recent"),
    ("linux", "check directory size", "du-size"),
    ("linux", "check disk space", "df-space"),
    ("linux", "how full is the drive", "df-space"),
    ("linux", "create empty file", "touch-create"),
    ("linux", "make file executable", "chmod-exec"),
    ("linux", "view file contents", "cat-view"),
    ("linux", "read file", "cat-view"),
    ("linux", "watch file for new lines", "tail-follow"),
    ("linux", "watch log", "tail-follow"),
    ("linux", "search text in files", "grep-search"),
    ("linux", "find text in files", "grep-search"),
    ("linux", "replace text in file", "sed-replace"),
    ("linux", "compare two files", "diff-files"),
    ("linux", "count lines in file", "wc-count"),
    ("linux", "create tar.gz archive", "tar-create"),
    ("linux", "compress folder", "tar-create"),
    ("linux", "extract tar.gz archive", "tar-extract"),
    ("linux", "unzip", "zip-extract"),
    ("linux", "list running processes", "ps-list"),
    ("linux", "task manager", "top-htop"),
    ("linux", "kill process", "kill-pid"),
    ("linux", "force kill process", "kill-force"),
    ("linux", "check memory usage", "free-memory"),
    ("linux", "how much memory is free", "free-memory"),
    ("linux", "is service running", "systemctl-status"),
    ("linux", "restart service", "systemctl-restart"),
    ("linux", "download file", "curl-download"),
    ("linux", "show my ip address", "ip-info"),
    ("linux", "list listening ports", "ss-ports"),
    ("linux", "which ports are open", "ss-ports"),
    ("linux", "test host connectivity", "ping-host"),
    ("linux", "resolve domain name", "dns-lookup"),
    ("linux", "connect to remote host", "ssh-connect"),
    ("linux", "generate ssh key pair", "ssh-keygen"),
    ("linux", "copy file to remote host", "scp-copy"),
    ("linux", "list wifi networks", "nmcli-wifi"),
    ("linux", "change password", "passwd-change"),
    ("linux", "install python package", "pip-install"),
    ("linux", "who am i", "id-user"),
    ("linux", "run command as root", "sudo-run"),
    // ---- windows ----
    ("windows", "delete directory", "ps-rm-rf"),
    ("windows", "remove folder", "ps-rm-rf"),
    ("windows", "list files in directory", "ps-ls-list"),
    ("windows", "create directory", "ps-mkdir-new"),
    ("windows", "new folder", "ps-mkdir-new"),
    ("windows", "delete file", "ps-rm-file"),
    ("windows", "copy file", "ps-cp-file"),
    ("windows", "move file", "ps-mv-move"),
    ("windows", "rename file", "ps-mv-rename"),
    ("windows", "go to parent directory", "ps-cd-up"),
    ("windows", "find large files", "ps-find-large"),
    ("windows", "check disk space", "ps-df-space"),
    ("windows", "view file contents", "ps-cat-view"),
    ("windows", "watch file for new lines", "ps-tail-follow"),
    ("windows", "show last N lines", "ps-tail-last"),
    ("windows", "search text in files", "ps-grep-search"),
    ("windows", "compare two files", "ps-diff-files"),
    ("windows", "extract zip archive", "ps-zip-extract"),
    ("windows", "create symbolic link", "ps-ln-symlink"),
    ("windows", "locate executable", "ps-which-path"),
    ("windows", "list running processes", "ps-ps-list"),
    ("windows", "task manager", "ps-top-cpu"),
    ("windows", "terminate process by PID", "ps-kill-pid"),
    ("windows", "show directory tree", "ps-ls-tree"),
    ("windows", "print working directory", "ps-pwd-where"),
    ("windows", "create empty file", "ps-touch-create"),
    // ---- git ----
    ("git", "undo last commit", "git-reset-soft"),
    ("git", "uncommit", "git-reset-soft"),
    ("git", "list local branches", "git-branch-list"),
    ("git", "create new branch", "git-branch-new"),
    ("git", "switch to branch", "git-checkout"),
    ("git", "create and switch to new branch", "git-checkout-new"),
    ("git", "delete local branch", "git-branch-delete"),
    ("git", "rename branch", "git-branch-rename"),
    ("git", "show current branch", "git-branch-current"),
    ("git", "list remotes", "git-remote-list"),
    ("git", "pull latest changes", "git-pull"),
    ("git", "push commits", "git-push"),
    ("git", "force push", "git-push-force"),
    ("git", "commit all changes", "git-commit-all"),
    ("git", "commit with message", "git-commit-msg"),
    ("git", "amend last commit", "git-commit-amend"),
    ("git", "show commit history", "git-log"),
    ("git", "show history with graph", "git-log-graph"),
    ("git", "show working tree status", "git-status"),
    ("git", "show unstaged changes", "git-diff"),
    ("git", "stage all changes", "git-add-all"),
    ("git", "discard changes to file", "git-restore"),
    ("git", "discard all local changes", "git-reset-hard"),
    ("git", "merge branch into current", "git-merge"),
    ("git", "rebase onto branch", "git-rebase"),
    ("git", "squash commits", "git-rebase-interactive"),
    ("git", "stash current changes", "git-stash-save"),
    ("git", "restore stash", "git-stash-pop"),
    ("git", "list tags", "git-tag-list"),
    ("git", "remove untracked files", "git-clean"),
    // ---- kube ----
    ("kube", "list pods", "kube-pods"),
    ("kube", "show pods", "kube-pods"),
    ("kube", "list deployments", "kube-deployments"),
    ("kube", "list services", "kube-services"),
    ("kube", "list cluster nodes", "kube-nodes"),
    ("kube", "list namespaces", "kube-ns-list"),
    ("kube", "show pod logs", "kube-logs"),
    ("kube", "follow pod logs", "kube-logs-follow"),
    ("kube", "describe a pod", "kube-pod-describe"),
    ("kube", "why is pod failing", "kube-pod-describe"),
    ("kube", "apply manifest file", "kube-apply"),
    ("kube", "delete resource", "kube-delete"),
    ("kube", "delete namespace", "kube-delete-ns"),
    ("kube", "list namespace events", "kube-events"),
    // ---- docker ----
    ("docker", "list images", "docker-images"),
    ("docker", "pull image", "docker-pull"),
    ("docker", "build image from dockerfile", "docker-build"),
    ("docker", "remove image", "docker-rmi"),
    ("docker", "list running containers", "docker-ps"),
    ("docker", "list all containers", "docker-ps-all"),
    ("docker", "stop container", "docker-stop"),
    ("docker", "restart container", "docker-restart"),
    ("docker", "kill container", "docker-kill"),
    ("docker", "shell into container", "docker-exec"),
    ("docker", "show container logs", "docker-logs"),
    ("docker", "follow container logs", "docker-logs-follow"),
    ("docker", "inspect container details", "docker-inspect"),
    ("docker", "start compose services", "docker-compose-up"),
    (
        "docker",
        "stop and remove compose services",
        "docker-compose-down",
    ),
    ("docker", "list volumes", "docker-volume-ls"),
    ("docker", "create volume", "docker-volume-create"),
    ("docker", "remove volume", "docker-volume-rm"),
    ("docker", "list networks", "docker-network-ls"),
    ("docker", "copy file from container", "docker-cp-from"),
    ("docker", "copy file to container", "docker-cp-to"),
    ("docker", "show container port mappings", "docker-port"),
];

struct CaseResult {
    mode: &'static str,
    query: &'static str,
    expected: &'static str,
    // 1-based rank of the expected command, `None` when absent.
    rank: Option<usize>,
    top_id: String,
    top_trigger: String,
}

static SEARCHER: LazyLock<Searcher> = LazyLock::new(|| {
    Searcher::new(
        load_builtin().expect("builtin catalog must load"),
        HashMap::new(),
        &crate::config::Ranking::default(),
    )
});

fn empty_context() -> ContextSnapshot {
    ContextSnapshot {
        shell: ShellType::Unknown,
        cwd: PathBuf::new(),
        cwd_entries: Vec::new(),
        git: None,
        kube: None,
        tags: HashSet::new(),
    }
}

fn run_case(mode: &'static str, query: &'static str, expected: &'static str) -> CaseResult {
    let active_mode = ActiveMode::from_id(mode).expect("case references a valid mode");
    let results = SEARCHER.search(query, &empty_context(), active_mode);
    let rank = results
        .iter()
        .position(|scored| scored.command.id == expected)
        .map(|index| index + 1);
    let top = &results
        .first()
        .expect("search always returns results")
        .command;
    CaseResult {
        mode,
        query,
        expected,
        rank,
        top_id: top.id.clone(),
        top_trigger: top.trigger.clone(),
    }
}

fn run_all() -> Vec<CaseResult> {
    CASES
        .iter()
        .map(|(mode, query, expected)| run_case(mode, query, expected))
        .collect()
}

fn top_k(results: &[CaseResult], k: usize) -> f64 {
    results
        .iter()
        .filter(|case| case.rank.is_some_and(|r| r <= k))
        .count() as f64
        / results.len() as f64
}

fn mrr(results: &[CaseResult]) -> f64 {
    results
        .iter()
        .map(|case| case.rank.map_or(0.0, |rank| 1.0 / rank as f64))
        .sum::<f64>()
        / results.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_case_id_exists_in_catalog() {
        let catalog = load_builtin().unwrap();
        for (mode, _, expected) in CASES {
            assert!(
                catalog.iter().any(|command| &command.id == expected),
                "unknown expected id {expected:?} (mode {mode})"
            );
            assert!(ActiveMode::from_id(mode).is_some(), "unknown mode {mode:?}");
        }
    }

    #[test]
    fn accuracy_report() {
        let results = run_all();
        let misses: Vec<_> = results.iter().filter(|case| case.rank != Some(1)).collect();
        println!(
            "cases: {} | top-1: {:.1}% | top-3: {:.1}% | top-5: {:.1}% | MRR: {:.3}",
            results.len(),
            top_k(&results, 1) * 100.0,
            top_k(&results, 3) * 100.0,
            top_k(&results, 5) * 100.0,
            mrr(&results),
        );
        for case in &misses {
            println!(
                "  MISS [{:>7}] {:?} (want {:?} at rank {:?}, got {:?} — {:?})",
                case.mode, case.query, case.expected, case.rank, case.top_id, case.top_trigger
            );
        }
    }

    #[test]
    fn top1_accuracy_meets_threshold() {
        let results = run_all();
        let accuracy = top_k(&results, 1);
        assert!(
            accuracy >= 0.85,
            "top-1 accuracy {:.1}% below 85% ({}/{} cases)",
            accuracy * 100.0,
            results.iter().filter(|case| case.rank == Some(1)).count(),
            results.len(),
        );
    }

    #[test]
    fn mrr_meets_threshold() {
        let score = mrr(&run_all());
        assert!(score >= 0.90, "MRR {score:.3} below 0.90");
    }
}
