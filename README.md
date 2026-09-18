# Clatterbox

Mechanical keyboard sounds for every keystroke, on a keyboard that doesn't make any.

Clatterbox is a small, open-source tray app for Windows, macOS, and Linux. It listens to your
keyboard and plays a sample or a synthesized click on every key down and key up: stereo panning
by key position, per-keystroke pitch and gain variation, switchable sound packs, low latency,
low CPU. It's an open-source, cross-platform alternative in the same space as Klack, built from
scratch with CC0 sound sources — it does not use Klack's assets, name, or any trademarked switch
or keyboard names.

## Features

- Key-down **and** key-up sounds, with stereo panning based on where the key sits on the board.
- Per-keystroke pitch and gain variation so typing doesn't sound like a machine gun.
- Switchable sound packs with live preview, from a system tray menu or a settings window.
- Procedural (synthesized) packs that need no audio files at all — Clatterbox always makes
  sound, even if every sample pack fails to load.
- Import your own sample packs from a folder.
- Launch at login, per-OS native tray integration, settings window created on demand and
  destroyed when closed (no resident webview when you're not looking at it).
- No network access. None. See Privacy below.

## Install

Releases are unsigned at this stage (see `CHANGELOG.md` and the "Signing" note in
`docs/SPEC.md`). Each OS will warn you before running an app from an unknown developer —
that's expected for an unsigned open-source build, not a sign that something's wrong.

### Windows

1. Download the `.msi` or `.exe` installer from the [Releases](../../releases) page.
2. Windows SmartScreen will say "Windows protected your PC." Click **More info**, then
   **Run anyway**.
3. Clatterbox starts in the system tray. No further permission is needed on Windows.

### macOS

1. Download the `.dmg` from Releases and drag Clatterbox into Applications.
2. Because the build isn't notarized, Gatekeeper blocks a normal double-click. Instead,
   **right-click (or Control-click) the app → Open**, then confirm in the dialog that appears.
   This only needs to happen once per build; note that on an unsigned/ad-hoc build, updating
   Clatterbox (a new build with a new signature) means macOS will ask again.
3. On first launch, Clatterbox needs **Input Monitoring** permission to hear keystrokes — it
   does **not** need Accessibility. A banner in the settings window walks you through it:
   click "Allow keyboard access," grant the permission in System Settings → Privacy & Security
   → Input Monitoring, then relaunch Clatterbox if it asks you to (macOS sometimes requires a
   relaunch after the permission is granted before the keyboard tap can actually start).
4. Secure Input (e.g. typing into a password field) silently blocks all keyboard events system
   -wide, including Clatterbox's — that's expected macOS behavior, not a bug.

### Linux

1. Download the `.AppImage` or `.deb`/`.rpm` from Releases.
2. **X11 sessions** work out of the box — Clatterbox uses XInput2 raw key events, which any
   client can read without root.
3. **Wayland sessions** have no global keyboard API for ordinary clients. Clatterbox falls back
   to reading `/dev/input` directly via evdev, which only works if your user is already in the
   `input` group. **Clatterbox will never add you to that group, request `sudo`/`pkexec`, or
   otherwise try to escalate its own privileges** — if it can't read a keyboard device, it says
   so in the settings window and stays silent. Be aware that `input` group membership lets
   *any* program you run read *all* input devices system-wide, not just Clatterbox; only add
   yourself to it if you understand and accept that trade-off.
4. On GNOME, tray icons need the [AppIndicator/KStatusNotifierItem extension](https://extensions.gnome.org/extension/615/appindicator-support/) — GNOME dropped
   native tray icon support some time ago.

## Privacy

Clatterbox's keyboard hook never logs, stores, persists, transmits, or shows in its own
settings window *which key* you pressed.

Concretely:

- A platform key code (a Windows scan code, a macOS virtual keycode, a Linux evdev code) exists
  only as a local variable inside the platform-specific callback and the single `dispatch`
  function that all three platforms funnel through. `dispatch` immediately reduces it to a
  `KeySound { class, dir, x }` value — one of 5 coarse classes (default / space / enter /
  backspace / modifier), a direction (down/up), and a pan position on the board — and the
  original key code is dropped. Nothing past that point can recover which key it was.
- There are no per-keystroke events sent to the UI, no keystroke counters, no "typing
  statistics," and no visualizer of any kind.
- Clatterbox makes **zero network connections**. No updater, no telemetry, no crash reporter.
  The app's content security policy forbids loading anything from a remote origin.
- The types that carry a reduced key sound (`KeySound`, `Trigger`) deliberately do not
  implement Rust's `Debug` trait, specifically to make it a compile error to accidentally
  format one into a log line.
- This is enforced, not just promised: CI runs `scripts/check-privacy.sh`, which fails the
  build if any logging macro (`log::`, `tracing::`, `println!`, `eprintln!`, `dbg!`) or a
  `#[derive(Debug)]` shows up anywhere in the keyboard-hook crate or the core modules that
  handle key identity (`crates/keyhook/src/**`, `crates/core/src/keymap/**`,
  `crates/core/src/repeat.rs`).

No keystrokes are recorded, stored, or sent anywhere.

## Sound pack format

A pack is a folder with a `pack.toml` manifest and some audio files (`.wav`, `.ogg`, `.flac`,
or `.mp3`):

```
my-pack/
  pack.toml
  LICENSE.txt            (optional, recommended)
  default-down-1.wav  default-down-2.wav ...  default-up-1.wav ...
  space-down-1.wav ...   (any file names; subdirectories are fine)
```

```toml
schema = 1
name = "Slate Linear"
author = "Jane Doe"
license = "CC0-1.0"
source_url = "https://freesound.org/..."     # optional
description = "Deep, muted linear."          # optional
gain_db = 0.0                                # optional, -24.0..=12.0

[sounds.default]                             # required; "down" must be non-empty
down = ["default-down-1.wav", "default-down-2.wav"]
up   = ["default-up-1.wav"]                  # optional

[sounds.space]                               # optional: space | enter | backspace | modifier
down = ["space-down-1.wav"]
up   = ["space-up-1.wav"]
gain_db = 0.0                                # optional per-class trim, for loudness-matching mixed sources
```

Only `default.down` is required. Clatterbox derives everything else at load time: a missing
`up` set is generated from the matching `down` set (pitched down 300 cents, -9 dB, trimmed to
60% length with a short fade); a missing special class (`space`, `enter`, `backspace`,
`modifier`) is generated from `default` with a class-specific pitch/gain offset. This means a
pack with just 8-16 down-stroke recordings already sounds complete. Explicit recordings, where
you have them, are always used as-is and never mixed with derived audio.

Each key-down/up set can have up to 16 round-robin variations; Clatterbox picks one at random
per keystroke, avoiding immediate repeats. Files are limited to 4 MiB and 2 seconds each.

To make your own pack: open the settings window, click **Open packs folder**, drop your folder
in there, then **Reload**. Or use **Import pack…** to copy a folder from anywhere on disk.
Import validates the whole pack (paths, licenses, decode) before accepting it.

## Build from source

Requires a stable Rust toolchain (`rustup` will pick up `rust-toolchain.toml` automatically)
and Node.js LTS.

```sh
npm install
npm run tauri dev      # run the app
npm run typecheck      # tsc --noEmit on the settings UI
cargo test --workspace # headless tests: no audio device or keyboard hook needed
cargo clippy --workspace --all-targets -- -D warnings
```

On Linux, building needs `libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev
libasound2-dev patchelf` (see `.github/workflows/ci.yml` for the exact `apt-get` line).

## Architecture

```
                OS keyboard event (Raw Input / CGEventTap / XInput2 / evdev)
                               │  platform keycode — never leaves this box
          crates/keyhook: backend → dispatch() → KeySound{class,dir,x}
                               │  rtrb (wait-free) queue
          crates/audio: cpal output callback → crates/core mixer (32 voices,
                         pan/pitch/gain variation, cubic Hermite resampling)
                               │
          src-tauri: tray menu, settings IPC commands, settings window (Vite + TS)
```

Three Rust crates plus the Tauri shell:

- `crates/core` — pure, platform-independent logic: settings, the mixer, sound pack loading
  and derivation, procedural synth generation, the keyboard-position map, repeat filtering.
  No I/O, no platform code; its tests run headless anywhere.
- `crates/audio` — owns the `cpal` output stream and the audio-callback/owner-thread split.
- `crates/keyhook` — one thin backend per OS (Windows Raw Input, macOS CGEventTap, Linux
  XInput2 with an evdev fallback), all funneling into the shared `dispatch` function.
- `src-tauri` — the app shell: tray, IPC commands, settings persistence, the on-demand
  settings window.

See `docs/SPEC.md` for the full design (threading model, latency budget, exact algorithms).
A screenshot of the settings window is in `docs/screenshots/settings-light.png`.

## Credits

- `builtin/classic` ("Classic Office") — CC0 recordings by unicaegames, from
  [Keyboard Soundpack 1](https://opengameart.org/content/keyboard-soundpack-1-typing-and-single-keystrokes)
  on OpenGameArt.
- `builtin/tactile` ("Tactile") — CC0 recordings by StavSounds, Foxfire-, alpinemesh, and
  yottasounds, from [Freesound](https://freesound.org/people/StavSounds/packs/42151/).

Full per-file attribution is in each pack's own `CREDITS.md` under
`src-tauri/resources/packs/`. See `NOTICE` for the summary required by the Apache License.

## License

Apache License 2.0 — see `LICENSE`. Bundled CC0 audio is public domain and not covered by the
Apache License terms (see `NOTICE`).
