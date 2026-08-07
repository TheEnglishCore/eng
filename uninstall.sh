#!/bin/sh
# uninstall.sh — Remove the system-wide `eng` command installed by
# install.sh.
#
# Usage:
#   ./uninstall.sh
#
# What this script does:
#   1. Re-derives the install path using the same detection rules
#      as install.sh (Termux vs. Linux).
#   2. Confirms the user actually wants to remove the file.
#   3. Deletes the binary and reports success.
#
# POSIX-compliant. No bash-only constructs.

# ---------------------------------------------------------------------------
# Strict mode
# ---------------------------------------------------------------------------
set -eu

# ---------------------------------------------------------------------------
# Path detection — must match install.sh exactly
# ---------------------------------------------------------------------------
# Reusing the same $PREFIX heuristic as install.sh keeps the two
# scripts in sync without us having to share a sourced file (POSIX
# sh has no clean cross-shell include story).
if [ -n "${PREFIX:-}" ] && echo "$PREFIX" | grep -q "com.termux"; then
    INSTALL_DIR="$PREFIX/bin"
else
    INSTALL_DIR="/usr/local/bin"
fi

TARGET_NAME="eng"
INSTALL_PATH="$INSTALL_DIR/$TARGET_NAME"

# ---------------------------------------------------------------------------
# Step 1 — verify there's something to remove
# ---------------------------------------------------------------------------
# If the file isn't there at all, we have nothing to do. Treat that
# as a soft success so the script is safe to re-run and idempotent
# in CI cleanup hooks.
if [ ! -e "$INSTALL_PATH" ]; then
    printf '%s\n' \
        "No 'eng' command found at $INSTALL_PATH." \
        "Nothing to uninstall."
    exit 0
fi

# ---------------------------------------------------------------------------
# Step 2 — show what we're about to remove and ask
# ---------------------------------------------------------------------------
# Always print the absolute path so the user can sanity-check it,
# especially important on systems where $INSTALL_DIR could resolve
# somewhere unexpected (e.g. a Termux user running under sudo with
# a stripped environment).
ABS_INSTALL_PATH=$(cd "$INSTALL_DIR" && pwd)/$TARGET_NAME

printf '%s\n' \
    "About to remove:" \
    "  $ABS_INSTALL_PATH" \
    ""

# Detect any pre-existing `eng` on PATH that is *not* the one we're
# about to delete. If one exists, warn the user that we'll only
# remove our own copy — the other one will remain.
if command -v eng >/dev/null 2>&1; then
    EXISTING_PATH=$(command -v eng)
    if [ "$EXISTING_PATH" != "$ABS_INSTALL_PATH" ]; then
        printf '%s\n' \
            "Note: an 'eng' command also exists at a different path:" \
            "  $EXISTING_PATH" \
            "That file will not be touched." \
            ""
    fi
fi

printf 'Proceed with uninstall? [y/N] '
read -r REPLY
case "$REPLY" in
    y|Y|yes|YES|Yes) ;;
    *)
        printf 'Aborted. Nothing was changed.\n'
        exit 0
        ;;
esac

# ---------------------------------------------------------------------------
# Step 3 — delete
# ---------------------------------------------------------------------------
# Use `rm -f` so a race (file vanished between the check above and
# the delete) is silent. Then re-check that the file is actually
# gone — that's our real success signal.
if ! rm -f "$INSTALL_PATH"; then
    printf '%s\n' \
        "Error: failed to remove $ABS_INSTALL_PATH." \
        "You may need elevated privileges, e.g.:" \
        "  sudo ./uninstall.sh" >&2
    exit 1
fi

if [ -e "$INSTALL_PATH" ]; then
    printf '%s\n' \
        "Error: $ABS_INSTALL_PATH still exists after rm." \
        "Check filesystem permissions." >&2
    exit 1
fi

# ---------------------------------------------------------------------------
# Step 4 — confirm
# ---------------------------------------------------------------------------
cat <<EOF

Engling uninstalled successfully.

Removed:
    $ABS_INSTALL_PATH

The 'eng' command is no longer available.

EOF
