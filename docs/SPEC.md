# Clatterbox — Implementation Spec (v1.0)

Open-source, cross-platform (Windows / macOS / Linux) tray app that plays mechanical-keyboard
sounds on every keystroke. Feature parity target: Klack (key-down + key-up samples, stereo
panning by key position, per-keystroke pitch variation, switchable packs with preview, tray
quick toggles, settings window, low latency, low CPU). Apache-2.0. Name: **Clatterbox**
(display) / `clatterbox` (crates, binary), bundle id `dev.zordhalo.clatterbox`, repo
`github.com/zordhalo/clatterbox`.

Status: **implementation-ready**. All decisions are final; §16 records the lead's rulings and
the body of this spec already reflects them. Sound sources: `research/SOUNDS.md`.
Research date for all versions: 2026-09-18 (crates.io / npm / GitHub API).

---

## 0. Non-negotiables

1. **Privacy.** The keyboard hook never logs, stores, persists, transmits, or emits to the
   webview which key was pressed. A platform keycode lives only as a local variable inside the
   platform callback and the `dispatch` function (§3.6), where it is reduced to
   `KeySound { class, dir, x }` — 5 possible classes and a pan position — and then dropped.
   Consequences that implementers must honour:
   - No `log::`, `tracing::`, `println!`, `eprintln!`, `dbg!` anywhere in `crates/keyhook/src/**`
     or `crates/core/src/keymap/**` / `repeat.rs` (enforced by `scripts/check-privacy.sh` in CI).
   - No per-keystroke Tauri events, no keystroke counters, no "typing stats", no visualizer.
   - No network access at all: no updater, no telemetry, no crash reporter; CSP forbids remote
     origins. The app makes zero outbound connections.
   - `KeySound`/`Trigger` do not implement `Debug` (prevents accidental formatting into logs).
   - README gets a "Privacy" section stating the above verbatim.
2. **Zero-asset operation.** Procedural synth packs (§5) always work; if every sample pack fails
   to load, the app still clicks.
3. **No Klack assets, name, or trademarked switch names** (Cherry, MX, NovelKeys, Gateron,
   Kailh, Holy Panda, etc.) in pack names, ids, UI copy, or docs.
4. Every source file < 500 lines. `cargo fmt`, `cargo clippy -D warnings`, `cargo test` green on
   all three OSes.

---

## 1. Chosen stack (pinned)

### Rust (edition 2024, MSRV = stable ≥ 1.88)

| Crate | Version | Used by | Why |
|---|---|---|---|
| `tauri` | `2.11` features `["tray-icon", "image-png"]` | app | Decided stack. |
| `tauri-build` | `2.6` | app (build) | |
| `tauri-plugin-single-instance` | `2.4` | app | Second launch focuses settings. |
| `tauri-plugin-autostart` | `2.5` | app | Launch at login (LaunchAgent on macOS). |
| `tauri-plugin-dialog` | `2.7` | app + UI | Folder picker for pack import. |
| `tauri-plugin-opener` | `2.5` | app | Open packs folder / macOS privacy pane URL. |
| `cpal` | `0.18` (default features → WASAPI / CoreAudio / ALSA) | audio | Direct device access, callback-driven, no intermediate mixer thread. |
| `rtrb` | `0.4` | audio, keyhook | Wait-free SPSC ring buffers into the audio callback. |
| `symphonia` | `0.6`, `default-features = false`, features `["wav","pcm","ogg","vorbis","flac","mp3"]` | core | Pack decoding at load time only (most researched CC0 samples are MP3). |
| `toml` | `1.1` | core | `pack.toml`. |
| `serde` / `serde_json` | `1.0` / `1.0` | core, app | Settings + IPC. |
| `thiserror` | `2.0` | all | Error enums. |
| `fastrand` | `2.5` | core | Seedable, alloc-free RNG usable in the audio callback. |
| `parking_lot` | `0.12` | audio, app | `Mutex::try_lock` in callback without poisoning. |
| `tracing` + `tracing-subscriber` | `0.1` / `0.3` | app, audio, core (NOT keyhook) | stderr logs, level `info`. |
| `windows` | `0.62` (same major as `tao`/`cpal` → no duplicate) | keyhook (Windows) | Raw Input. |
| `objc2-core-graphics` / `objc2-core-foundation` | `0.3` | keyhook (macOS) | CGEventTap + CFRunLoop; same family as `tao`. |
| `x11rb` | `0.14` features `["xinput"]` | keyhook (Linux) | XInput2 raw key events, pure Rust (no libX11). |
| `evdev` | `0.13` | keyhook (Linux) | Wayland fallback via `/dev/input`. |
| `libc` | `0.2` | keyhook (Linux) | `poll(2)` over evdev fds. |

Explicitly **rejected**: `rdev` (last release 0.5.3, 2023-06; on macOS its listener calls TSM
key-translation APIs that crash off the main thread on macOS 14+; unmaintained forks with
<1k downloads), `rodio`/`kira` (see §4.1), `device_query` (polling), `global-hotkey`/`handy-keys`
(shortcut registration, not a raw key stream).

### Frontend

| Package | Version |
|---|---|
| `@tauri-apps/cli` (dev) | `^2.11.4` |
| `@tauri-apps/api` | `^2.11.1` |
| `@tauri-apps/plugin-dialog` | `^2.7.3` |
| `vite` (dev) | `^8.3.0` |
| `typescript` (dev) | `~5.9` (TS 7 native compiler is new; revisit after v0.1) |

**Vanilla TypeScript, no framework.** One window, ~10 controls, no routing, no shared client
state beyond one `Settings` object. A framework would add runtime weight and a second mental
model for a UI that is ~400 lines of TS. Typecheck = `tsc --noEmit`.

### CI

`actions/checkout@v7`, `actions/setup-node@v7`, `dtolnay/rust-toolchain@stable`,
`Swatinem/rust-cache@v2`, `tauri-apps/tauri-action@v1`.

---

## 2. Architecture

### 2.1 Diagram

```
                ┌──────────────────────────── OS ───────────────────────────────┐
                │ Windows: Raw Input (WM_INPUT, RIDEV_INPUTSINK)                 │
                │ macOS:   CGEventTap (listen-only, kCGHIDEventTap)              │
                │ Linux:   X11 XI2 RawKeyPress/Release  | evdev /dev/input/*     │
                └──────────────┬────────────────────────────────────────────────┘
                               │ platform keycode (never leaves this box ↓)
          ┌────────────────────▼─────────────────────┐   keyhook thread
          │ crates/keyhook  backend → PhysKey         │
          │   dispatch(): RepeatFilter → layout()     │── HookStatus ──┐
          │   → KeySound{class,dir,x}  (code dropped) │                │
          └────────────────────┬─────────────────────┘                │
                               │ TriggerTx (rtrb SPSC, wait-free)      │
                               ▼                                       │
┌─────────────────────── crates/audio ──────────────────────────┐      │
│  cpal output callback (RT thread)                              │      │
│   ├─ drain Hook queue, Preview queue, Control queue            │      │
│   ├─ Mixer::render  (core::mixer: 32 voices, resample+pan)     │      │
│   └─ retired packs → Garbage queue ─────────┐                  │      │
│  audio-owner thread: owns cpal::Stream, rebuilds on error /    │      │
│   default-device change, drops garbage, reports AudioStatus ───┼──┐   │
└────────────────────────────────────────────────────────────────┘  │   │
        ▲ set_pack / preview (Control queue)      EngineParams (atomics) │
        │                                            ▲                  │
┌───────┴────────────── src-tauri (main thread) ─────┴──────────────────▼──┐
│ AppState { settings, params, engine, hook, registry }                     │
│ tray.rs (menu) · commands.rs (IPC) · persist.rs (debounced JSON writer)   │
│ packs.rs (registry: synth/ builtin/ user/) · window.rs (on-demand webview)│
└───────────────┬───────────────────────────────────────────────────────────┘
                │ invoke / events: settings-changed, packs-changed, status-changed
        ┌───────▼────────┐
        │ ui/ (Vite+TS)  │  Settings window — created on demand, destroyed on close
        └────────────────┘
```

### 2.2 Threads

| Thread | Owner | Blocking allowed? | Does |
|---|---|---|---|
| main | Tauri | yes (event loop) | tray, IPC commands, window |
| keyhook | `keyhook::start` | yes (OS message/run loop) | receive OS events → `dispatch` → push `Trigger` |
| audio callback | cpal | **no** (no alloc, no lock waits, no syscalls, no logging) | mix |
| audio-owner | `audio::AudioEngine` | yes | stream lifecycle, device watch (3 s tick), garbage drop |
| persist | `persist.rs` | yes | debounced settings write (500 ms) |
| preview | spawned per preview | yes (sleep) | schedules the preview phrase into the Preview queue |

### 2.3 Hot path budget (key press → sound)

OS delivery (≤1 ms) + dispatch (<5 µs) + queue (wait-free) + wait for next callback (≤ 1 buffer)
+ device/driver latency. Target buffer 256 frames @ 48 kHz = 5.3 ms.

