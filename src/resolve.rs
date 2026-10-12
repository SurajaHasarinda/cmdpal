//! Fill `{placeholder}` tokens from the detected environment.
//!
//! Only when exactly one candidate exists, so no ambiguous guess is made silently.
//! `{path}` is never filled: the cwd would give `Remove-Item -Recurse -Force .`.

use crate::{
    context::{ContextSnapshot, ProcessContext},
    knowledge::Command,
};

pub struct Candidates<'a> {
    pub containers: &'a [String],
    pub images: &'a [String],
    pub networks: &'a [String],
    pub compose_services: &'a [String],
    pub pods: &'a [String],
    pub branches: &'a [String],
    pub remotes: &'a [String],
    pub namespace: Option<&'a str>,
}

impl<'a> Candidates<'a> {
    pub fn new(context: &'a ContextSnapshot, processes: &'a ProcessContext) -> Self {
        Self {
            containers: &processes.containers,
            images: &processes.images,
            networks: &processes.networks,
            compose_services: &processes.compose_services,
            pods: &[],
            branches: context
                .git
                .as_ref()
                .map(|git| git.branches.as_slice())
                .unwrap_or_default(),
            remotes: context
                .git
                .as_ref()
                .map(|git| git.remotes.as_slice())
                .unwrap_or_default(),
            namespace: context
                .kube
                .as_ref()
                .and_then(|kube| kube.namespace.as_deref()),
        }
    }

    /// The only candidate for a placeholder, or `None` if there isn't exactly one.
    fn only(&self, name: &str) -> Option<&str> {
        let group: &[String] = match name {
            "container" => self.containers,
            "image" => self.images,
            "network" => self.networks,
            "service" => self.compose_services,
            "pod" => self.pods,
            "branch" => self.branches,
            "remote" => self.remotes,
            "namespace" => return self.namespace,
            _ => return None,
        };
        single(group)
    }
}

fn single(group: &[String]) -> Option<&str> {
    match group {
        [only] => Some(only.as_str()),
        _ => None,
    }
}

/// The command with resolvable placeholders filled; others left verbatim.
pub fn resolve(command: &Command, candidates: &Candidates) -> String {
    let mut out = String::with_capacity(command.command.len());
    let mut rest = command.command.as_str();
    while let Some(open) = rest.find('{') {
        let Some(close_offset) = rest[open..].find('}') else {
            break;
        };
        let close = open + close_offset;
        let name = &rest[open + 1..close];
        out.push_str(&rest[..open]);
        match candidates.only(name) {
            Some(value) => out.push_str(value),
            None => out.push_str(&rest[open..=close]),
        }
        rest = &rest[close + 1..];
    }
    out.push_str(rest);
    out
}

