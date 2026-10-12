# cmdpal

A command palette for your terminal. Describe what you want in plain language, get the exact command on your clipboard. It never runs anything.

```
cmdpal                                    # open the console, search as you type
cmdpal --pick 1 "delete directory"        # → Remove-Item -Recurse -Force {path}
cmdpal --pick 1 -m git "undo last commit" # → git reset --soft HEAD~1
cmdpal "find the running ports"           # → answered by the model, copied
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
| Specific version | `.\install.ps1 -Version 0.1.2` | `… \| sh -s -- --version 0.1.2` |
| Uninstall | `.\install.ps1 -Uninstall` | `sh install.sh --uninstall` |

You can also build from source with `cargo install --git https://github.com/SurajaHasarinda/cmdpal`.

## Use

The shape of your command picks what happens — there is no engine flag.

```bash
cmdpal                            # open the console
cmdpal -m docker                  # open the console in docker mode
cmdpal "find the running ports"   # ask the model, copy the answer, exit
cmdpal --pick 1 "delete dir"      # offline lookup for scripts
```

**In the console** start typing and results update as you go. Pick one with `↑` `↓` and press `Enter` to copy it.

| Key | Action |
|---|---|
| `↑` `↓` | move selection |
| `Enter` | copy the command and quit |
| `Tab` | next mode |
| `Ctrl+A` | toggle to the AI engine |
| `?` | help |
| `Esc` / `Ctrl+C` | quit |

`-m` scopes the console for one run, for example `cmdpal -m git`. Without it, cmdpal reopens in the mode you last used.

| Flag | Does |
|---|---|
| `--setup` | configure the API key, URL and model |
| `--pick N` | print result N from the catalog and exit, for scripts |
| `--verbose` | with `--pick`, show the score breakdown |
| `--context` | show the environment cmdpal detected |

## AI mode

Run `cmdpal --setup` once to store an API key. Any OpenAI-compatible service works. The key is kept in your operating system's credential store, never in a plain config file.

After that, giving cmdpal a query is what asks the model — there is no flag to remember:

```bash
cmdpal "find the running ports"    # → docker port itembank-postgres
```

Inside the console, `Ctrl+A` switches engines, so the catalog and the model are one keystroke apart and you decide when to spend a request. The console itself always starts on the free, instant catalog.

The model is told your working directory, file tree, git state and containers so the answer fits your machine. `cmdpal --context` shows exactly what gets shared.

MIT licensed.
