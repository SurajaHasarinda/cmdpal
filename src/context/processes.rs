use std::{path::Path, process::Command};

const DOCKER: &str = "docker";
const KUBECTL: &str = "kubectl";
const MAX_LINES: usize = 25;
const MAX_LINE_LEN: usize = 200;

#[derive(Default)]
pub struct ProcessContext {
    pub containers: Vec<String>,
    pub images: Vec<String>,
    pub networks: Vec<String>,
    pub compose_services: Vec<String>,
    pub kube_config: Vec<String>,
}

pub fn collect(cwd: &Path) -> ProcessContext {
    ProcessContext {
        containers: if which(DOCKER).is_some() {
            docker(&["ps", "--format", "{{.Names}}\t{{.Image}}\t{{.Status}}"])
        } else {
            Vec::new()
        },
        images: if which(DOCKER).is_some() {
            docker(&["images", "--format", "{{.Repository}}:{{.Tag}}"])
        } else {
            Vec::new()
        },
        networks: if which(DOCKER).is_some() {
            docker(&["network", "ls", "--format", "{{.Name}}"])
        } else {
            Vec::new()
        },
        compose_services: compose_services(cwd),
        kube_config: if which(KUBECTL).is_some() {
            kubectl(&["config", "view", "--minify", "--flatten"])
        } else {
            Vec::new()
        },
    }
}

pub fn compose_services(cwd: &Path) -> Vec<String> {
    const FILES: [&str; 4] = [
        "docker-compose.yml",
        "docker-compose.yaml",
        "compose.yml",
        "compose.yaml",
    ];
    let Some(name) = FILES.iter().find(|file| cwd.join(file).is_file()) else {
        return Vec::new();
    };
    let Ok(text) = std::fs::read_to_string(cwd.join(name)) else {
        return Vec::new();
    };
    parse_services(&text)
}

fn parse_services(text: &str) -> Vec<String> {
    let mut services = Vec::new();
    let mut in_services = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if !line.starts_with([' ', '\t']) {
            in_services = trimmed.trim_end_matches(':') == "services";
            continue;
        }
        if !in_services || indent_of(line) != 2 {
            continue;
        }
        if let Some(name) = trimmed.strip_suffix(':').filter(|name| !name.is_empty()) {
            services.push(name.to_string());
        }
    }
    services
}

fn indent_of(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

fn which(binary: &str) -> Option<std::path::PathBuf> {
    let paths = std::env::var_os("PATH")?;
    std::env::split_paths(&paths)
        .map(|dir| dir.join(binary))
        .find(|candidate| candidate.is_file())
}

fn docker(args: &[&str]) -> Vec<String> {
    run(DOCKER, args)
}

fn kubectl(args: &[&str]) -> Vec<String> {
    run(KUBECTL, args)
}

fn run(binary: &str, args: &[&str]) -> Vec<String> {
    let Ok(output) = Command::new(binary).args(args).output() else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|line| sanitize_line(line.trim()))
        .filter(|line| !line.is_empty())
        .take(MAX_LINES)
        .collect()
}

/// Commit messages, container names and paths are attacker-controllable and land in a prompt.
pub fn sanitize_line(line: &str) -> String {
    line.chars()
        .filter(|c| !c.is_control())
        .take(MAX_LINE_LEN)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_services_from_compose_file() {
        let yaml = "version: \"3\"\nservices:\n  web:\n    image: nginx\n  db:\n    image: postgres\nvolumes:\n  data:\n";
        assert_eq!(parse_services(yaml), vec!["web", "db"]);
    }

    #[test]
    fn nested_keys_are_not_services() {
        let yaml = "services:\n  web:\n    ports:\n      - \"80:80\"\n";
        assert_eq!(parse_services(yaml), vec!["web"]);
    }

    #[test]
    fn compose_without_services_is_empty() {
        assert!(parse_services("version: \"3\"\nvolumes:\n  data:\n").is_empty());
    }

    #[test]
    fn missing_binary_yields_empty_not_panic() {
        assert!(which("definitely-not-a-real-binary-xyz").is_none());
        assert!(run("definitely-not-a-real-binary-xyz", &[]).is_empty());
    }

    #[test]
    fn strips_control_characters_and_caps_length() {
        assert_eq!(sanitize_line("a\u{0}b"), "ab");
        assert_eq!(sanitize_line(&"x".repeat(500)).len(), MAX_LINE_LEN);
    }

    #[test]
    fn no_compose_file_yields_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(compose_services(dir.path()).is_empty());
    }
}
