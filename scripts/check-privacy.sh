#!/usr/bin/env bash
# Enforces the privacy non-negotiable (spec §0.1): the keyboard hook and the
# key-identity-carrying core modules must never log, store, or format a key
# identity. Run from the repo root; exits non-zero if a forbidden pattern is
# found in the guarded paths.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

PATTERN='(log|tracing)::|println!|eprintln!|dbg!|#\[derive\([^)]*Debug'
PATHS=(
  "crates/keyhook/src"
  "crates/core/src/keymap"
  "crates/core/src/repeat.rs"
)

rc=0
if command -v rg >/dev/null 2>&1; then
  # rg's own -E flag means --encoding, not "extended regex" (rg regexes are
  # already Rust-regex, no -E needed) — passing it exits 2 (usage error),
  # which `|| true` would otherwise swallow as "no matches found".
  MATCHES=$(rg -n "$PATTERN" "${PATHS[@]}") || rc=$?
else
  echo "warning: ripgrep (rg) not found, falling back to grep -E" >&2
  MATCHES=$(grep -rnE "$PATTERN" "${PATHS[@]}") || rc=$?
fi

# Exit code 1 means "no matches" for both rg and grep — that's the pass case.
# Anything else (2+) is a real tool failure (bad pattern, missing path, etc.)
# and must not be treated as "clean".
if [[ $rc -gt 1 ]]; then
  echo "Privacy check errored (rg/grep exit code $rc) — treating as failure." >&2
  echo "$MATCHES" >&2
  exit 1
fi

if [[ -n "$MATCHES" ]]; then
  echo "Privacy check failed: forbidden pattern found in a key-identity-carrying path." >&2
  echo "See docs/SPEC.md §0.1 — no log::/tracing::/println!/eprintln!/dbg!/#[derive(Debug)]" >&2
  echo "in crates/keyhook/src, crates/core/src/keymap, or crates/core/src/repeat.rs." >&2
  echo "$MATCHES" >&2
  exit 1
fi

echo "Privacy check passed: no forbidden patterns in guarded paths."
