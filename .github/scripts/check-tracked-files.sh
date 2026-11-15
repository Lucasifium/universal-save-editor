#!/usr/bin/env bash
# Fails when the repository (or the commits of a pull request) carries files or
# text that belong on a contributor's machine rather than in the project:
# assistant/agent working files, planning notes, tool attribution trailers,
# absolute paths into a personal user folder, and real-looking Steam account ids.
#
# Usage: check-tracked-files.sh [<base-ref>]
#   With a base ref, the commit messages in <base-ref>..HEAD are checked too.
set -uo pipefail
cd "$(git rev-parse --show-toplevel)"
self=".github/scripts/check-tracked-files.sh"
fail=0
report() { echo "::error::$1"; fail=1; }

# 1. Files that are working notes or tool state, not part of the program.
bad_files=$(git ls-files | grep -iE '(^|/)(\.claude|\.cursor|\.superpowers|\.aider[^/]*|\.codex)(/|$)|(^|/)(CLAUDE|AGENTS|GEMINI)\.md$|(^|/)docs/(superpowers|plans|specs)/|(^|/)(plans?|specs?|notes|scratch)/.*\.md$|tech[-_]?debt|handoff|(^|/)todo\.md$' || true)
[ -n "$bad_files" ] && report "Working-note or tool files are tracked:"$'\n'"$bad_files"

# 2. Text patterns in tracked text files.
check() { # <description> <perl regex>
  local hits
  hits=$(git grep -nIP "$2" -- . ":!$self" ':!Cargo.lock' ':!THIRD_PARTY_NOTICES.md' 2>/dev/null | cut -c1-200)
  [ -n "$hits" ] && report "$1:"$'\n'"$hits"
}
check "Tool attribution lines" '(?i)co-authored-by:|claude-session:|generated (with|by) \[?claude|claude\.ai/code|claude-projects-attribution'
check "Assistant workflow notes" '(?i)\b(superpowers|feature-dev|code-simplifier|ghost-writer|humanizer|design-skills|frontend-design)\b|\bproject[- ]thread\b|\bcoordinator session\b|tech[- ]debt (notes|register)'
# A user folder path is fine with a placeholder name (x, SomeOne, <user>, %USERNAME%, Public).
check "Absolute path into a personal user folder" '(?i)\b[a-z]:(\\\\|\\|/)+(users|documents and settings)(\\\\|\\|/)+(?!(x|someone|user|username|public|default|<[^>]*>|%[^%]*%|\$\w+|\{[^}]*\})(\\|/|"|$))[^\\/"\s]+'
check "Absolute path on a data drive" '(?<![A-Za-z0-9])[D-Y]:(\\\\|\\)(?!\.\.\.)[A-Za-z]'

# Steam ids: only these well-known placeholder ids may appear.
allowed='76561197960265728|76561197960265729|76561197960265730|76561198000000000|76561190000000000|76561190000000009'
ids=$(git grep -hoIP '(?<![0-9])7656119[0-9]{10}(?![0-9])' -- . ":!$self" | sort -u | grep -vxE "$allowed" || true)
[ -n "$ids" ] && report "Steam account ids that are not placeholders (use 76561197960265729):"$'\n'"$ids"

# 3. Commit messages of a pull request.
if [ $# -ge 1 ]; then
  msgs=$(git log --format='%h %B' "$1..HEAD" | grep -iE 'co-authored-by:|claude-session:|generated (with|by) \[?claude|claude\.ai/code' || true)
  [ -n "$msgs" ] && report "Commit messages carry tool attribution lines:"$'\n'"$msgs"
fi

[ $fail -eq 0 ] && echo "No stray working files, personal paths or attribution lines found."
exit $fail
