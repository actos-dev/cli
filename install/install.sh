#!/bin/sh
#
# install.sh — Install the Actos CLI (`actos`) on Linux and macOS.
#
# Install Actos by building it from source with Cargo (recommended):
#
#   curl -fsSL https://actos.com.tr/cli/install.sh | sh
#   # or
#   wget -qO- https://actos.com.tr/cli/install.sh | sh
#
# Or download it and run it explicitly:
#
#   curl -fsSL https://actos.com.tr/cli/install.sh -o install.sh
#   sh install.sh
#
# The script is pure POSIX shell — it works with /bin/sh on both Linux and
# macOS (Darwin), and is safe to pipe directly into `sh` because every code
# path is self-contained and the script never re-reads stdin.
#
# Options:
#   --version <x.y.z>    Install a specific published version (default: latest).
#   --no-modify-path     Do not print the PATH export / shell-profile advice.
#   --help               Show this help and exit.
#   --force              Reinstall over an existing `actos` binary.
#
# Exit codes:
#   0  success
#   1  unexpected internal error
#   2  could not find a compatible Cargo/Rust toolchain (nothing installed)
#   3  `cargo install` failed
#

set -eu

# ---------------------------------------------------------------------------
# Small logger helpers. Colors are only emitted when stdout is a TTY.
# ---------------------------------------------------------------------------
if [ -t 1 ]; then
    C_BOLD="$(printf '\033[1m')"
    C_GREEN="$(printf '\033[32m')"
    C_YELLOW="$(printf '\033[33m')"
    C_RED="$(printf '\033[31m')"
    C_CYAN="$(printf '\033[36m')"
    C_DIM="$(printf '\033[2m')"
    C_RESET="$(printf '\033[0m')"
else
    C_BOLD=""; C_GREEN=""; C_YELLOW=""; C_RED=""; C_CYAN=""; C_DIM=""; C_RESET=""
fi

info()  { printf '%s\n' "${C_CYAN}==>${C_RESET} $*"; }
ok()    { printf '%s\n' "${C_GREEN}✓${C_RESET} $*"; }
warn()  { printf '%s\n' "${C_YELLOW}⚠ ${C_RESET}$*"; }
err()   { printf '%s\n' "${C_RED}✗${C_RESET} $*" >&2; }
die()   { err "$1"; exit "${2:-1}"; }

# ---------------------------------------------------------------------------
# Usage
# ---------------------------------------------------------------------------
usage() {
    cat <<'EOF'
Actos CLI installer (Linux + macOS)

Usage: sh install.sh [options]

Options:
  --version <x.y.z>   Install a specific published version (default: latest).
  --no-modify-path    Do not print PATH export / shell-profile advice.
  --force             Reinstall even if `actos` is already installed.
  --help              Show this help and exit.

Examples:
  sh install.sh
  sh install.sh --version 0.1.0
  sh install.sh --no-modify-path

For a healthy default setup, make sure you have Rust installed first:
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
EOF
    exit 0
}

# ---------------------------------------------------------------------------
# Parse arguments.
# ---------------------------------------------------------------------------
VERSION=""
MODIFY_PATH=1
FORCE=0

while [ $# -gt 0 ]; do
    case "$1" in
        --help|-h)
            usage
            ;;
        --version)
            [ $# -ge 2 ] || die "--version requires an argument." 2
            VERSION="$2"
            shift 2
            ;;
        --no-modify-path)
            MODIFY_PATH=0
            shift
            ;;
        --force)
            FORCE=1
            shift
            ;;
        *)
            die "Unknown option: $1 (run with --help)." 2
            ;;
    esac
done

# ---------------------------------------------------------------------------
# Banner.
# ---------------------------------------------------------------------------
info "${C_BOLD}Actos CLI installer${C_RESET}"
info ""

# ---------------------------------------------------------------------------
# Detect OS and architecture (informational — we install from source via Cargo,
# so there are no prebuilt binaries to download per-platform).
# ---------------------------------------------------------------------------
OS="$(uname -s 2>/dev/null || echo unknown)"
ARCH="$(uname -m 2>/dev/null || echo unknown)"

case "$OS" in
    Linux)  os_pretty="Linux" ;;
    Darwin) os_pretty="macOS" ;;
    *)      os_pretty="$OS" ;;
esac

info "Detected platform: ${C_BOLD}$os_pretty${C_RESET} / ${C_BOLD}$ARCH${C_RESET}"

case "$OS" in
    Linux|Darwin) : ;;  # supported
    *)
        warn "Unknown OS '$OS'. The installer may not work here — "
        warn "Actos is known to build on Linux and macOS."
        ;;
esac

info ""

# ---------------------------------------------------------------------------
# Locate Cargo. We need the toolchain to build the binary from source.
# ---------------------------------------------------------------------------
check_cargo() {
    if ! command -v cargo >/dev/null 2>&1; then
        err ""
        err "Cargo (Rust) was not found on this system."
        err "Actos is installed from source, so Cargo is required."
        err ""
        err "Install Rust with rustup (one command):"
        err "    ${C_BOLD}curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh${C_RESET}"
        err ""
        err "Then open a new terminal (so your PATH is refreshed) and run"
        err "this installer again."
        die "Cargo is required to install Actos." 2
    fi

    # rustc is needed by cargo to compile; both ship together via rustup, but
    # check defensively in case someone only has cargo.
    if ! command -v rustc >/dev/null 2>&1; then
        err "Found Cargo but not rustc. Your Rust toolchain looks broken."
        err "Reinstall it with:  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        die "rustc is required to build Actos." 2
    fi
}

