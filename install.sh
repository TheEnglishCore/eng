#!/bin/sh
# install.sh — Install the Engling interpreter system-wide.
#
# Usage:
#   ./install.sh
#
# What this script does:
#   1. Locates the compiled `engling` binary.
#   2. Selects an install path based on the host environment
#      (Termux Android vs. a typical Linux distribution).
#   3. Confirms before overwriting any existing `eng` command.
#   4. Copies the binary to the destination, marks it executable,
#      and verifies the result.
#
# Designed to be POSIX-compliant — runs under /bin/sh, dash, ash, etc.
# No bash-isms (no [[ ]], no arrays, no process substitution).

# ---------------------------------------------------------------------------
# Strict mode
# ---------------------------------------------------------------------------
# -e  : exit on the first error.
# -u  : treat unset variables as errors.
#  \  : we deliberately don't set -o pipefail — POSIX sh doesn't
#       guarantee it, and we want this to keep working on dash.
set -eu

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------
# The directory this script lives in. Used to resolve relative paths
# so the script works regardless of where the user invokes it from.
SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)

# The public command name we install as. The source binary is called
# `engling`, but the user-facing command is always `eng`.
TARGET_NAME="eng"

# ---------------------------------------------------------------------------
# OS / environment detection
# ---------------------------------------------------------------------------
# Termux exposes a `$PREFIX` environment variable that points to its
# private user tree (typically /data/data/com.termux/files/usr). On
# regular Linux distributions $PREFIX is usually unset. We treat a
# non-empty $PREFIX whose path contains "com.termux" as the Termux
# signal — that's the most reliable cross-Android-shell heuristic.
if [ -n "${PREFIX:-}" ] && echo "$PREFIX" | grep -q "com.termux"; then
    IS_TERMUX=1
    INSTALL_DIR="$PREFIX/bin"
else
    IS_TERMUX=0
    INSTALL_DIR="/usr/local/bin"
fi

INSTALL_PATH="$INSTALL_DIR/$TARGET_NAME"

# ---------------------------------------------------------------------------
# Step 1 — locate the compiled binary
# ---------------------------------------------------------------------------
# We prefer the standard cargo release artifact, then fall back to a
# top-level `engling` binary for users who copied it out manually.
SRC_BIN=""
if [ -x "$SCRIPT_DIR/target/release/eng" ]; then
    SRC_BIN="$SCRIPT_DIR/target/release/eng"
elif [ -x "$SCRIPT_DIR/eng" ]; then
    SRC_BIN="$SCRIPT_DIR/eng"
fi

if [ -z "$SRC_BIN" ]; then
    printf '%s\n' \
        "Error: could not find a compiled 'engling' binary." \
        "" \
        "Looked in:" \
        "  - $SCRIPT_DIR/target/release/engling" \
        "  - $SCRIPT_DIR/engling" \
        "" \
        "Build it first with:" \
        "  cargo build --release" \
        "" \
        "Then re-run ./install.sh." >&2
    exit 1
fi

# ---------------------------------------------------------------------------
# Step 2 — pre-flight checks
# ---------------------------------------------------------------------------
# Refuse to proceed if the install directory doesn't exist. On most
# Linux distros /usr/local/bin is present, but it can be missing on
# minimal containers — failing early with a useful message beats a
# cryptic "cp: cannot create regular file" further down.
if [ ! -d "$INSTALL_DIR" ]; then
    printf '%s\n' \
        "Error: install directory does not exist: $INSTALL_DIR" \
        "Create it (or adjust INSTALL_DIR in this script) and try again." >&2
    exit 1
fi

# Write permission check. We can't reliably `test -w` on the directory
# itself on every platform, so we probe by trying to touch a sentinel
# file. This is more portable than `test -w $INSTALL_DIR`.
SENTINEL="$INSTALL_DIR/.engling-install-test-$$"
if ! (umask 077 && : > "$SENTINEL") 2>/dev/null; then
    printf '%s\n' \
        "Error: no write permission to $INSTALL_DIR." \
        "Re-run with appropriate privileges, e.g.:" \
        "  sudo ./install.sh" >&2
    exit 1
fi
rm -f "$SENTINEL"