/// Whether resolving this command needs the process context.
pub fn needs_processes(command: &Command) -> bool {
    command.placeholders.iter().any(|placeholder| {
        matches!(
            placeholder.name.as_str(),
            "container" | "image" | "network" | "service"
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::knowledge::{Danger, Placeholder};

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    fn command(template: &str, placeholders: &[(&str, &str)]) -> Command {
        Command {
            id: "test".into(),
            mode: "docker".into(),
            trigger: "trigger".into(),
            command: template.into(),
            description: "description".into(),
            aliases: Vec::new(),
            context_tags: Vec::new(),
            placeholders: placeholders
                .iter()
                .map(|(name, kind)| Placeholder {
                    name: (*name).to_string(),
                    kind: (*kind).to_string(),
                })
                .collect(),
            danger_level: Danger::Low,
            base_score: 0.5,
        }
    }

    fn candidates<'a>(processes: &'a ProcessContext) -> Candidates<'a> {
        Candidates {
            containers: &processes.containers,
            images: &processes.images,
            networks: &processes.networks,
            compose_services: &processes.compose_services,
            pods: &[],
            branches: &[],
            remotes: &[],
            namespace: None,
        }
    }

    #[test]
    fn fills_a_placeholder_with_a_single_candidate() {
        let processes = ProcessContext {
            containers: strings(&["itembank-postgres"]),
            ..ProcessContext::default()
        };
        let resolved = resolve(
            &command("docker port {container}", &[("container", "text")]),
            &candidates(&processes),
        );
        assert_eq!(resolved, "docker port itembank-postgres");
    }

    #[test]
    fn leaves_the_token_when_candidates_are_ambiguous() {
        let processes = ProcessContext {
            containers: strings(&["api", "db"]),
            ..ProcessContext::default()
        };
        let resolved = resolve(
            &command("docker port {container}", &[("container", "text")]),
            &candidates(&processes),
        );
        assert_eq!(resolved, "docker port {container}");
    }

    #[test]
    fn leaves_the_token_when_nothing_was_detected() {
        let processes = ProcessContext::default();
        let resolved = resolve(
            &command("docker port {container}", &[("container", "text")]),
            &candidates(&processes),
        );
        assert_eq!(resolved, "docker port {container}");
    }

    #[test]
    fn a_tag_never_leaks_into_the_container_name() {
        // Regression: tab-joined docker output once collapsed into "itembank-postgrespostgres:16".
        let processes = ProcessContext {
            containers: strings(&["itembank-postgres"]),
            images: strings(&["postgres:16"]),
            ..ProcessContext::default()
        };
        let resolved = resolve(
            &command("docker port {container}", &[("container", "text")]),
            &candidates(&processes),
        );
        assert_eq!(resolved, "docker port itembank-postgres");
        assert!(!resolved.contains("postgres:16"));
    }

    #[test]
    fn path_is_never_resolved() {
        // cwd would produce "Remove-Item -Recurse -Force ."
        let processes = ProcessContext::default();
        let resolved = resolve(
            &command("Remove-Item -Recurse -Force {path}", &[("path", "path")]),
            &candidates(&processes),
        );
        assert_eq!(resolved, "Remove-Item -Recurse -Force {path}");
    }

    #[test]
    fn resolves_several_placeholders_in_one_command() {
        let processes = ProcessContext {
            containers: strings(&["web"]),
            images: strings(&["nginx:latest"]),
            ..ProcessContext::default()
        };
        let resolved = resolve(
            &command(
                "docker run -p {host_port}:{container_port} {image}",
                &[
                    ("host_port", "port"),
                    ("container_port", "port"),
                    ("image", "text"),
                ],
            ),
            &candidates(&processes),
        );
        assert_eq!(
            resolved,
            "docker run -p {host_port}:{container_port} nginx:latest"
        );
    }

    #[test]
    fn unknown_names_are_preserved_verbatim() {
        let processes = ProcessContext {
            containers: strings(&["only-one"]),
            ..ProcessContext::default()
        };
        let resolved = resolve(
            &command("git log --author={pattern}", &[("pattern", "text")]),
            &candidates(&processes),
        );
        assert_eq!(resolved, "git log --author={pattern}");
    }

    #[test]
    fn an_unterminated_brace_is_not_consumed() {
        let processes = ProcessContext {
            containers: strings(&["only-one"]),
            ..ProcessContext::default()
        };
        let resolved = resolve(
            &command("echo {container", &[("container", "text")]),
            &candidates(&processes),
        );
        assert_eq!(resolved, "echo {container");
    }

    #[test]
    fn a_command_without_placeholders_is_unchanged() {
        let processes = ProcessContext::default();
        let resolved = resolve(&command("git status", &[]), &candidates(&processes));
        assert_eq!(resolved, "git status");
    }

    #[test]
    fn only_process_backed_names_trigger_collection() {
        assert!(needs_processes(&command(
            "docker port {container}",
            &[("container", "text")]
        )));
        assert!(!needs_processes(&command(
            "ls -la {path}",
            &[("path", "path")]
        )));
        assert!(!needs_processes(&command("git status", &[])));
    }
}