check_cargo
info "Found Cargo at:  $(command -v cargo)"

# ---------------------------------------------------------------------------
# Determine where Cargo installs binaries, and where our binary should land.
# ---------------------------------------------------------------------------
CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}"
CARGO_BIN="${CARGO_BIN:-$CARGO_HOME/bin}"

info "Cargo bin dir:  $CARGO_BIN"
ok ""

# ---------------------------------------------------------------------------
# Already installed? Skip the (potentially slow) rebuild unless --force.
# ---------------------------------------------------------------------------
if [ -f "$CARGO_BIN/actos" ] && [ "$FORCE" -ne 1 ]; then
    warn "'actos' is already installed at: $CARGO_BIN/actos"
    warn "Re-run with --force to reinstall, or --version to pick a version."
    info ""
else
    # -----------------------------------------------------------------------
    # Build & install from crates.io.
    # -----------------------------------------------------------------------
    info "Installing the Actos CLI from crates.io ..."
    info "  (${C_DIM}cargo install actos-cli --locked${C_RESET})"
    info ""
    info "Running: cargo install actos-cli --locked ${VERSION:+--version $VERSION}"
    info ""

    # Propagate --force to cargo so it actually overwrites an existing binary.
    if [ "$FORCE" -eq 1 ]; then
        force_arg="--force"
    else
        force_arg=""
    fi

    if ! cargo install actos-cli --locked $force_arg ${VERSION:+--version "$VERSION"}; then
        die "cargo install failed (exit code $?)." 3
    fi
    ok ""
fi

# ---------------------------------------------------------------------------
# Verify the binary actually runs.
# ---------------------------------------------------------------------------
if [ ! -x "$CARGO_BIN/actos" ]; then
    # Fall back to PATH in case CARGO_BIN detection was off.
    if command -v actos >/dev/null 2>&1; then
        CARGO_BIN="$(dirname "$(command -v actos)")"
    else
        die "Installer finished but 'actos' was not found in $CARGO_BIN." 3
    fi
fi

if ! "$CARGO_BIN/actos" --version >/dev/null 2>&1; then
    die "Installed binary does not run. Please check the Cargo output above." 3
fi

installed_version="$("$CARGO_BIN/actos" --version 2>&1 | head -n1)"
ok "Actos installed successfully:  ${C_BOLD}${installed_version}${C_RESET} (${C_DIM}$CARGO_BIN/actos${C_RESET})"

# ---------------------------------------------------------------------------
# PATH check + shell-profile advice.
# ---------------------------------------------------------------------------
if [ "$MODIFY_PATH" -eq 1 ]; then
    in_path=0
    bin_abs="$CARGO_BIN"
    # Word-split on ':' is intentional; disable the lint under `sh`.
    # shellcheck disable=SC2086
    IFS=:
    for dir in $PATH; do
        if [ "$dir" = "$bin_abs" ]; then
            in_path=1
            break
        fi
    done
    unset IFS

    info ""
    if [ "$in_path" -eq 1 ]; then
        ok "~/.cargo/bin is already on your PATH. You're all set."
    else
        warn "~/.cargo/bin is NOT on your current PATH."
        warn "Add it to your PATH so you can run 'actos' from anywhere:"
        info ""
        info "  export PATH=\"\$HOME/.cargo/bin:\$PATH\""
        info ""
        info "To make that permanent, pick the block for your shell and this"
        info "will be appended to your startup file on later shells:"
        info ""
        case "$(basename "${SHELL:-/bin/sh}")" in
            zsh)
                info "  # zsh  ->  ~/.zshrc"
                info "  echo 'export PATH=\"\$HOME/.cargo/bin:\$PATH\"' >> \"\$HOME/.zshrc\""
                info "  source \"\$HOME/.zshrc\""
                ;;
            fish)
                info "  # fish ->  ~/.config/fish/config.fish"
                info "  set -U fish_user_paths \"\$HOME/.cargo/bin\" \$fish_user_paths"
                ;;
            bash)
                info "  # bash ->  ~/.bashrc  (or ~/.bash_profile on macOS login shells)"
                info "  echo 'export PATH=\"\$HOME/.cargo/bin:\$PATH\"' >> \"\$HOME/.bashrc\""
                info "  source \"\$HOME/.bashrc\""
                ;;
            *)
                info "  Add this line to your shell's startup file (e.g. ~/.profile):"
                info "  export PATH=\"\$HOME/.cargo/bin:\$PATH\""
                ;;
        esac
        info ""
        warn "Until then, run Actos with its full path:  $CARGO_BIN/actos --help"
    fi
fi

# ---------------------------------------------------------------------------
# Final summary + next steps.
# ---------------------------------------------------------------------------
info ""
ok "${C_BOLD}Actos is ready.${C_RESET} Try it:"
info ""
info "    actos --help"
info ""
info "Uninstall later with:"
info "    cargo uninstall actos-cli        ${C_DIM}# removes the binary${C_RESET}"
info ""
exit 0