use std::path::Path;

const IGNORED: [&str; 8] = [
    ".git",
    "target",
    "node_modules",
    "vendor",
    "dist",
    ".venv",
    "__pycache__",
    ".idea",
];

pub struct TreeBudget {
    pub depth: usize,
    pub files: usize,
}

struct Walker<'a> {
    depth: usize,
    remaining: usize,
    out: &'a mut Vec<String>,
}

pub fn walk(root: &Path, budget: &TreeBudget) -> Vec<String> {
    let mut out = Vec::new();
    let mut walker = Walker {
        depth: budget.depth,
        remaining: budget.files,
        out: &mut out,
    };
    walker.descend(root, "", 0);
    out
}

impl Walker<'_> {
    fn descend(&mut self, dir: &Path, prefix: &str, level: usize) {
        if level >= self.depth || self.remaining == 0 {
            return;
        }
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        let mut sorted: Vec<_> = entries.flatten().collect();
        sorted.sort_by_key(std::fs::DirEntry::file_name);
        for entry in sorted {
            if self.remaining == 0 {
                return;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') || IGNORED.contains(&name.as_str()) {
                continue;
            }
            self.remaining -= 1;
            let relative = join(prefix, &name);
            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            if metadata.is_dir() {
                self.out.push(format!("{relative}/"));
                self.descend(&entry.path(), &relative, level + 1);
            } else {
                self.out.push(relative);
            }
        }
    }
}

fn join(prefix: &str, name: &str) -> String {
    if prefix.is_empty() {
        name.to_string()
    } else {
        format!("{prefix}/{name}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scaffold(files: &[&str]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for file in files {
            let path = dir.path().join(file);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, "").unwrap();
        }
        dir
    }

    fn budget(depth: usize, files: usize) -> TreeBudget {
        TreeBudget { depth, files }
    }

    #[test]
    fn respects_depth_limit() {
        let dir = scaffold(&["a/b/c/deep.txt", "top.txt"]);
        let found = walk(dir.path(), &budget(1, 50));
        assert!(found.contains(&"top.txt".to_string()));
        assert!(found.contains(&"a/".to_string()));
        assert!(!found.iter().any(|line| line.contains("deep.txt")));
    }

    #[test]
    fn respects_file_budget() {
        let dir = scaffold(&["a.txt", "b.txt", "c.txt", "d.txt"]);
        assert_eq!(walk(dir.path(), &budget(5, 2)).len(), 2);
    }

    #[test]
    fn skips_ignored_and_hidden() {
        let dir = scaffold(&["keep.txt", "target/out.bin", ".hidden", ".git/HEAD"]);
        let found = walk(dir.path(), &budget(5, 50));
        assert_eq!(found, vec!["keep.txt".to_string()]);
    }

    #[test]
    fn missing_root_is_empty_not_panic() {
        assert!(walk(Path::new("no-such-dir-xyz"), &budget(3, 10)).is_empty());
    }

    #[test]
    fn zero_budget_yields_nothing() {
        let dir = scaffold(&["a.txt"]);
        assert!(walk(dir.path(), &budget(3, 0)).is_empty());
    }
}
