/// A terminal command is a single line; anything else is prose the model ignored.
pub fn sanitize(raw: &str) -> Option<String> {
    let body = strip_fences(raw);
    let lines: Vec<&str> = body
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    let [line] = lines.as_slice() else {
        return None;
    };
    if line.contains('\0') {
        return None;
    }
    Some((*line).to_string())
}

fn strip_fences(raw: &str) -> String {
    let trimmed = raw.trim();
    let Some(rest) = trimmed.strip_prefix("```") else {
        return trimmed.to_string();
    };
    let body = match rest.split_once('\n') {
        Some((info, body)) if info.trim().chars().all(|c| c.is_ascii_alphabetic()) => body,
        Some((_, body)) => body,
        None => rest,
    };
    body.trim().trim_end_matches('`').trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_plain_command() {
        assert_eq!(sanitize("git status"), Some("git status".into()));
    }

    #[test]
    fn strips_fences_and_language_tags() {
        assert_eq!(
            sanitize("```bash\ngit status\n```"),
            Some("git status".into())
        );
        assert_eq!(sanitize("```\ngit status\n```"), Some("git status".into()));
        assert_eq!(sanitize("```sh\nls -la```"), Some("ls -la".into()));
    }

    #[test]
    fn rejects_multiline_prose() {
        assert_eq!(
            sanitize("You can use:\ngit status\nwhich shows state"),
            None
        );
        assert_eq!(sanitize("first line\nsecond line"), None);
    }

    #[test]
    fn rejects_empty_and_nul() {
        assert_eq!(sanitize("   \n  "), None);
        assert_eq!(sanitize("git\0status"), None);
    }

    #[test]
    fn trims_surrounding_whitespace() {
        assert_eq!(
            sanitize("  kubectl get pods  \n"),
            Some("kubectl get pods".into())
        );
    }
}
