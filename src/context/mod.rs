pub(crate) mod gitlog;
pub(crate) mod processes;
mod providers;
mod snapshot;
pub(crate) mod tree;

pub use processes::ProcessContext;
pub use providers::detect_shell;
pub use snapshot::{ContextSnapshot, EnrichedContext, GitContext, KubeContext, ShellType};

use crate::modes::ActiveMode;
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver},
    thread,
    time::{Duration, Instant},
};

const PROVIDER_BUDGET: Duration = Duration::from_millis(100);

pub fn detect() -> ContextSnapshot {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let deadline = Instant::now() + PROVIDER_BUDGET;
    let git_result = spawn_provider({
        let cwd = cwd.clone();
        move || providers::detect_git(&cwd)
    });
    let kube_result = spawn_provider(providers::detect_kube);

    let shell = detect_shell();
    let cwd_entries = providers::list_entries(&cwd);
    let git = await_until(git_result, deadline);
    let kube = await_until(kube_result, deadline);
    let tags = collect_tags(shell, &cwd_entries, git.as_ref(), kube.as_ref());
    ContextSnapshot {
        shell,
        cwd,
        cwd_entries,
        git,
        kube,
        tags,
    }
}

fn spawn_provider<T: Send + 'static>(
    work: impl FnOnce() -> Option<T> + Send + 'static,
) -> Receiver<Option<T>> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let _ = sender.send(work());
    });
    receiver
}

fn await_until<T>(receiver: Receiver<Option<T>>, deadline: Instant) -> Option<T> {
    receiver
        .recv_timeout(deadline.saturating_duration_since(Instant::now()))
        .ok()
        .flatten()
}

fn collect_tags(
    shell: ShellType,
    entries: &[String],
    git: Option<&GitContext>,
    kube: Option<&KubeContext>,
) -> HashSet<String> {
    let mut tags: HashSet<String> = entries.iter().map(|name| name.to_lowercase()).collect();
    tags.insert(format!("{shell:?}").to_lowercase());
    if let Some(git) = git {
        tags.insert("git".into());
        tags.extend(
            git.branches
                .iter()
                .chain(&git.branch)
                .map(|branch| branch.to_lowercase()),
        );
        let signals = [
            (git.changes > 0, ["stage", "commit"]),
            (git.ahead > 0, ["push", "remote"]),
            (git.behind > 0, ["pull", "merge"]),
        ];
        tags.extend(
            signals
                .into_iter()
                .filter(|(active, _)| *active)
                .flat_map(|(_, names)| names.map(String::from)),
        );
    }
    tags.extend(docker_tags(&tags).into_iter().map(String::from));
    if let Some(kube) = kube {
        tags.insert("kube".into());
        tags.extend(
            kube.context
                .iter()
                .chain(&kube.namespace)
                .map(|name| name.to_lowercase()),
        );
    }
    tags
}

const COMPOSE_FILES: [&str; 4] = [
    "docker-compose.yml",
    "docker-compose.yaml",
    "compose.yml",
    "compose.yaml",
];

fn docker_tags(entries: &HashSet<String>) -> Vec<&'static str> {
    let mut tags = Vec::new();
    if entries.contains("dockerfile") {
        tags.extend(["docker", "build", "image"]);
    }
    if COMPOSE_FILES.iter().any(|file| entries.contains(*file)) {
        tags.extend(["docker", "compose", "services"]);
    }
    tags
}

pub fn summary(context: &ContextSnapshot, mode: ActiveMode) -> String {
    match (mode, &context.git, &context.kube) {
        (ActiveMode::Git, Some(git), _) => format!(
            "git · {} · {} changes",
            git.branch.as_deref().unwrap_or("detached"),
            git.changes
        ),
        (ActiveMode::Git, None, _) => "git · no repository detected".into(),
        (ActiveMode::Kube, _, Some(kube)) => format!(
            "kube · {}/{}",
            kube.context.as_deref().unwrap_or("?"),
            kube.namespace.as_deref().unwrap_or("default")
        ),
        (ActiveMode::Kube, _, None) => "kube · no kubeconfig detected".into(),
        (ActiveMode::Docker, _, _) => {
            let compose = if context.tags.contains("compose") {
                " · compose"
            } else {
                ""
            };
            format!("docker · {}{compose}", folder_name(&context.cwd))
        }
        (os_mode, ..) => format!("{} · {}", os_mode.id(), folder_name(&context.cwd)),
    }
}

fn folder_name(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}
