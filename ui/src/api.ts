// Typed IPC contract mirroring the Rust types (SPEC §4.6, §5.4, §8.1, §8.3, §8.4).
// Phase 0 contract; WP4 owns this file afterwards.
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

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

export const getSettings = () => invoke<Settings>("get_settings");
export const updateSettings = (patch: SettingsPatch) =>
  invoke<Settings>("update_settings", { patch });
export const listPacks = () => invoke<PackInfo[]>("list_packs");
export const reloadPacks = () => invoke<PackInfo[]>("reload_packs");
export const previewPack = (id: string) => invoke<null>("preview_pack", { id });
export const importPack = (srcDir: string) => invoke<PackInfo>("import_pack", { srcDir });
export const openPacksDir = () => invoke<null>("open_packs_dir");
export const getStatus = () => invoke<Status>("get_status");
export const requestInputPermission = () => invoke<HookStatus>("request_input_permission");
export const restartApp = () => invoke<never>("restart_app");

export const onSettingsChanged = (cb: (s: Settings) => void): Promise<UnlistenFn> =>
  listen<Settings>(EVENTS.settingsChanged, (e) => cb(e.payload));
export const onPacksChanged = (cb: (p: PackInfo[]) => void): Promise<UnlistenFn> =>
  listen<PackInfo[]>(EVENTS.packsChanged, (e) => cb(e.payload));
export const onStatusChanged = (cb: (s: Status) => void): Promise<UnlistenFn> =>
  listen<Status>(EVENTS.statusChanged, (e) => cb(e.payload));
