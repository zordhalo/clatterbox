// Dev-only fixture backend for `?mock` (SPEC §13 WP4 "Done when"). Never imported in production
// builds — see the dynamic import in api.ts. Simulates just enough of the Rust side to iterate on
// the UI with `npm run dev` and no Tauri process attached.
import type {
  AudioStatus,
  HookStatus,
  PackInfo,
  Settings,
  SettingsPatch,
  Status,
} from "./api";

let settings: Settings = {
  version: 1,
  enabled: true,
  volume: 0.6,
  pack: "builtin/classic",
  key_up_enabled: true,
  pitch_variation: 0.35,
  spatial_enabled: true,
  spatial_width: 0.6,
  launch_at_login: false,
};

const packs: PackInfo[] = [
  {
    id: "builtin/classic",
    name: "Classic Office",
    author: "unicaegames",
    license: "CC0-1.0",
    source_url: "https://opengameart.org/content/keyboard-soundpack-1-typing-and-single-keystrokes",
    description: "Bright, dry office typing.",
    kind: "builtin",
    valid: true,
    error: null,
    derived: ["space.down", "enter.down", "backspace.down", "modifier.down", "default.up", "space.up", "enter.up", "backspace.up", "modifier.up"],
  },
  {
    id: "builtin/tactile",
    name: "Tactile",
    author: "StavSounds, Foxfire-, alpinemesh, yottasounds",
    license: "CC0-1.0",
    source_url: "https://freesound.org/people/StavSounds/packs/42151/",
    description: "Deeper, tactile bump.",
    kind: "builtin",
    valid: true,
    error: null,
    derived: ["space.down", "modifier.down", "space.up", "modifier.up"],
  },
  {
    id: "synth/thock",
    name: "Synth Thock",
    author: "Clatterbox contributors",
    license: "CC0-1.0",
    source_url: null,
    description: "Procedural, muted linear.",
    kind: "synth",
    valid: true,
    error: null,
    derived: [],
  },
  {
    id: "synth/click",
    name: "Synth Click",
    author: "Clatterbox contributors",
    license: "CC0-1.0",
    source_url: null,
    description: "Procedural, tactile click.",
    kind: "synth",
    valid: true,
    error: null,
    derived: [],
  },
  {
    id: "user/broken-pack",
    name: "broken-pack",
    author: "",
    license: "",
    source_url: null,
    description: null,
    kind: "user",
    valid: false,
    error: "default.down: no sample files found",
    derived: [],
  },
];

let hookStatus: HookStatus = { state: "running", backend: "raw_input" };
let audioStatus: AudioStatus = {
  state: "running",
  device: "Speakers (Realtek High Definition Audio)",
  sample_rate: 48000,
  buffer_frames: 256,
};

type Listener<T> = (payload: T) => void;
const settingsListeners = new Set<Listener<Settings>>();
const packsListeners = new Set<Listener<PackInfo[]>>();
const statusListeners = new Set<Listener<Status>>();

function status(): Status {
  return { hook: hookStatus, audio: audioStatus, platform: "windows", version: "0.1.0" };
}

function sanitize(s: Settings): Settings {
  const clamp01 = (n: number) => Math.min(1, Math.max(0, n));
  return {
    ...s,
    volume: clamp01(s.volume),
    pitch_variation: clamp01(s.pitch_variation),
    spatial_width: clamp01(s.spatial_width),
  };
}

/**
 * Exposes scenario switches on `window` for manual QA of the status banner while running
 * `npm run dev -- --mode mock` style flows (e.g. `__clatterboxMock.setHookStatus({...})`).
 */
function exposeDebugHandle() {
  (window as unknown as { __clatterboxMock: unknown }).__clatterboxMock = {
    setHookStatus(s: HookStatus) {
      hookStatus = s;
      statusListeners.forEach((cb) => cb(status()));
    },
    setAudioStatus(s: AudioStatus) {
      audioStatus = s;
      statusListeners.forEach((cb) => cb(status()));
    },
  };
}
exposeDebugHandle();

export async function mockInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  await new Promise((r) => setTimeout(r, 30));
  switch (cmd) {
    case "get_settings":
      return settings as unknown as T;
    case "update_settings": {
      const patch = (args?.patch ?? {}) as SettingsPatch;
      settings = sanitize({ ...settings, ...patch });
      settingsListeners.forEach((cb) => cb(settings));
      return settings as unknown as T;
    }
    case "list_packs":
      return packs as unknown as T;
    case "reload_packs":
      packsListeners.forEach((cb) => cb(packs));
      return packs as unknown as T;
    case "preview_pack":
      // eslint-disable-next-line no-console
      console.info(`[mock] preview_pack ${String(args?.id)}`);
      return null as unknown as T;
    case "import_pack": {
      const info: PackInfo = {
        id: `user/${String(args?.srcDir).split(/[\\/]/).pop() ?? "imported"}`,
        name: "Imported Pack",
        author: "You",
        license: "Unknown",
        source_url: null,
        description: null,
        kind: "user",
        valid: true,
        error: null,
        derived: ["space.down", "default.up"],
      };
      packs.push(info);
      packsListeners.forEach((cb) => cb(packs));
      return info as unknown as T;
    }
    case "open_packs_dir":
      return null as unknown as T;
    case "get_status":
      return status() as unknown as T;
    case "request_input_permission":
      hookStatus = { state: "needs_restart" };
      statusListeners.forEach((cb) => cb(status()));
      return hookStatus as unknown as T;
    case "restart_app":
      hookStatus = { state: "running", backend: "raw_input" };
      statusListeners.forEach((cb) => cb(status()));
      return undefined as unknown as T;
    default:
      throw new Error(`[mock] unhandled command: ${cmd}`);
  }
}

export async function mockListen<T>(
  event: string,
  cb: (payload: T) => void,
): Promise<() => void> {
  if (event === "settings-changed") {
    const l = cb as Listener<Settings>;
    settingsListeners.add(l);
    return () => settingsListeners.delete(l);
  }
  if (event === "packs-changed") {
    const l = cb as Listener<PackInfo[]>;
    packsListeners.add(l);
    return () => packsListeners.delete(l);
  }
  if (event === "status-changed") {
    const l = cb as Listener<Status>;
    statusListeners.add(l);
    return () => statusListeners.delete(l);
  }
  return () => {};
}

export async function mockPickFolder(): Promise<string | null> {
  return "C:\\Users\\demo\\Downloads\\my-pack";
}
