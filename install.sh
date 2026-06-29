#!/usr/bin/env bash
# Standard installer for the Solana Vault Standard Skill.
#
# Installs a self-contained, auto-discoverable skill so agentic IDEs register it
# natively at `<config>/skills/solana-vault-standard/SKILL.md`. Installs for both
# Claude Code (`.claude/skills/`) and Cursor (`.cursor/skills/`), and drops a
# CLAUDE.md router at the project root as a universal fallback.
#
# Usage:
#   ./install.sh [TARGET_DIR]
#
# Defaults: TARGET_DIR = current directory. Zero external dependencies.

set -euo pipefail

SRC_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TARGET_DIR="${1:-$PWD}"

# shellcheck source=install-common.sh
. "$SRC_DIR/install-common.sh"

echo "Installing the Solana Vault Standard Skill into: $TARGET_DIR"

# Each agentic IDE that auto-discovers skills at <config>/skills/<name>/.
for cfg in .claude .cursor; do
  svs_install_skill "$SRC_DIR" "$TARGET_DIR/$cfg/skills"
done

svs_install_router "$SRC_DIR" "$TARGET_DIR"

echo "Done. Registered as the '$SKILL_NAME' skill for Claude Code and Cursor."
echo "Entry point: <config>/skills/$SKILL_NAME/SKILL.md"
echo "Try the command: /new-vault"
