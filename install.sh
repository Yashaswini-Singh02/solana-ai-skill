#!/usr/bin/env bash
# Standard installer for the Solana Vault Standard Skill.
# Installs the skill into a target project's agent config with sensible defaults.
#
# Usage:
#   ./install.sh [TARGET_DIR]
#
# Defaults: TARGET_DIR = current directory. Installs skill/, agents/, commands/,
# and rules/ under <TARGET_DIR>/.cursor/ (and symlinks CLAUDE.md). Zero external
# dependencies — pure file copy.

set -euo pipefail

SRC_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TARGET_DIR="${1:-$PWD}"

DEST="$TARGET_DIR/.cursor"
echo "Installing Solana Vault Standard Skill into: $DEST"

mkdir -p "$DEST"
for d in skill agents commands rules templates; do
  mkdir -p "$DEST/$d"
  cp -R "$SRC_DIR/$d/." "$DEST/$d/"
done

# Make the CLAUDE.md config discoverable at the project root if absent.
if [ ! -f "$TARGET_DIR/CLAUDE.md" ]; then
  cp "$SRC_DIR/CLAUDE.md" "$TARGET_DIR/CLAUDE.md"
  echo "Wrote $TARGET_DIR/CLAUDE.md"
fi

echo "Done. Entry point: $DEST/skill/SKILL.md"
echo "Try the command: /new-vault"
