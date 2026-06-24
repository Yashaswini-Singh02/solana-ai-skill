#!/usr/bin/env bash
# Custom installer for the Solana Vault Standard Skill (full options).
#
# Usage:
#   ./install-custom.sh --target DIR [--dest-name NAME] [--no-templates]
#                       [--no-agents] [--no-commands] [--no-rules]
#
# Examples:
#   ./install-custom.sh --target ../my-app
#   ./install-custom.sh --target ../my-app --no-templates --dest-name .agent
#
# Zero external dependencies — pure file copy.

set -euo pipefail

SRC_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TARGET_DIR="$PWD"
DEST_NAME=".cursor"
INC_TEMPLATES=1
INC_AGENTS=1
INC_COMMANDS=1
INC_RULES=1

while [ $# -gt 0 ]; do
  case "$1" in
    --target) TARGET_DIR="$2"; shift 2;;
    --dest-name) DEST_NAME="$2"; shift 2;;
    --no-templates) INC_TEMPLATES=0; shift;;
    --no-agents) INC_AGENTS=0; shift;;
    --no-commands) INC_COMMANDS=0; shift;;
    --no-rules) INC_RULES=0; shift;;
    -h|--help)
      grep '^#' "$0" | sed 's/^# \{0,1\}//'; exit 0;;
    *) echo "unknown option: $1" >&2; exit 1;;
  esac
done

DEST="$TARGET_DIR/$DEST_NAME"
echo "Installing into: $DEST"
mkdir -p "$DEST/skill"
cp -R "$SRC_DIR/skill/." "$DEST/skill/"

copy_opt() { # name flag
  if [ "$2" -eq 1 ]; then
    mkdir -p "$DEST/$1"; cp -R "$SRC_DIR/$1/." "$DEST/$1/"; echo "  + $1"
  else
    echo "  - $1 (skipped)"
  fi
}

copy_opt agents "$INC_AGENTS"
copy_opt commands "$INC_COMMANDS"
copy_opt rules "$INC_RULES"
copy_opt templates "$INC_TEMPLATES"

if [ ! -f "$TARGET_DIR/CLAUDE.md" ]; then
  cp "$SRC_DIR/CLAUDE.md" "$TARGET_DIR/CLAUDE.md"
  echo "  + CLAUDE.md"
fi

echo "Done. Entry point: $DEST/skill/SKILL.md"
