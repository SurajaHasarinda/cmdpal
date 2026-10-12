# cmdpal

A command palette for your terminal. Describe what you want in plain language, get the exact command on your clipboard. It never runs anything.

```
cmdpal delete directory          # → Remove-Item -Recurse -Force {path}
cmdpal -m git undo last commit   # → git reset --soft HEAD~1
cmdpal -m docker shell into container
```

## Features

- **Plain-language search** over a built-in catalog of shell commands — instant and offline.
- **Five modes** for `windows`, `linux`, `git`, `kube` and `docker`, so the same request returns the right flavor of command.
- **Optional AI mode** — ask a model to write the command, using your live environment.
- **Safe by default** — commands are copied to your clipboard, never executed. Risky ones are flagged.
- **Scriptable** — print a result straight to stdout.
- **Per-user install** on Windows and Linux. No admin rights, no Rust needed.

## Install

**Windows** (PowerShell 5.1 or 7, x64):

```powershell
irm https://github.com/SurajaHasarinda/cmdpal/releases/latest/download/install.ps1 | iex
```

**Linux** (x86_64, glibc — Ubuntu/Debian/Fedora/Arch …):

```sh
curl -fsSL https://github.com/SurajaHasarinda/cmdpal/releases/latest/download/install.sh | sh
```

Then open a new terminal and run `cmdpal`. Each installer verifies a SHA-256 checksum before installing.

| | Windows | Linux |
|---|---|---|
| Installs to | `%LOCALAPPDATA%\Programs\cmdpal` | `~/.local/bin` |
| Specific version | `.\install.ps1 -Version 0.1.1` | `… \| sh -s -- --version 0.1.1` |
| Uninstall | `.\install.ps1 -Uninstall` | `sh install.sh --uninstall` |

You can also build from source with `cargo install --git https://github.com/SurajaHasarinda/cmdpal`.

## Use

```
cmdpal [-m <mode>] [--ai] [query...]
```

Start typing and results update as you go. Pick one with `↑` `↓` and press `Enter` to copy it.

| Key | Action |
|---|---|
| `↑` `↓` | move selection |
| `Enter` | copy the command and quit |
| `Tab` | next mode |
| `Ctrl+A` | toggle AI mode |
| `?` | help |
| `Esc` / `Ctrl+C` | quit |

`-m` picks a mode for one run, for example `cmdpal -m git`. Without it, cmdpal reopens in the mode you last used.

Useful flags:

| Flag | Does |
|---|---|
| `--setup` | configure the API key and AI settings |
| `--ai` | answer with AI instead of the catalog |
| `--pick N` | print result N and exit, for scripts |
| `--context` | show the environment cmdpal detected |

## AI mode

Run `cmdpal --setup` and choose **AI** to store an API key. Any OpenAI-compatible service works. The key is kept in your operating system's credential store, never in a plain config file.

Then press `Ctrl+A` inside the palette to switch engines — the catalog and the model are both one keystroke away, so you decide when to spend a request.

AI mode sends your working directory, file tree, git state and containers so the answer fits your machine. Ask before using it if that matters to you; `cmdpal --context` shows exactly what gets shared.

MIT licensed.
