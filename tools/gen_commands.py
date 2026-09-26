import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SECTION_MODES = {"### 7.1": "linux", "### 7.2": "git", "### 7.3": "kube", "### 7.5": "docker", "### 7.6": "windows"}
OS_MODES = ("linux", "windows")
SHELLS = {"linux": ["bash", "zsh", "fish"], "windows": ["pwsh"]}

CATEGORY_TAGS = {
    "Filesystem": ["filesystem", "directory", "file"],
    "Text & search": ["text", "search", "file"],
    "Archives": ["archive", "file"],
    "Processes": ["process"],
    "Networking": ["network"],
    "Permissions & users": ["permissions", "user"],
    "Packages": ["package", "install"],
    "System info": ["system", "shell"],
    "Shell text editors": ["shell", "text", "editor"],
    "Dev environment": ["shell", "environment"],
    "Branches": ["branch"],
    "Remote": ["remote"],
    "Commits": ["commit", "history"],
    "Staging": ["stage", "reset"],
    "Merge & rebase": ["merge", "rebase", "conflict"],
    "Stash": ["stash"],
    "Tags": ["tag"],
    "Cleanup & bisect": ["cleanup", "bisect"],
    "Resources — get/describe": ["resources", "describe"],
    "Logs": ["logs"],
    "Lifecycle": ["lifecycle", "delete", "apply"],
    "Scaling & rollouts": ["scale", "rollout", "deployments"],
    "Context & config": ["config", "context", "namespace"],
    "Debugging": ["debug", "logs"],
    "Images": ["image"],
    "Containers": ["container"],
    "Inspect & copy": ["inspect", "container"],
    "Compose": ["compose", "services"],
    "Volumes & networks": ["volume", "network"],
    "Cleanup & system": ["cleanup", "system"],
}
KUBE_KEYWORDS = {
    "pod": "pods",
    "deployment": "deployments",
    "service": "services",
    "namespace": "namespace",
    "port-forward": "port-forward",
    "rollout": "rollout",
    "scale": "scale",
}
PLACEHOLDER_TYPES = {
    "path": "path", "dest": "path", "source": "path", "file": "file", "file1": "file",
    "file2": "file", "branch": "git_branch", "old": "git_branch", "new": "git_branch",
    "remote": "git_remote", "tag": "git_tag", "namespace": "kube_namespace",
    "pod": "kube_pod", "port": "port", "local": "port",
}
POPULAR = {
    "ls-list", "cd-up", "mkdir-new", "rm-file", "cp-file", "mv-move", "grep-search", "cat-view",
    "tail-follow", "ps-list", "curl-get", "ssh-connect", "tar-extract", "git-status", "git-log",
    "git-commit-msg", "git-diff", "git-add-all", "git-pull", "git-merge", "kube-logs",
    "kube-apply", "kube-exec", "kube-get-any", "kube-deployments",
    "docker-images", "docker-ps-all", "docker-run", "docker-logs", "docker-compose-up",
}
BASE_OVERRIDES = {"git-checkout": 0.95}
ALIASES = json.loads((ROOT / "data" / "aliases.json").read_text(encoding="utf-8"))
# Acceptance commands keep a higher base score; their phrasings are merged into the alias list.
ACCEPTANCE = {
    "rm-rf": ["remove folder", "delete folder", "erase directory", "delete a directory"],
    "ss-ports": ["show listening ports", "which ports are open"],
    "curl-download": ["download file from url"],
    "chmod-exec": ["make file executable"],
    "find-name": ["find file by name"],
    "git-checkout-new": ["create branch", "make new branch"],
    "git-checkout": ["switch branch", "change branch"],
    "git-reset-soft": ["undo last commit"],
    "git-stash-save": ["stash changes"],
    "git-push": ["push to remote"],
    "kube-pods": ["list pods"],
    "kube-logs-follow": ["follow logs"],
    "kube-rollout-undo": ["rollback deployment"],
    "kube-port-forward": ["forward port"],
    "docker-ps": ["show running containers"],
    "docker-stop": ["shut down container"],
    "docker-build": ["build docker image"],
    "docker-logs-follow": ["tail container logs"],
    "docker-system-prune": ["clean up docker", "free disk space"],
    "docker-exec": ["shell into container", "open shell in container"],
}
KUBE_EXTRAS = [
    ("kube-cp-download", "download file from pod", "kubectl cp {namespace}/{pod}:{remote_path} {local_path}", "Lifecycle", "low"),
    ("kube-secret-get", "show secret value", "kubectl get secret {name} -o jsonpath='{.data}' -n {namespace}", "Resources — get/describe", "low"),
    ("kube-config-set-cluster", "configure cluster server", "kubectl config set-cluster {name} --server={url}", "Context & config", "low"),
    ("kube-proxy-start", "start api proxy", "kubectl proxy --port={port}", "Debugging", "low"),
    ("kube-apply-dry-run", "preview manifest apply", "kubectl apply -f {file} --dry-run=client", "Lifecycle", "low"),
]