| Platform | Target end-to-end | Notes |
|---|---|---|
| macOS (CoreAudio) | ≤ 12 ms | Fixed 256-frame buffer normally accepted. |
| Linux (ALSA via pipewire-alsa/pulse) | ≤ 15 ms | Fixed 256 frames, falls back to Default. |
| Windows (WASAPI shared) | ≤ 20 ms | Shared-mode engine period (~10 ms) dominates; cpal does not expose IAudioClient3 low-latency. Accept. |

CPU: idle (not typing) < 0.1 % of one core; sustained 15 keys/s < 0.5 %. RSS without the
settings window < 40 MB (webview is destroyed when the window closes).

---

## 3. Keyboard capture

### 3.1 Decision

Own thin backends per OS inside `crates/keyhook` (no third-party hook crate). All backends
produce `(PhysKey, KeyDir, Instant)` and call the shared `dispatch` (§3.6). Auto-repeat is
suppressed **uniformly in core** (`RepeatFilter`), not by trusting OS repeat flags (Windows Raw
Input has none; macOS modifiers arrive as FlagsChanged).

### 3.2 Windows — Raw Input (chosen) over WH_KEYBOARD_LL

- Dedicated thread creates a message-only window (`CreateWindowExW` with parent `HWND_MESSAGE`),
  then `RegisterRawInputDevices` with `usUsagePage=0x01, usUsage=0x06, dwFlags=RIDEV_INPUTSINK,
  hwndTarget=hwnd`. Loop `GetMessageW`/`DispatchMessageW`; in `WM_INPUT` call
  `GetRawInputData(RID_INPUT)` into a stack buffer (`RAWINPUT` is fixed-size for keyboards).
- Fields: `RAWKEYBOARD.MakeCode` (scan code), `Flags & RI_KEY_BREAK` (up), `RI_KEY_E0` (extended).
  Map `(MakeCode, E0)` → `PhysKey` (scan codes are layout-independent = physical position).
  Ignore `MakeCode == 0xFF` / `0x00` (overrun / fake) and the `E1` Pause prefix (map Pause via
  VKey fallback `VK_PAUSE`).
- Why not `WH_KEYBOARD_LL`: an LL hook sits *in* the system input chain — any stall in our thread
  delays every keystroke system-wide, and Windows silently unhooks after `LowLevelHooksTimeout`.
  Raw Input is asynchronous (can't slow the user's typing) and `SetWindowsHookEx(WH_KEYBOARD_LL)`
  is a heavier AV heuristic signal than `RegisterRawInputDevices`.
- Stop: `PostMessageW(hwnd, WM_CLOSE)` from `HookHandle::stop`, then join.
- `windows` features: `Win32_Foundation`, `Win32_UI_WindowsAndMessaging`,
  `Win32_UI_Input`, `Win32_UI_Input_KeyboardAndMouse`, `Win32_System_LibraryLoader`.
- Not received while the secure desktop (UAC, Ctrl+Alt+Del) is active — expected; `RepeatFilter`
  staleness rule (§3.6) handles the missing key-ups.

### 3.3 macOS — CGEventTap, listen-only

- Thread creates `CGEventTapCreate(kCGHIDEventTap, kCGHeadInsertEventTap,
  kCGEventTapOptionListenOnly, mask(KeyDown|KeyUp|FlagsChanged), callback, ctx)`, wraps it in a
  `CFMachPort` run-loop source, adds to this thread's `CFRunLoop`, runs `CFRunLoopRun()`.
  Stop = `CFRunLoopStop` on the stored run loop ref.
- Callback: if type is `kCGEventTapDisabledByTimeout`/`ByUserInput` → `CGEventTapEnable(tap,
  true)` and return. Keycode = `CGEventGetIntegerValueField(ev, kCGKeyboardEventKeycode)`
  (virtual keycodes `kVK_*` are physical positions). Return the event unmodified (listen-only).
- **FlagsChanged** (modifiers, Caps Lock): keycode identifies the modifier; direction = the
  modifier's device-dependent flag bit is set → Down, cleared → Up (`NX_DEVICELSHIFTKEYMASK
  0x02`, `RSHIFT 0x04`, `LCTL 0x01`, `RCTL 0x2000`, `LALT 0x20`, `RALT 0x40`, `LCMD 0x08`,
  `RCMD 0x10`). Caps Lock: emit Down on every FlagsChanged for keycode 57, followed by Up after
  it (no reliable release).
- Permission (Input Monitoring, `kTCCServiceListenEvent`). A listen-only tap needs **Input
  Monitoring only**, not Accessibility. Declare (not in objc2 bindings if missing):
  ```rust
  unsafe extern "C" { fn CGPreflightListenEventAccess() -> bool; fn CGRequestListenEventAccess() -> bool; }
  ```
  Flow:
  1. At start: `CGPreflightListenEventAccess()`; false → status `NeedsPermission`, do not create
     the tap, open the settings window (app side) showing the permission banner.
  2. Banner "Allow keyboard access" → command `request_input_permission` → calls
     `CGRequestListenEventAccess()` (system prompt on first call) **and** opens
     `x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent`.
  3. Hook thread re-checks preflight every 2 s while `NeedsPermission`. When it flips to true,
     try creating the tap; if `CGEventTapCreate` returns null (macOS often requires relaunch
     after grant) → status `NeedsRestart`; banner shows "Relaunch Clatterbox" → `app.restart()`.
- Notes for README: permission is bound to the code signature; ad-hoc/unsigned dev builds
  re-prompt after each rebuild. Secure Input (password fields) blocks events — silent, expected.
- `objc2-core-graphics` features: `CGEvent`, `CGEventTypes`, `CGRemoteOperation`, `CGEventSource`
  (verify exact feature names on docs.rs 0.3.2 — the WP3 owner adjusts); `objc2-core-foundation`
  features `CFRunLoop`, `CFMachPort`.

### 3.4 Linux — X11 XInput2 raw events, evdev fallback for Wayland

Backend selection at start:
1. `XDG_SESSION_TYPE == "x11"` or (`DISPLAY` set and `WAYLAND_DISPLAY` unset) → **X11 backend**.
2. Else (Wayland) → **evdev backend**, which activates only if the process can already open
   keyboard devices (user is in the `input` group). The app never requests privileges, never
   runs `pkexec`/`sudo`, and never modifies groups. If no keyboard device is readable (EACCES)
   → status `NeedsPermission { hint: LINUX_INPUT_GROUP_HINT }` (informational text only).
3. If X11 backend fails to connect / no XInput 2.0 → try evdev.

**X11**: `x11rb::connect(None)`, `xinput_xi_query_version(2, 0)`, `xinput_xi_select_events` on
the root window with `deviceid = XIAllMasterDevices`, mask `XI_RawKeyPress | XI_RawKeyRelease`.
Loop `wait_for_event()`; `RawKeyPress/RawKeyRelease.detail` = X keycode → evdev code =
`detail - 8`. Works for all clients on X11 without root.

**evdev**: enumerate `/dev/input/event*`, keep devices whose `supported_keys()` contain `KEY_A`
and `KEY_SPACE` (keyboards; skips mice/power buttons). Single thread, `libc::poll` over all fds
(set non-blocking), `fetch_events()` on readable fds; `EV_KEY` value 1 = Down, 0 = Up,
2 = OS repeat (ignore). Rescan `/dev/input` every 3 s for hot-plug (poll timeout 3000 ms).
Never grab devices (`EVIOCGRAB` forbidden).

`LINUX_INPUT_GROUP_HINT` (UI copy): "On Wayland, Clatterbox can only hear keys if your user can
read keyboard devices (the `input` group). Clatterbox will not change this for you. Be aware
that `input` group membership lets any program you run read all input devices. See the README
for details."

### 3.5 Keycode → PhysKey tables

`PhysKey` is a `#[repr(u8)]` enum of physical positions on an ANSI/ISO 105-key board (named by
US-ANSI legends; ISO extras `IntlBackslash`, `IntlRo`, `IntlYen`; `Unknown`). Tables:
`windows/scancode.rs` (set-1 scan codes + E0), `macos/keycode.rs` (`kVK_*`),
`linux/keycode.rs` (evdev `KEY_*`, shared by X11 via −8). Unknown codes → `PhysKey::Unknown`
(plays `Default` class at x = 0.5).

### 3.6 Dispatch + RepeatFilter (core, shared)

```rust
// crates/keyhook/src/dispatch.rs
pub(crate) struct Dispatcher { filter: RepeatFilter, params: Arc<EngineParams>, tx: TriggerTx }
impl Dispatcher {
    /// The ONLY place a key identity is converted. Must not log, store, or clone `key`.
    #[inline] pub(crate) fn dispatch(&mut self, key: PhysKey, dir: KeyDir, now_ms: u64);
}
```
`dispatch`: `if !filter.accept(key, dir, now_ms) { return }` → if `!params.enabled` return →
if `dir == Up && !params.key_up` return → `let (class, x) = layout::lookup(key)` →
`tx.push(Trigger::key(KeySound { class, dir, x }))` (on full queue: drop silently).

