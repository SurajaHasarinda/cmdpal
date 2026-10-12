# cmdpal

A fast, offline command palette for the terminal. Describe what you want in plain language and get the exact command, copied to your clipboard. It never runs anything for you.

```
cmdpal delete directory          # → Remove-Item -Recurse -Force {path}   (or rm -rf {path} on Linux)
cmdpal -m git undo last commit   # → git reset --soft HEAD~1
cmdpal -m docker shell into container
```

## Install

Per-user, no admin rights, no Rust required.

**Windows** (PowerShell 5.1 or 7, x64):

```powershell
irm https://github.com/SurajaHasarinda/cmdpal/releases/latest/download/install.ps1 | iex
```

**Linux** (x86_64, glibc — Ubuntu/Debian/Fedora/Arch …):

```sh
curl -fsSL https://github.com/SurajaHasarinda/cmdpal/releases/latest/download/install.sh | sh
```

Then open a new terminal and run `cmdpal`. The installers verify a SHA-256 checksum before installing anything.

| | Windows | Linux |
|---|---|---|
| Installs to | `%LOCALAPPDATA%\Programs\cmdpal` (added to your user PATH) | `~/.local/bin` (prints the `export PATH=…` line if it's missing from PATH) |
| Specific version | `.\install.ps1 -Version 0.2.0` | `… \| sh -s -- --version 0.2.0` |
| Upgrade | re-run the installer | re-run the installer |
| Uninstall | `.\install.ps1 -Uninstall` (`-Purge` also deletes history) | `sh install.sh --uninstall` (`--purge` also deletes history) |

Download `install.ps1` / `install.sh` from the [latest release](https://github.com/SurajaHasarinda/cmdpal/releases/latest) first if you prefer to read a script before running it. Windows binaries are not code-signed, so SmartScreen may ask for confirmation the first time.

**From source** (needs Rust and a C compiler): `cargo install --git https://github.com/SurajaHasarinda/cmdpal`

## Use

```
cmdpal [-m <mode>] [query...]
```

| Key | Action |
|---|---|
| type | search; results update as you type |
| `↑` `↓` / `Ctrl+P` `Ctrl+N` | move selection |
| `Enter` | copy the command and quit |
| `Ctrl+Y` | copy the command with its description as a comment |
| `Tab` | next mode: windows → linux → git → kube → docker |
| `?` | help |
| `Esc` / `Ctrl+C` | quit |

Modes: `windows` (PowerShell) and `linux` are chosen automatically from your OS at first launch; after that cmdpal reopens in the mode you last used. Override once with `-m`, for example `cmdpal -m linux` from WSL or Git Bash.

Placeholders like `{path}` are shown in orange: replace them after pasting. High-risk commands (recursive deletes, force pushes) are shown in red with a warning.

Other options: `--pick N` prints result N and exits (for scripts), `--verbose` adds the score breakdown, `--context` prints the detected environment as JSON.

Usage history and the remembered mode are stored in `%APPDATA%\cmdpal\db.sqlite` (Windows) or `~/.local/share/cmdpal/db.sqlite` (Linux); set `CMDPAL_DB` to change it.

## Development

```sh
cargo test
cargo fmt --check && cargo clippy --all-targets -- -D warnings
python tools/gen_commands.py   # rebuild data/commands.json after editing the catalog tables or data/aliases.json
```

Release: bump `version` in `Cargo.toml`, then `git tag vX.Y.Z && git push origin vX.Y.Z`. CI builds the Windows and Linux archives, smoke-tests both installers, and publishes the release.

MIT licensed.