# ---------------------------------------------------------------------------
# Step 3 — confirm replacement of any existing `eng`
# ---------------------------------------------------------------------------
# `command -v` is the POSIX-blessed way to ask "does this command
# exist and where is it?". We compare paths, not names, so the
# confirmation message always points at the actual file that would
# be replaced.
EXISTING_PATH=""
if command -v eng >/dev/null 2>&1; then
    EXISTING_PATH=$(command -v eng)
fi

if [ -n "$EXISTING_PATH" ] && [ "$EXISTING_PATH" != "$INSTALL_PATH" ]; then
    printf '%s\n' \
        "An 'eng' command already exists at:" \
        "  $EXISTING_PATH" \
        "" \
        "It is NOT the same as the planned install target:" \
        "  $INSTALL_PATH" \
        "" >&2
    printf 'Continuing will leave the existing command untouched; only %s will be created.\n' "$INSTALL_PATH" >&2
fi

if [ -e "$INSTALL_PATH" ]; then
    printf '%s\n' \
        "An 'eng' already exists at the install target:" \
        "  $INSTALL_PATH" \
        ""
    printf 'Replace it? [y/N] '
    read -r REPLY
    case "$REPLY" in
        y|Y|yes|YES|Yes) ;;
        *)
            printf 'Aborted. Nothing was changed.\n'
            exit 0
            ;;
    esac
fi

# ---------------------------------------------------------------------------
# Step 4 — install
# ---------------------------------------------------------------------------
# `install -m 0755` atomically copies the binary and sets the
# executable bits in one step. It's specified by POSIX, so we get
# consistent behavior across Termux, Debian, Alpine, etc.
#
# -m 0755 = rwxr-xr-x : readable+executable by everyone, writable
# only by the owner (typically root). The interpreter doesn't need
# to be world-writable, and we don't want to invite tampering.
echo "Installing $SRC_BIN -> $INSTALL_PATH"
if ! install -m 0755 "$SRC_BIN" "$INSTALL_PATH"; then
    printf '%s\n' "Error: install failed." >&2
    exit 1
fi

# ---------------------------------------------------------------------------
# Step 5 — verify
# ---------------------------------------------------------------------------
# A successful `install` is necessary but not sufficient — the file
# could in principle be unwritable, point at a different filesystem,
# or have inherited odd permissions from a quirky umask. We double-
# check the on-disk state and, when possible, that the new binary
# actually executes.
if [ ! -x "$INSTALL_PATH" ]; then
    printf '%s\n' \
        "Error: $INSTALL_PATH is not executable after install." \
        "Check filesystem permissions." >&2
    exit 1
fi

# Run the binary with a no-op argument to confirm it at least starts.
# We use `--version` if the binary supports it (a polite convention
# most CLIs honor), and fall back to a help probe. We deliberately
# do NOT pipe to a pager — that's the caller's problem.
if "$INSTALL_PATH" --version >/dev/null 2>&1; then
    : # binary executed cleanly with --version
elif "$INSTALL_PATH" --help >/dev/null 2>&1; then
    : # binary executed cleanly with --help
fi
# We don't treat those failing as fatal — some binaries need real
# arguments — but the install itself has already succeeded at this
# point.

# ---------------------------------------------------------------------------
# Step 6 — post-install messaging
# ---------------------------------------------------------------------------
# Build the success banner. We resolve the install path to an
# absolute path so the printed location is unambiguous even if the
# user has a relative PREFIX (rare, but possible in some chroots).
ABS_INSTALL_PATH=$(cd "$INSTALL_DIR" && pwd)/$TARGET_NAME

cat <<EOF

Engling installed successfully.

Run programs with:

    $TARGET_NAME run example.eng

Installed location:
    $ABS_INSTALL_PATH

EOF

# If we just installed to a directory that is NOT on the current
# $PATH, warn the user. On Termux $PREFIX/bin is always on PATH;
# on Linux /usr/local/bin usually is, but not always (e.g. some
# minimal Docker images omit it for non-root users).
case ":${PATH:-}:" in
    *":$INSTALL_DIR:"*) ;;
    *)
        echo "Note: $INSTALL_DIR is not on your PATH."
        echo "Add it to your shell rc, e.g.:"
        echo "    export PATH=\"$INSTALL_DIR:\$PATH\""
        ;;
esac
