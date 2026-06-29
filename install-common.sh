#!/usr/bin/env bash
# Shared helpers for the Solana Vault Standard Skill installers.
#
# Builds a self-contained, auto-discoverable skill directory that agentic IDEs
# register natively from `<config>/skills/<name>/SKILL.md` — e.g. Claude Code
# (`.claude/skills/`) and Cursor (`.cursor/skills/`). The skill's focused
# sub-files sit at the skill root next to SKILL.md, and the resources it
# references (templates/agents/commands/rules) are bundled as subdirectories,
# with the cross-references flattened so they resolve inside the skill dir.
#
# Zero external dependencies — pure file copy + sed.

SKILL_NAME="${SKILL_NAME:-solana-vault-standard}"

# Flatten the skill's own sibling-relative resource paths so they resolve from
# the skill root: `../templates` -> `./templates`, etc.
_svs_rewrite_skill_root() {
  local f="$1"
  sed -e 's|\.\./templates|./templates|g' \
      -e 's|\.\./agents|./agents|g' \
      -e 's|\.\./commands|./commands|g' \
      -e 's|\.\./rules|./rules|g' \
      "$f" > "$f.svstmp" && mv "$f.svstmp" "$f"
}

# Flatten root-relative `skill/foo.md` doc references inside bundled resources to
# `../foo.md` (the skill files now live one level up at the skill root). Anchored
# on a leading backtick so repo URLs like `solana-ai-skill/` are never touched.
_svs_rewrite_resource() {
  local f="$1"
  sed -e 's|`skill/|`../|g' "$f" > "$f.svstmp" && mv "$f.svstmp" "$f"
}

# svs_install_skill SRC_DIR SKILLS_ROOT [resource ...]
# Builds <SKILLS_ROOT>/<SKILL_NAME>/ as a discoverable, self-contained skill.
# Resources default to: templates agents commands rules.
svs_install_skill() {
  local src_dir="$1"; local skills_root="$2"; shift 2
  local resources=("$@")
  [ ${#resources[@]} -eq 0 ] && resources=(templates agents commands rules)

  local dest="$skills_root/$SKILL_NAME"
  rm -rf "$dest"
  mkdir -p "$dest"

  # SKILL.md + focused sub-files sit at the skill root (the discovery entry).
  cp -R "$src_dir/skill/." "$dest/"

  # Bundle the referenced resources as subdirectories.
  local d f
  for d in "${resources[@]}"; do
    [ -d "$src_dir/$d" ] && cp -R "$src_dir/$d" "$dest/$d"
  done

  # Flatten `../resource` refs in the skill's own files...
  for f in "$dest"/*.md; do
    [ -f "$f" ] && _svs_rewrite_skill_root "$f"
  done
  # ...and `skill/` refs in the bundled resource docs (top level only).
  for d in "${resources[@]}"; do
    [ -d "$dest/$d" ] || continue
    for f in "$dest/$d"/*.md; do
      [ -f "$f" ] && _svs_rewrite_resource "$f"
    done
  done

  echo "  + $dest/SKILL.md"
}

# svs_install_router SRC_DIR TARGET_DIR
# Drops a CLAUDE.md router at the project root as a universal fallback for tools
# that read project instructions but do not auto-discover skills.
svs_install_router() {
  local src_dir="$1"; local target_dir="$2"
  if [ ! -f "$target_dir/CLAUDE.md" ]; then
    cp "$src_dir/CLAUDE.md" "$target_dir/CLAUDE.md"
    echo "  + $target_dir/CLAUDE.md"
  fi
}
