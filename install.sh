#!/bin/sh
# cmdpal installer for Linux x86_64 (per-user, no sudo).
#
#   curl -fsSL https://github.com/OWNER/cmdpal/releases/latest/download/install.sh | sh
#   curl -fsSL .../install.sh | sh -s -- --version 0.2.0
#   sh install.sh --uninstall [--purge]
#
# Environment: CMDPAL_INSTALL_DIR (default ~/.local/bin), CMDPAL_BASE_URL (release URL or a local
# directory holding the release assets), CMDPAL_REPO (owner/repo).
set -eu

REPO="${CMDPAL_REPO:-OWNER/cmdpal}"
ARCHIVE="cmdpal-x86_64-unknown-linux-musl.tar.gz"

say() { printf '==> %s\n' "$*"; }
die() { printf 'error: %s\n' "$*" >&2; exit 1; }

usage() {
    cat <<'EOF'
Usage: install.sh [--version X.Y.Z] [--uninstall [--purge]]

  --version X.Y.Z   install a specific release instead of the latest
  --uninstall       remove cmdpal from the install directory
  --purge           with --uninstall, also delete usage history
EOF
}

fetch() { # url-or-dir name destination
    if [ -d "$1" ]; then
        cp "$1/$2" "$3"
    elif command -v curl >/dev/null 2>&1; then
        curl -fsSL "$1/$2" -o "$3"
    elif command -v wget >/dev/null 2>&1; then
        wget -qO "$3" "$1/$2"
    else
        die "need curl or wget to download cmdpal"
    fi
}

sha256_of() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | awk '{print $1}'
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$1" | awk '{print $1}'
    else
        die "need sha256sum or shasum to verify the download"
    fi
}

verify() { # archive-path sums-path
    expected=$(awk -v name="$ARCHIVE" '$2 == name || $2 == "*" name { print $1 }' "$2")
    [ -n "$expected" ] || die "SHA256SUMS has no entry for $ARCHIVE; refusing to install"
    actual=$(sha256_of "$1")
    [ "$actual" = "$expected" ] || die "checksum mismatch for $ARCHIVE (expected $expected, got $actual); nothing was installed"
}

uninstall() {
    rm -f "$install_dir/cmdpal"
    say "Removed $install_dir/cmdpal"
    if [ "$purge" -eq 1 ]; then
        rm -rf "${XDG_DATA_HOME:-$HOME/.local/share}/cmdpal"
        say "Removed usage history"
    fi
    echo "cmdpal uninstalled."
}

install() {
    if [ "$(uname -s)" != "Linux" ] || [ "$(uname -m)" != "x86_64" ]; then
        die "prebuilt binaries exist only for Linux x86_64 (found $(uname -s) $(uname -m)); build from source: cargo install --git https://github.com/$REPO"
    fi

    if [ -z "${CMDPAL_BASE_URL:-}" ]; then
        if [ "$version" = "latest" ]; then
            CMDPAL_BASE_URL="https://github.com/$REPO/releases/latest/download"
        else
            CMDPAL_BASE_URL="https://github.com/$REPO/releases/download/v${version#v}"
        fi
    fi

    work=$(mktemp -d)
    trap 'rm -rf "$work"' EXIT INT TERM HUP

    say "Downloading $ARCHIVE ($version)"
    fetch "$CMDPAL_BASE_URL" "$ARCHIVE" "$work/$ARCHIVE"
    fetch "$CMDPAL_BASE_URL" "SHA256SUMS" "$work/SHA256SUMS"

    say "Verifying SHA-256"
    verify "$work/$ARCHIVE" "$work/SHA256SUMS"

    mkdir "$work/stage"
    tar -xzf "$work/$ARCHIVE" -C "$work/stage"
    [ -f "$work/stage/cmdpal" ] || die "archive does not contain a cmdpal binary"

    mkdir -p "$install_dir"
    # Copy then rename in the same directory: the swap is atomic and works even if cmdpal is running.
    cp "$work/stage/cmdpal" "$install_dir/.cmdpal.new.$$"
    chmod 755 "$install_dir/.cmdpal.new.$$"
    mv -f "$install_dir/.cmdpal.new.$$" "$install_dir/cmdpal"

    say "Installed to $install_dir/cmdpal"
    echo "Installed $("$install_dir/cmdpal" --version)"
    case ":$PATH:" in
        *":$install_dir:"*) echo "Try:  cmdpal delete directory" ;;
        *)
            echo
            echo "$install_dir is not on your PATH. Add this line to ~/.profile, ~/.bashrc or ~/.zshrc:"
            echo "  export PATH=\"$install_dir:\$PATH\""
            ;;
    esac
}

main() {
    version="latest"
    action="install"
    purge=0
    while [ $# -gt 0 ]; do
        case "$1" in
            --version)
                [ $# -ge 2 ] || die "--version needs a value"
                version="$2"
                shift 2
                ;;
            --uninstall) action="uninstall"; shift ;;
            --purge) purge=1; shift ;;
            -h | --help) usage; exit 0 ;;
            *) die "unknown option: $1 (see --help)" ;;
        esac
    done
    install_dir="${CMDPAL_INSTALL_DIR:-$HOME/.local/bin}"
    "$action"
}

# Everything runs from this last line, so a download cut off mid-pipe can never execute half a script.
main "$@"