`RepeatFilter` (`crates/core/src/repeat.rs`): `held: [u64; 2]` bitset over `PhysKey as u8`,
`last_ms: [u64; 128]`.
- Down: if held and `now - last_ms[k] <= 1200` → it's a repeat: update `last_ms`, reject.
  If held but gap > 1200 ms → stale (missed key-up): accept as fresh. Else mark held, accept.
- Up: if held → clear, accept. If not held → reject (orphan up, e.g. key pressed before launch).
- `reset()` clears all (called when a backend restarts).
(1200 ms > Windows' max initial repeat delay of 1000 ms; subsequent repeats are ≤ 400 ms apart.)

---

## 4. Audio engine

### 4.1 Decision: `cpal` directly + own mixer

- **rodio**: sink/source graph, per-sound allocation, extra mixer thread and its own latency;
  pan/pitch per voice means building a Source chain per keystroke (allocations). No.
- **kira**: good game-audio engine, but brings its own clocking/command model and a larger
  dependency tree for what is a 32-voice sample player; our mixer is ~300 lines and fully
  testable as a pure function.
- **cpal**: lowest-latency path, the callback *is* the mixer, and the mixer lives in
  `crates/core` with zero I/O so `cargo test` covers it headless.

### 4.2 Stream configuration (`crates/audio/src/stream.rs`)

1. `cpal::default_host().default_output_device()`; none → `AudioStatus::NoDevice`, retry on tick.
2. Pick config: prefer `SampleFormat::F32`, 2 channels, rate from `default_output_config()`
   (cpal 0.17+ prefers 48 kHz). Accept any channel count ≥ 1 (mono = (L+R)/2; >2 = write ch0/ch1,
   zero rest). Supported formats: F32, I16, I32 (convert in callback, stack-free loop).
3. `BufferSize::Fixed(256)` if within `SupportedBufferSize::Range`, else `BufferSize::Default`.
   If `build_output_stream` with Fixed fails, retry once with Default.
4. `stream.start()` (0.18 name; `play()` deprecated). Streams no longer auto-start (0.17+).
5. Error callback → send `OwnerMsg::StreamError` (std mpsc; the error callback is not the RT
   data callback). Owner drops the stream and rebuilds after 250 ms.
6. Device watch: owner tick every 3 s compares `default_output_device().id()` with the current;
   on change → rebuild (cpal has no default-device-change notification; WASAPI 0.18.2 no longer
   emits `DeviceChanged`).
7. Before building a new stream the owner locks the mixer (no stream alive → uncontended) and
   calls `mixer.set_output_rate(rate)`.

`cpal::Stream` is `!Send` on some backends → created, owned and dropped on the audio-owner thread
only.

### 4.3 Queues

| Queue | Type | Producer | Consumer | Capacity |
|---|---|---|---|---|
| Hook | `rtrb<Trigger>` | keyhook thread (`TriggerTx`) | callback | 256 |
| Preview | `rtrb<Trigger>` | preview thread (behind `Mutex<TriggerTx>` in engine) | callback | 64 |
| Control | `rtrb<MixerCmd>` | main thread (behind `Mutex`) | callback | 16 |
| Garbage | `rtrb<Arc<LoadedPack>>` | callback | owner thread (drops) | 16 |

The mixer state lives in `Arc<parking_lot::Mutex<MixerHost>>`; the data callback does
`try_lock()` and outputs silence if it fails (only happens during a rebuild). `MixerHost` holds
the `Mixer`, the three consumers and the garbage producer.

### 4.4 Mixer (`crates/core/src/mixer/`)

- `MAX_VOICES = 32`. Voice: `{ slot: PackSlot, class, dir, var: u8, pos: f64, step: f64,
  gain_l: f32, gain_r: f32, fade: u16 }` — references samples by index into the pack slot; no
  Arc clones in the callback.
- Allocation: first idle voice; if none, steal the voice with the largest `pos/len` (oldest).
  Stolen voice is replaced immediately (a 32-voice steal is inaudible under typing).
- Variation choice: uniform random among the set's variations, avoiding the immediately
  previous index for that class+dir when `len > 1`.
- **Pitch**: `max_cents = 150.0 * pitch_variation`; `cents = max_cents * (r1 + r2 - 1.0)`
  (triangular in ±max); `ratio = 2^(cents/1200)`; `step = ratio * sample.rate / out_rate`.
- **Gain variation**: `±1.0 dB * pitch_variation` (same triangular draw).
- **Pan (equal power)**: `p = if spatial { ((x * 2.0 - 1.0) * width).clamp(-1, 1) } else { 0 }`;
  `θ = (p + 1.0) * π/4`; `gain_l = cos θ`, `gain_r = sin θ` (so `l² + r² = 1`).
- **Interpolation**: 4-point cubic Hermite on mono `f32` source; reads past end = 0.
- **Master**: `gain = volume² * pack.gain * 0.9`; after summing all voices, soft clip
  `y = x / (1 + |x|·0.25)` only when `|x| > 0.8` region (implement as smooth knee in
  `math::soft_clip`), guaranteeing `|y| ≤ 1`.
- Pack swap (`MixerCmd::SetPack { slot, pack }`): all voices in that slot get `fade = 64`
  samples (linear fade-out), after which the old `Arc` is pushed to Garbage. If Garbage is full,
  keep the old pack in a 4-entry pending array and retry next callback (never drop in callback).
- Triggers with `slot = Preview` ignore `params.enabled` and `params.key_up`.

### 4.5 Public types & signatures

```rust
// crates/core/src/params.rs
pub struct EngineParams {
    pub enabled: AtomicBool, pub key_up: AtomicBool, pub spatial: AtomicBool,
    volume: AtomicU32, pitch_variation: AtomicU32, spatial_width: AtomicU32, // f32 bits
}
impl EngineParams {
    pub fn from_settings(s: &Settings) -> Self;
    pub fn apply(&self, s: &Settings);            // Relaxed stores
    pub fn volume(&self) -> f32; pub fn pitch_variation(&self) -> f32; pub fn spatial_width(&self) -> f32;
}

// crates/core/src/mixer/mod.rs
pub const MAX_VOICES: usize = 32;
pub enum MixerCmd { SetPack { slot: PackSlot, pack: Arc<LoadedPack> } }
pub struct Mixer { /* voices, packs: [Option<Arc<LoadedPack>>; 2], rng: fastrand::Rng, out_rate, params */ }
impl Mixer {
    pub fn new(params: Arc<EngineParams>, out_rate: u32, seed: u64) -> Self;
    pub fn set_output_rate(&mut self, out_rate: u32);
    /// Returns the retired pack (caller routes it to the garbage queue).
    pub fn apply(&mut self, cmd: MixerCmd) -> Option<Arc<LoadedPack>>;
    pub fn trigger(&mut self, t: Trigger);
    /// Interleaved output, overwrites `out`. Never allocates.
    pub fn render(&mut self, out: &mut [f32], channels: usize);
    pub fn active_voices(&self) -> usize;
}

// crates/core/src/types.rs (so keyhook does not depend on audio); re-exported by crates/audio
pub struct TriggerTx(pub rtrb::Producer<Trigger>);
impl TriggerTx { #[inline] pub fn push(&mut self, t: Trigger) -> bool; } // false = queue full, dropped

// crates/audio/src/lib.rs
pub struct AudioEngine { /* owner thread handle, control tx, preview tx, status */ }
impl AudioEngine {
    /// Spawns the owner thread and opens the default device. Returns the Hook producer.
    /// Never fails hard: device problems are reported via status + on_status.
    pub fn start(params: Arc<EngineParams>, on_status: Box<dyn Fn(AudioStatus) + Send + Sync>)
        -> (AudioEngine, TriggerTx);
    pub fn set_pack(&self, slot: PackSlot, pack: Arc<LoadedPack>);
    pub fn preview(&self, t: Trigger) -> bool;
    pub fn status(&self) -> AudioStatus;
    pub fn shutdown(self);
}
```

### 4.6 Shared contract types — `crates/core/src/types.rs` (created verbatim in Phase 0)

```rust
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyClass { Default = 0, Space = 1, Enter = 2, Backspace = 3, Modifier = 4 }
impl KeyClass {
    pub const COUNT: usize = 5;
    pub const ALL: [KeyClass; 5] = [Self::Default, Self::Space, Self::Enter, Self::Backspace, Self::Modifier];
    pub fn as_str(self) -> &'static str; // "default" | "space" | "enter" | "backspace" | "modifier"
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum KeyDir { Down, Up }

/// Privacy boundary: carries no key identity. Intentionally NOT Debug.
#[derive(Clone, Copy)]
pub struct KeySound { pub class: KeyClass, pub dir: KeyDir, /** 0.0 = far left .. 1.0 = far right */ pub x: f32 }

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PackSlot { Main = 0, Preview = 1 }

/// Intentionally NOT Debug.
#[derive(Clone, Copy)]
pub struct Trigger { pub sound: KeySound, pub slot: PackSlot }
impl Trigger {
    pub fn key(sound: KeySound) -> Self { Self { sound, slot: PackSlot::Main } }
    pub fn preview(sound: KeySound) -> Self { Self { sound, slot: PackSlot::Preview } }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum HookStatus {
    Starting,
    Running { backend: String },            // "raw_input" | "event_tap" | "x11_xi2" | "evdev"
    NeedsPermission { hint: String },
    NeedsRestart,                           // macOS: granted, relaunch required
    Unsupported { reason: String },
    Failed { reason: String },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum AudioStatus {
    Starting,
    Running { device: String, sample_rate: u32, buffer_frames: Option<u32> },
    NoDevice,
    Failed { reason: String },
}
```

---

## 5. Sound packs

### 5.1 Pack ids and locations

| Id form | Source | Location |
|---|---|---|
| `synth/<preset>` | procedural (§6) | in code: `synth/thock`, `synth/click` |
| `builtin/<dir>` | bundled CC0 samples | Tauri resource dir `packs/<dir>/` |
| `user/<dir>` | imported by user | `<app_config_dir>/packs/<dir>/` |

`<dir>` must match `^[a-z0-9][a-z0-9-]{0,47}$`; other directories are skipped with a warning.
`app_config_dir` = Tauri `app.path().app_config_dir()` (e.g. `%APPDATA%\dev.zordhalo.clatterbox`,
`~/Library/Application Support/dev.zordhalo.clatterbox`, `~/.config/dev.zordhalo.clatterbox`).

### 5.2 Directory layout + `pack.toml`

```
my-pack/
  pack.toml
  LICENSE.txt            (optional; recommended)
  default-down-1.wav  default-down-2.wav ... default-up-1.wav ...
  space-down-1.wav ...   (any file names; subdirectories allowed)
```

```toml
schema = 1                                   # required, must be 1
name = "Slate Linear"                        # required, 1..=48 chars
author = "Jane Doe"                          # required, 1..=96 chars
license = "CC0-1.0"                          # required, SPDX id or "public-domain"
source_url = "https://freesound.org/..."     # optional, must start with https://
description = "Deep, muted linear."          # optional, <= 200 chars
gain_db = 0.0                                # optional, clamped to -24.0..=12.0

[sounds.default]                             # required, down must be non-empty
down = ["default-down-1.wav", "default-down-2.wav"]
up   = ["default-up-1.wav"]                  # optional

[sounds.space]                               # optional: space | enter | backspace | modifier
down = ["space-down-1.wav"]
up   = ["space-up-1.wav"]
gain_db = 0.0                                # optional per-class trim, -24.0..=12.0 (loudness-match mixed sources)
```

### 5.2.1 Resolution and derived samples (`crates/core/src/pack/derive.rs`, computed once at load)

Most real packs have many key-down takes and few or no key-ups / special keys. Missing sets
are **derived offline at load time into real buffers**, so the audio callback never does extra
work. Derivation = offline resample of a source `Sample` by a fixed cents offset (same 4-point
Hermite as the mixer; output length = `len / ratio`), then gain, then optional truncation.

Resolution per class `c` (explicit always wins and is never mixed with derived):

| Set | If explicit samples exist | Otherwise |
|---|---|---|
| `c.down` (c ≠ default) | explicit `c.down` | **derived** from `default.down` with the class offset below |
| `default.up` | explicit `default.up` | **derived up** from `default.down` |
| `c.up` (c ≠ default) | explicit `c.up` | **derived up** from the resolved `c.down` (explicit or derived) |

Class offsets for derived `down` (from `default.down`):

| Class | Pitch | Gain |
|---|---|---|
| space | −200 cents | +1 dB |
| enter | −150 cents | +1 dB |
| backspace | −50 cents | 0 dB |
| modifier | +100 cents | −2 dB |

Derived `up` (from a down set): pitch **−300 cents**, **−9 dB**, truncate to **60 %** of the
resampled length with a **5 ms** linear fade-out at the cut.

Rules: derive from at most the first 16 source variations; derivation runs on the already
decoded/trimmed buffers (§5.3); a derived set's effective `gain_db` = source class `gain_db` +
the offset gain. `PackInfo.derived: Vec<String>` lists derived sets (e.g.
`["space.down", "default.up"]`) for a settings-UI tooltip. Synth packs generate every set
natively and never use this path. Because `c.up` derives from `c`'s own down, ups stay
timbrally matched to their downs; a pack's single real release sample is used only for
`default.up`.

### 5.3 Validation rules (`manifest.rs` + `decode.rs`)

- Unknown top-level keys and unknown sound classes → error (`#[serde(deny_unknown_fields)]`).
- Paths: relative, UTF-8, no `..` component, no absolute/drive prefix; after join,
  `canonicalize()` must stay under the canonicalized pack dir (blocks symlink escape).
- Extensions: `.wav`, `.ogg`, `.flac`, `.mp3` (case-insensitive).
- Per file ≤ 4 MiB, decoded duration ≤ 2.0 s, sample rate 8 000..=192 000; per class+dir ≤ 16
  files; whole pack decoded ≤ 32 MiB of f32.
- Decoding: symphonia probe → first audio track → decode all packets → mix channels to mono
  `f32` → **trim leading silence**: first sample whose magnitude exceeds `file_peak × 10^(−30/20)`
  (−30 dB relative to the file's own peak — robust to MP3 priming samples and noisy preview
  floors), keep 1 ms pre-roll. This is a latency requirement: MP3 decoders emit ~25–50 ms of
  priming silence that would be heard as lag. Trim the tail below −60 dB relative to peak
  (keep 10 ms, 5 ms fade). Then peak-normalize to −1 dBFS **only** if the file peaks above
  0 dBFS (otherwise keep authored levels; cross-source loudness matching uses per-class
  `gain_db`).
- Errors are collected per pack into `PackError { pack_id, message }`; an invalid pack is listed
  with `valid: false` and its error, but cannot be selected.
- Builtin packs additionally must have `license ∈ {"CC0-1.0", "public-domain"}` and a
  `CREDITS.md` in the pack dir (test-enforced).

### 5.4 Builtin packs at v0.1 (WP5 assembles from `research/raw-sounds/`)

Pack dirs contain the **original CC0 files** renamed to neutral names (no re-encode),
`pack.toml`, and `CREDITS.md` mapping every shipped file → original filename, author, source
URL, license. No brand or switch-model names in `name`, `description`, or file names (§0.3);
recording-gear facts may appear in `CREDITS.md` only as quoted source metadata.

**`builtin/classic` — "Classic Office"** (OpenGameArt, unicaegames, CC0)
- `default.down`: `raw-sounds/opengameart-unicaegames/extracted/Single Keys/keypress-001.wav`
  … `keypress-032.wav` → `down-01.wav` … `down-32.wav`.
- Everything else derived (§5.2.1). `author = "unicaegames"`,
  `source_url = "https://opengameart.org/content/keyboard-soundpack-1-typing-and-single-keystrokes"`.
- Not shipped in v0.1: the continuous "Human Typing" / "Generated Typing" recordings (need slicing).

**`builtin/tactile` — "Tactile"** (Freesound, CC0, mixed authors)
- `default.down`: the 12 `raw-sounds/freesound-stavsounds/7666xx.mp3` → `down-01.mp3` … `down-12.mp3`.
- `default.up`: `raw-sounds/freesound-foxfire/570755_key-release.mp3` → `up-01.mp3`.
- `enter.down`: `raw-sounds/freesound-alpinemesh/627647_enter-key.mp3` → `enter-down-01.mp3`.
- `backspace.down`: `raw-sounds/freesound-yottasounds/380141_single-key.mp3` → `backspace-down-01.mp3`.
- Space, modifier, and all non-default ups derived (§5.2.1).
- `author = "StavSounds, Foxfire-, alpinemesh, yottasounds"`,
  `source_url = "https://freesound.org/people/StavSounds/packs/42151/"`.
- WP5 sets `enter`/`backspace` `gain_db` by ear so the non-StavSounds takes sit level with the
  pack, and records the values in `CREDITS.md`.

**`synth/thock`, `synth/click`** — procedural (§6), always available.

**Default pack: `builtin/classic`**; startup falls back to `synth/thock` if it fails to load.

### 5.5 Types

```rust
// crates/core/src/pack/mod.rs
#[derive(Clone, Debug, Serialize)]
pub struct PackInfo {
    pub id: String, pub name: String, pub author: String, pub license: String,
    pub source_url: Option<String>, pub description: Option<String>,
    pub kind: PackKind,                  // "synth" | "builtin" | "user" (serde snake_case)
    pub valid: bool, pub error: Option<String>,
    pub derived: Vec<String>,            // e.g. ["space.down", "default.up"]; empty for synth
}
pub struct Sample { pub data: Box<[f32]>, pub rate: u32 }
pub struct SampleSet { pub down: Vec<Sample>, pub up: Vec<Sample> }
pub struct LoadedPack { pub info: PackInfo, pub gain: f32, sets: [SampleSet; KeyClass::COUNT], /* resolved fallback idx */ }
impl LoadedPack {
    pub fn samples(&self, class: KeyClass, dir: KeyDir) -> &[Sample]; // fallback-resolved, may be empty for Up
}

// crates/core/src/pack/registry.rs
pub struct PackRegistry { builtin_dir: Option<PathBuf>, user_dir: PathBuf, infos: Vec<PackInfo> }
impl PackRegistry {
    pub fn new(builtin_dir: Option<PathBuf>, user_dir: PathBuf) -> Self;
    pub fn rescan(&mut self) -> &[PackInfo];            // manifest-only parse, cheap
    pub fn list(&self) -> &[PackInfo];                   // builtin, then user, then synth; by name (§16)
    pub fn load(&self, id: &str, synth_rate: u32) -> Result<LoadedPack, PackError>;
    /// Validates `src` fully (decode included), then copies into user_dir/<dir name>.
    /// Fails if the id exists. Returns the new info.
    pub fn import_dir(&mut self, src: &Path) -> Result<PackInfo, PackError>;
}
```

The app keeps an LRU of at most 3 `Arc<LoadedPack>` (current + recent previews).

---

## 6. Procedural synth packs (`crates/core/src/synth/`)

Generated at startup (and on output-rate change) at 48 kHz mono, deterministic from a fixed
seed so tests are stable. Budget: < 10 ms total generation.

### 6.1 Building blocks (`dsp.rs`)

- `Noise` (xorshift32, white), `Biquad` (RBJ cookbook: bandpass constant-0dB-peak, lowpass,
  highpass), `exp_env(len, tau_ms)`, `SineSweep { f0, f1, tau }`, `tanh` soft saturation,
  `normalize_peak(buf, dbfs)`, `fade_out(buf, ms)`.

### 6.2 Voice recipe (per sample)

```
excite  = noise * exp_env(τ = 0.6..1.2 ms)                      // sharp impulse, ~4 ms long
click   = bandpass(excite, fc = C_hz,  Q = 0.8) * g_click       // keycap contact
body    = Σ_i bandpass(excite, fc = M_i, Q = 12..25) * g_i      // 2-3 modal resonances, decays set by Q
thump   = sine_sweep(f0 = T_hz*1.6 → T_hz, τ = 6 ms) * exp_env(τ = 8..14 ms) * g_thump  // bottom-out
out     = tanh(1.4 * (click + body + thump)) ; highpass(40 Hz) ; fade_out(3 ms)
length  = 90..160 ms ; normalize_peak(-3 dBFS) then apply per-class level
```

### 6.3 Presets

| Param | `synth/thock` (muted linear) | `synth/click` (tactile-click) |
|---|---|---|
| C_hz click band | 2 400 | 4 800 (Q 3.0, plus 2nd impulse 4 ms later at −4 dB) |
| Modes M_i (Hz) | 210, 620, 1 350 | 380, 1 100, 2 900 |
| Mode Q | 14, 18, 22 | 18, 22, 25 |
| T_hz thump | 95 | 140 |
| g_click / g_thump | 0.5 / 1.0 | 1.0 / 0.5 |

Class modifiers (applied to mode freqs `×f`, thump `×f`, decay `×d`, level dB):
Default ×1.0/×1.0/0; Modifier ×0.95/×1.05/−1; Backspace ×0.88/×1.15/+0.5; Enter ×0.85/×1.2/+1;
Space ×0.7/×1.4/+1.5 **plus** stabilizer rattle (bandpassed noise burst 3 kHz Q2, 8 ms after
onset, −14 dB). **Up** samples: click gain ×0.6, no thump, modes ×1.15, length 60 ms, level −8 dB.

Variations: 5 per class per direction; each jitters mode freqs ±5 %, Q ±15 %, gains ±1.5 dB
using `fastrand::Rng::with_seed(0xA11CE ^ class ^ dir ^ var)`.

API:
```rust
pub enum SynthPreset { Thock, Click }
impl SynthPreset { pub fn id(self) -> &'static str; pub fn info(self) -> PackInfo; } // license "CC0-1.0", author "Clatterbox contributors"
pub fn generate(preset: SynthPreset, rate: u32) -> LoadedPack;
```

---

## 7. Keyboard position map (`crates/core/src/keymap/layout.rs`)

Coordinates in key units (u) of the main block on an ANSI board; main block spans
`0.0..15.0u`. `x = (left + width/2) / 15.0`, clamped to `0.0..=1.0`. Navigation cluster,
arrows, and numpad → `x = 1.0`. Function row uses its physical columns (Esc 0.5u → 0.033,
F12 at 14.5u → 0.967).

Row definitions (left edge, width) — implement as `const` tables:

| Row | Keys (width u) |
|---|---|
| Number | Grave 1, Digit1–Digit0 1 each, Minus 1, Equal 1, Backspace 2 |
| Top | Tab 1.5, Q–P 1 each, BracketL 1, BracketR 1, Backslash 1.5 |
| Home | CapsLock 1.75, A–L, Semicolon, Quote 1 each, Enter 2.25 |
| Bottom | ShiftL 2.25, Z–M, Comma, Period, Slash 1 each, ShiftR 2.75 |
| Space | CtrlL 1.25, MetaL 1.25, AltL 1.25, Space 6.25, AltR 1.25, MetaR 1.25, Menu 1.25, CtrlR 1.25 |

ISO: `IntlBackslash` at 1.25u on the bottom row (x = 1.875/15), `IntlRo` = Slash position+1.
Class mapping: `Space → Space`; `Enter, NumpadEnter → Enter`; `Backspace, Delete → Backspace`;
`ShiftL/R, CtrlL/R, AltL/R, MetaL/R, CapsLock, Tab, Fn, Menu → Modifier`; everything else →
`Default`.

```rust
pub fn lookup(key: PhysKey) -> (KeyClass, f32);
```

---

## 8. App shell

### 8.1 Settings — `crates/core/src/settings.rs` (JSON on disk and over IPC)

File: `<app_config_dir>/settings.json`. snake_case keys.

```json
{
  "version": 1,
  "enabled": true,
  "volume": 0.6,
  "pack": "builtin/classic",
  "key_up_enabled": true,
  "pitch_variation": 0.35,
  "spatial_enabled": true,
  "spatial_width": 0.6,
  "launch_at_login": false
}
```

| Field | Type | Range / default |
|---|---|---|
| `version` | u32 | `1` (future migrations switch on it) |
| `enabled` | bool | `true` |
| `volume` | f32 | `0.0..=1.0`, `0.6` |
| `pack` | string | pack id, `"builtin/classic"` (runtime fallback `"synth/thock"`) |
| `key_up_enabled` | bool | `true` |
| `pitch_variation` | f32 | `0.0..=1.0`, `0.35` |
| `spatial_enabled` | bool | `true` |
| `spatial_width` | f32 | `0.0..=1.0`, `0.6` |
| `launch_at_login` | bool | `false` |

```rust
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]                          // missing fields → defaults; unknown fields ignored
pub struct Settings { /* fields above */ }
impl Default for Settings { /* table above */ }
impl Settings {
    pub fn sanitize(&mut self);            // clamp ranges, NaN → default, version → 1
    pub fn merged(&self, patch: &SettingsPatch) -> Settings; // returns sanitized
}
#[derive(Clone, Debug, Default, Deserialize)]
pub struct SettingsPatch { pub enabled: Option<bool>, pub volume: Option<f32>, pub pack: Option<String>,
    pub key_up_enabled: Option<bool>, pub pitch_variation: Option<f32>, pub spatial_enabled: Option<bool>,
    pub spatial_width: Option<f32>, pub launch_at_login: Option<bool> }

pub fn load(path: &Path) -> (Settings, LoadOutcome);  // Missing → default + FirstRun; parse error → rename to
                                                     // settings.json.corrupt-<unix_ts>, default + Recovered
pub fn save_atomic(path: &Path, s: &Settings) -> io::Result<()>; // write settings.json.tmp, fsync, rename
pub enum LoadOutcome { Loaded, FirstRun, Recovered }
```

`src-tauri/src/persist.rs`: `Persister::new(path) -> Persister`, `persister.schedule(settings)`
— background thread, debounce 500 ms, last write wins; `flush()` on quit.

### 8.2 AppState (`src-tauri/src/state.rs`)

```rust
pub struct AppState {
    pub settings: Mutex<Settings>,
    pub params: Arc<EngineParams>,
    pub engine: AudioEngine,
    pub hook: Mutex<Option<HookHandle>>,
    pub registry: Mutex<PackRegistry>,
    pub loaded: Mutex<PackCache>,          // LRU(3) of Arc<LoadedPack>
    pub persister: Persister,
}
```
Startup order in `lib.rs::run()`: single-instance plugin (must be first) → autostart →
dialog → opener → `setup`: load settings → `EngineParams` → `AudioEngine::start` → registry scan
→ load selected pack (on failure: fall back to `synth/thock`, persist, emit status) →
`set_pack(Main)` → `keyhook::start` → build tray → if `FirstRun` or hook `NeedsPermission`, open
settings window. macOS: `app.set_activation_policy(ActivationPolicy::Accessory)` (no Dock icon).

### 8.3 Tauri commands (`src-tauri/src/commands.rs`)

All return `Result<T, CmdError>`; `CmdError` serializes as `{ "code": string, "message": string }`
with codes `invalid_input | not_found | io | pack_invalid | exists | platform`.

| Command (Rust name) | Args (JS keys) | Returns | Effect |
|---|---|---|---|
| `get_settings` | — | `Settings` | |
| `update_settings` | `{ patch: SettingsPatch }` | `Settings` | merge+sanitize → `params.apply` → if pack changed load+`set_pack(Main)` (error ⇒ `pack_invalid`, settings unchanged) → if `launch_at_login` changed call autolaunch enable/disable → persist (debounced) → rebuild tray check states → emit `settings-changed` |
| `list_packs` | — | `PackInfo[]` | |
| `reload_packs` | — | `PackInfo[]` | `rescan`, emit `packs-changed` |
| `preview_pack` | `{ id: string }` | `null` | load into LRU, `set_pack(Preview)`, spawn preview thread playing the phrase below |
| `import_pack` | `{ srcDir: string }` | `PackInfo` | `registry.import_dir`, emit `packs-changed` |
| `open_packs_dir` | — | `null` | create if missing, `opener.open_path` |
| `get_status` | — | `Status` | |
| `request_input_permission` | — | `HookStatus` | macOS §3.3 step 2; other OSes no-op returning status |
| `restart_app` | — | never | `app.restart()` |

Preview phrase (Trigger::preview, Down then Up 55 ms later; 110 ms between keys):
Default x 0.25, Default x 0.45, Default x 0.60, Space x 0.47, Default x 0.70, Backspace x 0.93,
Enter x 0.90. Starting a new preview cancels the old (shared `AtomicU64` generation counter).

```rust
#[derive(Serialize, Clone)] pub struct Status { pub hook: HookStatus, pub audio: AudioStatus,
    pub platform: &'static str /* "windows"|"macos"|"linux" */, pub version: &'static str }
```

### 8.4 Events (Rust → UI), constants in `src-tauri/src/events.rs`

| Name | Payload |
|---|---|
| `settings-changed` | `Settings` (emitted for every change, including from tray) |
| `packs-changed` | `PackInfo[]` |
| `status-changed` | `Status` |

No other events. **Never** an event per keystroke.

### 8.5 Tray (`src-tauri/src/tray.rs`)

`TrayIconBuilder::with_id("main")`, `show_menu_on_left_click(true)`, tooltip
`"Clatterbox — on" | "Clatterbox — off"`, icon swaps between `icons/tray.png` and
`icons/tray-off.png` (macOS: `icon_as_template(true)`).

```
[✓] Sound enabled                    id: toggle_enabled
    Switch ▸  (●) Synth Thock         id: pack:<pack id>   (CheckMenuItems, manual radio)
              ( ) Synth Click
              ( ) …builtin / user (valid only)
[✓] Key-up sounds                    id: toggle_keyup
─────────
    Grant keyboard access…           id: permission   (only while hook NeedsPermission/NeedsRestart)
    Settings…                        id: settings
─────────
    Quit Clatterbox                     id: quit        (persister.flush(), engine.shutdown(), exit)
```
Tray actions go through the same internal `apply_patch(app, patch)` function used by
`update_settings` (single code path). The menu is rebuilt on `settings-changed`/`packs-changed`
(rebuilding is cheap and avoids stale handles).

### 8.6 Settings window (`src-tauri/src/window.rs`)

Label `settings`, 440×620 logical, not resizable, centered, title "Clatterbox Settings", created on
demand with `WebviewWindowBuilder` and **destroyed** on close (no resident webview). If it
exists, `show()+set_focus()` instead. Single-instance callback and tray "Settings…" call
`window::open(app)`.

### 8.7 Config / capabilities

`src-tauri/tauri.conf.json` (key parts):
```json
{
  "productName": "Clatterbox",
  "identifier": "dev.zordhalo.clatterbox",
  "version": "0.1.0",
  "build": { "frontendDist": "../dist", "devUrl": "http://localhost:5173",
             "beforeDevCommand": "npm run dev", "beforeBuildCommand": "npm run build" },
  "app": { "windows": [],
           "security": { "csp": "default-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src ipc: http://ipc.localhost" } },
  "bundle": { "active": true, "targets": "all", "icon": ["icons/32x32.png","icons/128x128.png","icons/128x128@2x.png","icons/icon.icns","icons/icon.ico"],
              "resources": { "resources/packs/": "packs/" },
              "macOS": { "minimumSystemVersion": "14.2" },
              "licenseFile": "../LICENSE" }
}
```
macOS `Info.plist` (in `src-tauri/`): `LSUIElement = true`.
`src-tauri/capabilities/default.json`: windows `["settings"]`, permissions
`["core:default", "dialog:allow-open"]`. Everything else is a custom command (allowed by
default for app commands).

---

## 9. Frontend (`ui/`)

Layout, top to bottom (all controls apply live via `update_settings`; sliders send on `input`
throttled to 30 Hz — persistence debounce is on the Rust side):

1. **Status banner** (hidden when all Running): permission needed (button "Allow keyboard
   access" → `request_input_permission`), needs restart ("Relaunch" → `restart_app`), Linux
   input-group hint (informational text only, no command button — the app never escalates), audio `NoDevice`/`Failed`.
2. **Header**: app name + master switch (`enabled`).
3. **Volume** slider 0–100 %.
4. **Switch** list: one row per pack — radio (select), name, author · license, "▶ Preview"
   button (`preview_pack`); invalid packs shown disabled with error tooltip. Footer buttons:
   "Import pack…" (dialog `open({ directory: true })` → `import_pack`), "Open packs folder",
   "Reload".
5. **Sound**: Key-up sounds (toggle), Pitch variation slider (0–100 %).
6. **Spatial audio**: toggle + width slider (disabled when toggle off).
7. **General**: Launch at login (toggle).
8. **Footer**: version and the line "No keystrokes are recorded, stored, or sent anywhere."
   Pack `source_url`s are shown as plain selectable text, not links (the webview never
   navigates off-app).

Styling: system font stack, `prefers-color-scheme` light/dark via CSS custom properties, native
look; no external fonts/CDNs (CSP). Keyboard accessible (real `<input type=range|checkbox|radio>`,
labels, focus rings).

`ui/src/api.ts` mirrors every Rust type in §4.6, §5.5, §8.1, §8.3 as TS types and exports
typed wrappers: `getSettings()`, `updateSettings(patch)`, `listPacks()`, `reloadPacks()`,
`previewPack(id)`, `importPack(srcDir)`, `openPacksDir()`, `getStatus()`,
`requestInputPermission()`, `restartApp()`, `onSettingsChanged(cb)`, `onPacksChanged(cb)`,
`onStatusChanged(cb)`.

---

## 10. File tree & ownership

Every file < 500 lines. `WPn` = owning work package; only the owner edits a file after Phase 0.

```
clatterbox/
├─ Cargo.toml                         WP0  workspace: members crates/*, src-tauri; [workspace.dependencies] pins from §1
├─ rust-toolchain.toml                WP0  channel = "stable", components rustfmt, clippy
├─ rustfmt.toml, clippy.toml          WP0
├─ LICENSE (Apache-2.0), NOTICE       WP5
├─ README.md                          WP5  (incl. Privacy, Pack format, Permissions per OS, Build)
├─ package.json                       WP4  scripts: dev, build, typecheck, tauri
├─ tsconfig.json, vite.config.ts      WP4  vite root "ui", outDir "../dist"
├─ scripts/check-privacy.sh           WP5
├─ .github/workflows/ci.yml           WP5
├─ .github/workflows/release.yml      WP5
├─ crates/core/
│  ├─ Cargo.toml                      WP0
│  ├─ src/lib.rs                      WP0  module decls + re-exports only
│  ├─ src/types.rs                    WP0  verbatim §4.6 (frozen; changes need lead approval)
│  ├─ src/params.rs                   WP2
│  ├─ src/settings.rs                 WP1
│  ├─ src/repeat.rs                   WP3
│  ├─ src/keymap/mod.rs               WP3  PhysKey enum
│  ├─ src/keymap/layout.rs            WP3  lookup()
│  ├─ src/mixer/mod.rs                WP2  Mixer
│  ├─ src/mixer/voice.rs              WP2
│  ├─ src/mixer/math.rs               WP2  pan, pitch, hermite, soft_clip
│  ├─ src/pack/mod.rs                 WP2  PackInfo, LoadedPack, PackError
│  ├─ src/pack/manifest.rs            WP2
│  ├─ src/pack/decode.rs              WP2
│  ├─ src/pack/derive.rs              WP2  §5.2.1
│  ├─ src/pack/registry.rs            WP2
│  ├─ src/synth/mod.rs                WP2
│  ├─ src/synth/dsp.rs                WP2
│  ├─ src/synth/presets.rs            WP2
│  └─ tests/fixtures/packs/{valid-min,valid-full,bad-traversal,bad-schema,bad-missing-file,bad-too-long}/  WP2
├─ crates/audio/
│  ├─ Cargo.toml                      WP0
│  ├─ src/lib.rs                      WP2  AudioEngine, TriggerTx
│  ├─ src/stream.rs                   WP2  config negotiation + callback + format conversion
│  └─ src/owner.rs                    WP2  owner thread, rebuild, device watch, garbage
├─ crates/keyhook/
│  ├─ Cargo.toml                      WP0  target-specific deps
│  ├─ src/lib.rs                      WP3  start(), HookHandle, backend select
│  ├─ src/dispatch.rs                 WP3
│  ├─ src/windows/mod.rs              WP3
│  ├─ src/windows/scancode.rs         WP3
│  ├─ src/macos/mod.rs                WP3
│  ├─ src/macos/permission.rs         WP3
│  ├─ src/macos/keycode.rs            WP3
│  ├─ src/linux/mod.rs                WP3
│  ├─ src/linux/x11.rs                WP3
│  ├─ src/linux/evdev.rs              WP3
│  └─ src/linux/keycode.rs            WP3
├─ src-tauri/
│  ├─ Cargo.toml, build.rs            WP0 (deps) / WP1 (after)
│  ├─ tauri.conf.json, Info.plist     WP1
│  ├─ capabilities/default.json       WP1
│  ├─ icons/*                         WP5  (placeholder generated with `tauri icon` by WP1 in Phase 0)
│  ├─ resources/packs/{classic,tactile}/  WP5  pack.toml + CREDITS.md + CC0 files (§5.4)
│  └─ src/
│     ├─ main.rs                      WP1  windows_subsystem = "windows"; calls clatterbox_lib::run()
│     ├─ lib.rs                       WP1  builder, plugins, setup
│     ├─ state.rs                     WP1
│     ├─ commands.rs                  WP1
│     ├─ events.rs                    WP1
│     ├─ tray.rs                      WP1
│     ├─ window.rs                    WP1
│     ├─ persist.rs                   WP1
│     ├─ packs.rs                     WP1  PackCache LRU + load-with-fallback + preview scheduler
│     └─ error.rs                     WP1  CmdError
└─ ui/
   ├─ index.html                      WP4
   └─ src/{main.ts, api.ts, packs.ts, controls.ts, status.ts, styles.css}  WP4
```

Keyhook public API (WP3 → WP1 contract):
```rust
// crates/keyhook/src/lib.rs
pub struct HookHandle { /* thread + stop mechanism + Arc<Mutex<HookStatus>> */ }
impl HookHandle { pub fn status(&self) -> HookStatus; pub fn stop(self); }
/// Starts the platform backend on its own thread. Never panics; failures surface as status.
pub fn start(tx: TriggerTx, params: Arc<EngineParams>,
             on_status: Box<dyn Fn(HookStatus) + Send + Sync>) -> HookHandle;
/// macOS: CGRequestListenEventAccess + returns whether granted now. Others: true.
pub fn request_permission() -> bool;
pub const MACOS_PRIVACY_URL: &str =
    "x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent";
pub const LINUX_INPUT_GROUP_HINT: &str = "…§3.4…";
```
`TriggerTx` lives in `crates/core/src/types.rs` (core depends on `rtrb`, pure Rust, no I/O), so
keyhook never depends on audio.

Crate dependency graph: `core` ← `audio`, `core` ← `keyhook`, `{core,audio,keyhook}` ← `src-tauri`.
`core` has no platform/system dependencies → `cargo test -p clatterbox-core` runs anywhere.

---

## 11. Testing strategy

All tests headless; none open audio devices or install hooks.

| Area | Crate / file | Tests |
|---|---|---|
| Settings | core `settings.rs` | defaults round-trip; missing fields → defaults; unknown fields ignored; clamp/NaN sanitize; `merged` applies only `Some`; `load` on corrupt file renames + returns `Recovered`; atomic save leaves no `.tmp`. |
| Pack manifest | core `manifest.rs` | valid-min & valid-full parse; missing `default.down` → error; unknown class/key → error; `..`/absolute/symlink-escape rejected; bad `source_url` scheme rejected; `gain_db` clamped. |
| Decode | core `decode.rs` | fixture WAV 16-bit stereo → mono f32 length correct; too-long file rejected; leading-silence trim. Fixtures are generated tiny WAVs (≤ 20 KB each) committed under `tests/fixtures`. |
| Derivation | core `derive.rs` | pack with only `default.down` → all 10 sets non-empty; derived up length ≈ 0.6 × len × 2^(300/1200) (±2 samples); derived up peak ≈ source peak −9 dB (±0.5 dB); class offsets give the expected length ratios; explicit sets never replaced; `PackInfo.derived` lists exactly the derived sets. |
| MP3 trim | core `decode.rs` | tiny MP3 fixture: first sample above −30 dB-rel lies within 1.5 ms of buffer start after decode. |
| Registry | core `registry.rs` | scan ordering; invalid dir name skipped; `import_dir` copies + refuses duplicate; builtin license allowlist over `src-tauri/resources/packs` (integration test, skipped if dir empty). |
| Synth | core `synth` | every class×dir×var buffer non-empty, RMS > −40 dBFS, peak ≤ −2.9 dBFS, no NaN/Inf; deterministic (same hash twice); generation < 50 ms in debug. |
| Mixer math | core `math.rs` | `l²+r² ≈ 1` over x∈[0,1]; x=0,width=1 → r≈0; spatial off → l=r; pitch 0 → step = rate ratio; +1200 cents → step ×2; soft_clip bounded and monotonic. |
| Mixer | core `mixer` | trigger → render non-silent; left-panned trigger → L energy > 4× R; >32 triggers never panics and `active_voices ≤ 32`; disabled params → Main silent, Preview audible; pack swap fades out without clicks (max sample delta bound); render into mono/6-ch buffers. |
| Repeat filter | core `repeat.rs` | down,down(100ms),down(200ms),up → accept,reject,reject,accept; stale >1200 ms re-accept; orphan up rejected; independence across keys. |
| Keymap | core `layout.rs` + keyhook tables | every PhysKey except Unknown has x∈[0,1]; A < S < … < L strictly increasing x; Space ≈ 0.5±0.05; class mapping table; per-OS table tests (`#[cfg(target_os)]`) for known codes (e.g. Win scan 0x1E → A, mac kVK 0x00 → A, evdev 30 → A), no duplicate mappings. |
| Audio | audio `stream.rs` | pure fn `choose_config(supported: &[SupportedStreamConfigRange]) -> Choice` unit-tested with synthetic ranges; I16/I32 conversion helpers. |
| Frontend | root | `npm run typecheck` (`tsc --noEmit`), `npm run build`. |
| Privacy | `scripts/check-privacy.sh` | fails if `rg -n '(log|tracing)::|println!|eprintln!|dbg!|#\[derive\([^)]*Debug' crates/keyhook/src crates/core/src/keymap crates/core/src/repeat.rs` matches (Debug on `PhysKey` is also banned, so tests use `assert!(a == b)` instead of `assert_eq!`). |

Manual QA checklist (README): latency feel test, sleep/wake, headphone hot-swap, secure input,
per-OS permission first-run, uninstall leaves no autostart entry.

---

## 12. CI / release

### `ci.yml` (push to main, PRs)

```yaml
jobs:
  rust:
    strategy: { fail-fast: false, matrix: { os: [windows-latest, macos-latest, ubuntu-22.04] } }
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v7
      - if: runner.os == 'Linux'
        run: sudo apt-get update && sudo apt-get install -y libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libasound2-dev patchelf
      - uses: dtolnay/rust-toolchain@stable
        with: { components: "rustfmt, clippy" }
      - uses: Swatinem/rust-cache@v2
      - run: mkdir -p dist && echo '<!doctype html>' > dist/index.html   # tauri::generate_context! needs frontendDist
        shell: bash
      - if: runner.os == 'Linux'
        run: cargo fmt --all -- --check
      - run: cargo clippy --workspace --all-targets -- -D warnings
      - run: cargo test --workspace
      - if: runner.os == 'Linux'
        run: bash scripts/check-privacy.sh
  frontend:
    runs-on: ubuntu-22.04
    steps:
      - uses: actions/checkout@v7
      - uses: actions/setup-node@v7
        with: { node-version: lts/*, cache: npm }
      - run: npm ci && npm run typecheck && npm run build
```

### `release.yml` (tags `v*`)

```yaml
permissions: { contents: write }
jobs:
  build:
    strategy:
      fail-fast: false
      matrix:
        include:
          - { platform: macos-latest,   args: "--target universal-apple-darwin" }
          - { platform: ubuntu-22.04,   args: "" }
          - { platform: windows-latest, args: "" }
    runs-on: ${{ matrix.platform }}
    steps:
      - uses: actions/checkout@v7
      - if: matrix.platform == 'ubuntu-22.04'
        run: sudo apt-get update && sudo apt-get install -y libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libasound2-dev patchelf
      - uses: actions/setup-node@v7
        with: { node-version: lts/*, cache: npm }
      - uses: dtolnay/rust-toolchain@stable
        with: { targets: ${{ matrix.platform == 'macos-latest' && 'aarch64-apple-darwin,x86_64-apple-darwin' || '' }} }
      - uses: Swatinem/rust-cache@v2
        with: { workspaces: ". -> target" }
      - run: npm ci
      - uses: tauri-apps/tauri-action@v1
        env: { GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }} }
        with:
          tagName: v__VERSION__
          releaseName: Clatterbox v__VERSION__
          releaseBody: "See CHANGELOG. Unsigned builds — see README for first-run instructions."
          releaseDraft: true
          prerelease: false
          args: ${{ matrix.args }}
```
Signing/notarization env (`APPLE_CERTIFICATE`, `APPLE_ID`, …, Windows Trusted Signing) is added
later behind secrets; the workflow must succeed without them.

---

## 13. Work packages

### Phase 0 — contract skeleton (WP0, **one agent, blocking, ~1 h**; lead or WP1 coder)

Create: workspace `Cargo.toml` with `[workspace.dependencies]`; the three crate `Cargo.toml`s and
`src-tauri/Cargo.toml` with all deps from §1 (target-specific for keyhook);
`crates/core/src/types.rs` verbatim (§4.6 + `TriggerTx` per §10 note); every module file in §10
as a compiling stub (`todo!()` bodies, exact public signatures from this spec); `npm create`
equivalent `package.json`/`vite.config.ts`/`tsconfig.json`/`ui/index.html` minimal;
`tauri icon` placeholder icons. Gate: `cargo check --workspace` and `npm run build` pass on
Windows. Commit, then fan out.

### Phase 1 — parallel (4 agents)

| WP | Owner | Files (exclusive) | Depends on | Done when |
|---|---|---|---|---|
| **WP1 App shell** | coder-app | `src-tauri/**` (except icons/resources), `crates/core/src/settings.rs` | types.rs, stub APIs of WP2/WP3 | App launches to tray on Windows; settings persist; tray toggles work; commands + events per §8; autostart + single-instance; builds against stubs, then real impls |
| **WP2 Audio + packs + synth** | coder-audio | `crates/core/src/{params.rs,mixer/**,pack/**,synth/**}`, `crates/core/tests/**`, `crates/audio/**` | types.rs | §11 tests for these areas green; `examples/beep.rs` in `crates/audio` plays the synth pack through default device (manual) |
| **WP3 Key hook + keymap** | coder-hook | `crates/keyhook/**`, `crates/core/src/{repeat.rs,keymap/**}` | types.rs, params.rs signature | Windows backend works locally; macOS/Linux backends compile in CI (`cargo check` via clippy job); `examples/probe.rs` prints only `class` + `x` rounded to 0.1 (privacy) |
| **WP4 Frontend** | coder-ui | `ui/**`, `package.json`, `tsconfig.json`, `vite.config.ts` | §8.3/§8.4 contract | Typecheck + build green; works against a mocked `invoke` when `?mock` query param present (dev only, tree-shaken in prod) for screenshot iteration |
| **WP5 CI / README / packs** | coder-ops (+ researcher) | `.github/**`, `README.md`, `LICENSE`, `NOTICE`, `scripts/**`, `src-tauri/icons/**`, `src-tauri/resources/packs/**` | none | CI green on 3 OSes; release dry-run on a `v0.0.1-rc` tag produces 3 draft artifacts; packs pass registry validation test |

Interface rule: if a WP needs a signature change in another WP's file or `types.rs`, it sends
the change request to the lead; no cross-edits.

### Phase 2 — integration (WP1 owner + lead)

Wire real engine/hook into `setup`, end-to-end manual test on Windows, CI artifacts smoke-tested
on macOS/Linux by the user, fix-ups, tag `v0.1.0` draft.

---

## 14. Risks & mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| Windows AV/SmartScreen flags an unsigned app that reads global keyboard input | Install friction, quarantine | Raw Input instead of `SetWindowsHookEx`; no network code at all; no packers/UPX; open source with reproducible CI builds; submit to Microsoft Defender false-positive portal per release; code-sign via Azure Trusted Signing once name is final (§15). |
| macOS Input Monitoring permission UX; unsigned builds re-prompt after update; tap needs relaunch after grant | First-run confusion | Explicit banner flow §3.3 with preflight polling + "Relaunch"; README screenshots; sign + notarize for releases (Developer ID). |
| macOS tap disabled by timeout | Silent stop | Re-enable in callback on `kCGEventTapDisabledByTimeout`; callback is O(1). |
| Linux Wayland: no global key API for normal clients | No sound on Wayland | evdev fallback with `input`-group instructions and an honest security caveat; X11/XWayland path when available; document GNOME needs the AppIndicator extension for the tray. Future: XDG GlobalShortcuts portal cannot deliver raw keystrokes — not an option. |
| cpal 0.18 CoreAudio lists macOS 14.2 as minimum OS | Older Macs excluded | Bundle `minimumSystemVersion` 14.2 (covers the large majority in 2026). If demand appears, pin `cpal = "=0.17.3"` (API deltas: `play()`, per-direction CallbackInfo) — isolated to `crates/audio/src/stream.rs`. |
| WASAPI shared-mode latency ~20 ms | Feels slightly late on Windows | Fixed 256-frame request; measure; follow-up option: WASAPI exclusive/IAudioClient3 via custom backend (post-MVP). |
| Missed key-ups (secure desktop, focus loss) | Key stuck "held" → silent | RepeatFilter staleness 1200 ms rule. |
| Default device changes (headphones) | Audio stops | Error callback rebuild + 3 s default-device watch. |
| Hostile user packs (zip-slip, huge files, decoder bugs) | Crash / disk abuse | Folder import only (no archives in MVP); path canonicalization; size/duration caps; decode in `catch_unwind` per file. |
| Webview memory for a tray app | Bloat | Window created on demand, destroyed on close. |
| Trademark | Legal | Neutral names only (§0.3); the unicaegames set ships as "Classic Office", never by its keyboard model; name checked clean (`research/NAMING.md`). |
| Few or no real key-up samples in CC0 sources | Up-strokes sound synthetic | Derivation rules §5.2.1; swap in real recordings later (data-only change to `pack.toml`). |

---

## 15. Open questions

All v0.1 questions are resolved in §16. Post-MVP (not in scope): global toggle hotkey, per-app
mute list, output device picker, zip pack import, tray volume submenu, slicing the unicaegames
continuous-typing takes for more variations, real key-up recordings, code signing (Apple
Developer ID, Azure Trusted Signing).

---

## 16. Lead decisions (resolves §15, 2026-09-18)

1. **Name:** Clatterbox. Crate/binary prefix `clatterbox`, bundle id `dev.zordhalo.clatterbox`,
   repo `github.com/zordhalo/clatterbox`, license Apache-2.0.
2. **Signing:** v0.1 ships unsigned draft releases. README documents first-run bypass
   (SmartScreen "More info → Run anyway", macOS right-click → Open + Input Monitoring grant).
   Signing secrets are wired later; workflows must pass without them.
3. **Packs & default:** three packs at v0.1.
   - `builtin/classic`: OpenGameArt unicaegames CC0 set. Display name "Classic Office".
     Never "Cherry" or "KC1000" (§0.3).
   - `builtin/tactile`: Freesound StavSounds CC0 set, plus the Foxfire CC0 key-release as its
     `up` sample and the alpinemesh CC0 Enter sample.
   - `synth/thock`, the procedural pack.
   **Default pack = `builtin/classic`.** If it fails to load, fall back to `synth/thock`.
   Packs without `up`/`space`/`enter`/`backspace`/`modifier` samples derive them at load time:
   - Missing `up`: take the `down` variations, pitch them −300 cents, apply −9 dB, and truncate
     to 60% of the length with a 5 ms fade.
   - Missing special classes: use `default` with a class-specific pitch/gain offset
     (space −200c +1 dB, enter −150c +1 dB, backspace −50c, modifier +100c −2 dB).
4. **Decoding:** most researched samples are MP3, so enable symphonia's `mp3` feature
   (`["wav","pcm","ogg","vorbis","flac","mp3"]`). Builtin packs ship as the original CC0 files.
   Each pack gets a `pack.toml` with `license = "CC0-1.0"` and a per-file source URL list in
   `CREDITS.md` inside the pack dir.
5. **cpal 0.18 / macOS 14.2 minimum:** accepted for v0.1.
6. **Wayland/evdev:** the evdev fallback activates only when the user is already in the `input`
   group. The app never asks to escalate. The README documents the security trade-off.
