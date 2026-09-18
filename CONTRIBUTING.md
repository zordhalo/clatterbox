# Contributing to Clatterbox

Clatterbox is a small, spec-driven codebase. Before sending a PR, please read
`docs/SPEC.md` — it is the design contract (architecture, privacy rules, file ownership).
Changes that contradict it should be raised as an issue first, not diverged from quietly.

## Ground rules

- **Privacy is non-negotiable.** No logging, storing, or transmitting of which key was
  pressed, anywhere in `crates/keyhook` or `crates/core/src/keymap` / `repeat.rs`. CI runs
  `scripts/check-privacy.sh` to enforce this; it must pass.
- Every source file stays under 500 lines.
- `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings`, and
  `cargo test --workspace` must be clean on all three OSes before merge.
- No new runtime dependency without a reason in the PR description — see §1 of the spec for
  why each current dependency was chosen (and what was rejected).
- No trademarked switch or keyboard names (Cherry, MX, Gateron, Kailh, Holy Panda, etc.) in
  pack names, ids, UI copy, or docs (§0.3 of the spec).

## Sound packs

Contributing a new sample pack? See the "Sound pack format" section of the README. Packs must
ship under a license that allows redistribution (CC0-1.0 or public domain for anything bundled
in this repo); include a `CREDITS.md` mapping every file to its original source, author, and
license evidence.

## Development

```
npm install
npm run tauri dev
```

`cargo test --workspace` runs headless (no audio device, no keyboard hook needed).

## Reporting issues

Open a GitHub issue. For anything security- or privacy-related, please still use a public
issue — this project makes zero network calls and has no security contact channel beyond
GitHub.