def placeholder_type(name, mode):
    if mode == "kube" and name in ("local", "remote"):
        return "port"
    return PLACEHOLDER_TYPES.get(name, "text")


def build_entry(cmd_id, mode, category, trigger, command, danger):
    canonical = cmd_id.removeprefix("ps-") if mode == "windows" else cmd_id
    tags = [] if mode in OS_MODES else [mode]
    tags += CATEGORY_TAGS.get(category, [])
    if danger == "high":
        tags.append("destructive")
    if re.search(r"\s-\w*[rR]", command):
        tags.append("recursive")
    if mode == "kube":
        tags += [tag for word, tag in KUBE_KEYWORDS.items() if word in trigger or word in cmd_id]
    base = BASE_OVERRIDES.get(canonical, 0.9 if canonical in ACCEPTANCE else 0.7 if canonical in POPULAR else 0.5)
    names = list(dict.fromkeys(re.findall(r"\{([a-z_0-9]+)\}", command)))
    return {
        "id": cmd_id,
        "mode": mode,
        "trigger": trigger,
        "command": command,
        "description": trigger[0].upper() + trigger[1:],
        "aliases": list(dict.fromkeys(ACCEPTANCE.get(canonical, []) + ALIASES.get(canonical, []))),
        "context_tags": list(dict.fromkeys(tags)),
        "shell": SHELLS.get(mode, ["bash", "zsh", "fish", "pwsh"]),
        "placeholders": [
            {"name": n, "type": placeholder_type(n, mode), "description": n.replace("_", " ")} for n in names
        ],
        "danger_level": danger,
        "base_score": base,
        "source": "builtin",
    }


def parse_plan(text):
    entries, mode, category = [], None, None
    for line in text.splitlines():
        if line.startswith("#"):
            mode = next((m for prefix, m in SECTION_MODES.items() if line.startswith(prefix)), None)
        heading = re.match(r"^\*\*(.+?)\s*\(\d+", line)
        if heading:
            category = heading.group(1).strip()
            continue
        if not mode or not line.startswith("|"):
            continue
        cells = line.replace("\\|", "\x00").strip().strip("|").split("|")
        if len(cells) < 3 or cells[0].strip() in ("id", "") or cells[0].strip().startswith("-"):
            continue
        cmd_cell = re.match(r"\s*`(.+?)`(.*)", cells[2])
        if not cmd_cell:
            sys.exit(f"unparseable row: {line}")
        rest = cmd_cell.group(2).lower()
        danger = "high" if "high" in rest else "medium" if "medium" in rest else "low"
        entries.append(build_entry(cells[0].strip(), mode, category, cells[1].strip(), cmd_cell.group(1).replace("\x00", "|"), danger))
    entries += [build_entry(i, "kube", c, t, cmd, d) for i, t, cmd, c, d in KUBE_EXTRAS]
    return entries


entries = parse_plan((ROOT / "PLAN.md").read_text(encoding="utf-8"))
ids = [e["id"] for e in entries]
assert len(ids) == len(set(ids)), "duplicate ids"
counts = {m: sum(e["mode"] == m for e in entries) for m in (*OS_MODES, "git", "kube", "docker")}
print(counts, len(entries))
canonical_ids = {e["id"].removeprefix("ps-") if e["mode"] == "windows" else e["id"] for e in entries}
unknown = [key for key in ALIASES if key not in canonical_ids]
assert not unknown, f"aliases.json keys with no command: {unknown}"
missing = [i for i in ACCEPTANCE if i not in ids]
assert not missing, missing
(ROOT / "data").mkdir(exist_ok=True)
body = ",\n".join(json.dumps(e, ensure_ascii=False) for e in entries)
(ROOT / "data" / "commands.json").write_text(f"[\n{body}\n]\n", encoding="utf-8")
