#!/usr/bin/env bash
# Custom installer for the Solana Vault Standard Skill (full options).
#
# Installs a self-contained, auto-discoverable skill at
# `<config>/skills/<name>/SKILL.md` for each selected agentic IDE.
#
# Usage:
#   ./install-custom.sh [--target DIR] [--ide LIST] [--skill-name NAME]
#                       [--no-templates] [--no-agents] [--no-commands]
#                       [--no-rules] [--no-router]
#
# Options:
#   --target DIR      Project to install into (default: current directory).
#   --ide LIST        Comma-separated IDE config dirs to register the skill in
#                     (default: claude,cursor). Each NAME maps to
#                     <target>/.NAME/skills/<skill-name>/.
#   --skill-name NAME Skill directory name (default: solana-vault-standard).
#   --no-templates    Skip bundling the templates/ resource.
#   --no-agents       Skip bundling the agents/ resource.
#   --no-commands     Skip bundling the commands/ resource.
#   --no-rules        Skip bundling the rules/ resource.
#   --no-router       Skip writing the CLAUDE.md router at the project root.
#
# Examples:
#   ./install-custom.sh --target ../my-app
#   ./install-custom.sh --target ../my-app --ide cursor --no-templates
#
# Zero external dependencies — pure file copy.

set -euo pipefail

SRC_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# shellcheck source=install-common.sh
. "$SRC_DIR/install-common.sh"

TARGET_DIR="$PWD"
IDES="claude,cursor"
INC_TEMPLATES=1
INC_AGENTS=1
INC_COMMANDS=1
INC_RULES=1
INC_ROUTER=1

while [ $# -gt 0 ]; do
  case "$1" in
    --target) TARGET_DIR="$2"; shift 2;;
    --ide) IDES="$2"; shift 2;;
    --skill-name) SKILL_NAME="$2"; shift 2;;
    --no-templates) INC_TEMPLATES=0; shift;;
    --no-agents) INC_AGENTS=0; shift;;
    --no-commands) INC_COMMANDS=0; shift;;
    --no-rules) INC_RULES=0; shift;;
    --no-router) INC_ROUTER=0; shift;;
    -h|--help)
      grep '^#' "$0" | sed 's/^# \{0,1\}//'; exit 0;;
    *) echo "unknown option: $1" >&2; exit 1;;
  esac
done

# Assemble the resource list from the include flags.
RESOURCES=()
[ "$INC_TEMPLATES" -eq 1 ] && RESOURCES+=(templates)
[ "$INC_AGENTS" -eq 1 ]    && RESOURCES+=(agents)
[ "$INC_COMMANDS" -eq 1 ]  && RESOURCES+=(commands)
[ "$INC_RULES" -eq 1 ]     && RESOURCES+=(rules)

echo "Installing the '$SKILL_NAME' skill into: $TARGET_DIR"

# Split the comma-separated IDE list and install into each config root.
IFS=',' read -r -a ide_list <<< "$IDES"
for ide in "${ide_list[@]}"; do
  ide="${ide#.}"   # tolerate either "cursor" or ".cursor"
  [ -n "$ide" ] || continue
  echo "IDE: $ide"
  svs_install_skill "$SRC_DIR" "$TARGET_DIR/.$ide/skills" "${RESOURCES[@]}"
done

[ "$INC_ROUTER" -eq 1 ] && svs_install_router "$SRC_DIR" "$TARGET_DIR"

echo "Done. Entry point: <config>/skills/$SKILL_NAME/SKILL.md"
