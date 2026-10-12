use super::*;
use crate::context::gitlog::GitDetail;
use crate::context::processes::ProcessContext;
use crate::context::GitContext;
use std::{collections::HashSet, path::PathBuf};

fn snapshot() -> ContextSnapshot {
    ContextSnapshot {
        shell: ShellType::Pwsh,
        cwd: PathBuf::from("D:/repos/app"),
        cwd_entries: vec![],
        git: Some(GitContext {
            branch: Some("main".into()),
            ..Default::default()
        }),
        kube: None,
        tags: HashSet::new(),
    }
}

fn empty() -> EnrichedContext {
    EnrichedContext {
        tree: vec![],
        git: GitDetail::default(),
        processes: ProcessContext::default(),
    }
}

#[test]
fn environment_is_always_present() {
    let rendered = render(&snapshot(), &empty(), Some(ActiveMode::Git));
    assert!(rendered.contains("requested_mode: git"));
    assert!(rendered.contains("D:/repos/app"));
    assert!(rendered.contains("shell: powershell"));
}

#[test]
fn a_bare_query_omits_the_mode_line() {
    let rendered = render(&snapshot(), &empty(), None);
    assert!(!rendered.contains("requested_mode"));
    // The live context still describes the machine, so the model is never blind.
    assert!(rendered.contains("os: "));
    assert!(rendered.contains("shell: powershell"));
    assert!(rendered.contains("working_directory: "));
}

#[test]
fn empty_sections_are_omitted() {
    let rendered = render(&snapshot(), &empty(), Some(ActiveMode::Git));
    assert!(!rendered.contains("# docker"));
    assert!(!rendered.contains("# filesystem"));
}

#[test]
fn git_section_carries_branch_history_and_changes() {
    let mut enriched = empty();
    enriched.git.is_repo = true;
    enriched.git.commits = vec!["abc1234 fix crash".into()];
    enriched.git.changed_paths = vec!["src/main.rs".into()];
    let rendered = render(&snapshot(), &enriched, Some(ActiveMode::Git));
    assert!(rendered.contains("branch: main"));
    assert!(rendered.contains("abc1234 fix crash"));
    assert!(rendered.contains("src/main.rs"));
}

#[test]
fn docker_section_lists_runtime_state() {
    let mut enriched = empty();
    enriched.processes.containers = vec!["web".into()];
    enriched.processes.images = vec!["nginx:latest".into()];
    enriched.processes.networks = vec!["bridge".into()];
    enriched.processes.compose_services = vec!["web".into()];
    let rendered = render(&snapshot(), &enriched, Some(ActiveMode::Docker));
    assert!(rendered.contains("containers:"));
    assert!(rendered.contains("nginx:latest"));
    assert!(rendered.contains("bridge"));
    assert!(rendered.contains("web"));
}

#[test]
fn user_turn_is_delimited_and_ends_with_the_request() {
    let prompt = user_turn(
        "delete a branch",
        &snapshot(),
        Some(ActiveMode::Git),
        &Config::default(),
    );
    assert!(prompt.starts_with("<context>"));
    assert!(prompt.contains("</context>"));
    assert!(prompt.ends_with("Request: delete a branch"));
    assert!(DEFAULT_SYSTEM_PROMPT.contains("untrusted data"));
}

#[test]
fn oversized_context_is_truncated_on_a_char_boundary() {
    let long = "x".repeat(10_000);
    let capped = cap(long, 100);
    assert!(capped.ends_with("… truncated"));
    assert!(capped.len() <= 120);
}

#[test]
fn short_context_is_untouched() {
    assert_eq!(cap("short".into(), 100), "short");
}
