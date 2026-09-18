// Typed IPC contract mirroring the Rust types (SPEC §4.6, §5.4, §8.1, §8.3, §8.4).
// Phase 0 contract; WP4 owns this file afterwards.
import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { listen as tauriListen, type UnlistenFn } from "@tauri-apps/api/event";
import { open as dialogOpen } from "@tauri-apps/plugin-dialog";

// `?mock` dev mode (SPEC §13 WP4): swap the transport for an in-memory fixture so the UI can be
// built and screenshotted without a running Tauri backend. `import.meta.env.DEV` is inlined to
// `false` in production builds, so this whole branch (and the ./mock chunk) is dead-code-eliminated.
const mockImpl =
  import.meta.env.DEV && new URLSearchParams(location.search).has("mock")
    ? await import("./mock")
    : null;

function callInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  return mockImpl ? mockImpl.mockInvoke<T>(cmd, args) : tauriInvoke<T>(cmd, args);
}

function callListen<T>(event: string, cb: (payload: T) => void): Promise<UnlistenFn> {
  return mockImpl
    ? mockImpl.mockListen<T>(event, cb)
    : tauriListen<T>(event, (e) => cb(e.payload));
}

export interface Settings {
  version: number;
  enabled: boolean;
  volume: number;
  pack: string;
  key_up_enabled: boolean;
  pitch_variation: number;
  spatial_enabled: boolean;
  spatial_width: number;
  launch_at_login: boolean;
}

export type SettingsPatch = Partial<Omit<Settings, "version">>;

export type PackKind = "synth" | "builtin" | "user";

export interface PackInfo {
  id: string;
  name: string;
  author: string;
  license: string;
  source_url: string | null;
  description: string | null;
  kind: PackKind;
  valid: boolean;
  error: string | null;
  /** Derived sets (SPEC §5.2.1), e.g. ["space.down", "default.up"]; empty for synth. */
  derived: string[];
}

export type HookStatus =
  | { state: "starting" }
  | { state: "running"; backend: "raw_input" | "event_tap" | "x11_xi2" | "evdev" }
  | { state: "needs_permission"; hint: string }
  | { state: "needs_restart" }
  | { state: "unsupported"; reason: string }
  | { state: "failed"; reason: string };

export type AudioStatus =
  | { state: "starting" }
  | { state: "running"; device: string; sample_rate: number; buffer_frames: number | null }
  | { state: "no_device" }
  | { state: "failed"; reason: string };

export interface Status {
  hook: HookStatus;
  audio: AudioStatus;
  platform: "windows" | "macos" | "linux";
  version: string;
}

export type CmdErrorCode =
  | "invalid_input"
  | "not_found"
  | "io"
  | "pack_invalid"
  | "exists"
  | "platform";

export interface CmdError {
  code: CmdErrorCode;
  message: string;
}

export const EVENTS = {
  settingsChanged: "settings-changed",
  packsChanged: "packs-changed",
  statusChanged: "status-changed",
} as const;

export const getSettings = () => callInvoke<Settings>("get_settings");
export const updateSettings = (patch: SettingsPatch) =>
  callInvoke<Settings>("update_settings", { patch });
export const listPacks = () => callInvoke<PackInfo[]>("list_packs");
export const reloadPacks = () => callInvoke<PackInfo[]>("reload_packs");
export const previewPack = (id: string) => callInvoke<null>("preview_pack", { id });
export const importPack = (srcDir: string) => callInvoke<PackInfo>("import_pack", { srcDir });
export const openPacksDir = () => callInvoke<null>("open_packs_dir");
export const getStatus = () => callInvoke<Status>("get_status");
export const requestInputPermission = () => callInvoke<HookStatus>("request_input_permission");
export const restartApp = () => callInvoke<never>("restart_app");

export const onSettingsChanged = (cb: (s: Settings) => void): Promise<UnlistenFn> =>
  callListen<Settings>(EVENTS.settingsChanged, cb);
export const onPacksChanged = (cb: (p: PackInfo[]) => void): Promise<UnlistenFn> =>
  callListen<PackInfo[]>(EVENTS.packsChanged, cb);
export const onStatusChanged = (cb: (s: Status) => void): Promise<UnlistenFn> =>
  callListen<Status>(EVENTS.statusChanged, cb);

/** Folder picker for pack import (SPEC §9.4). Mocked under `?mock` (no dialog plugin in a bare browser). */
export const pickPackFolder = async (): Promise<string | null> =>
  mockImpl ? mockImpl.mockPickFolder() : ((await dialogOpen({ directory: true })) ?? null);
