// Switches and sliders bound to update_settings (SPEC §9: header, volume, sound, spatial, general).
import { updateSettings, type Settings, type SettingsPatch } from "./api";

const THROTTLE_MS = 1000 / 30; // sliders send on `input`, throttled to 30 Hz (SPEC §9)

function throttle<A extends unknown[]>(fn: (...args: A) => void, ms: number): (...args: A) => void {
  let last = 0;
  let timer: ReturnType<typeof setTimeout> | undefined;
  let pending: A | undefined;
  return (...args: A) => {
    const now = performance.now();
    const remaining = ms - (now - last);
    pending = args;
    if (remaining <= 0) {
      last = now;
      clearTimeout(timer);
      timer = undefined;
      fn(...args);
    } else if (!timer) {
      timer = setTimeout(() => {
        last = performance.now();
        timer = undefined;
        if (pending) fn(...pending);
      }, remaining);
    }
  };
}

const els = {
  enabled: document.getElementById("enabled-input") as HTMLInputElement,
  volume: document.getElementById("volume-input") as HTMLInputElement,
  volumeValue: document.getElementById("volume-value") as HTMLOutputElement,
  keyUp: document.getElementById("keyup-input") as HTMLInputElement,
  pitch: document.getElementById("pitch-input") as HTMLInputElement,
  pitchValue: document.getElementById("pitch-value") as HTMLOutputElement,
  spatial: document.getElementById("spatial-input") as HTMLInputElement,
  width: document.getElementById("width-input") as HTMLInputElement,
  widthValue: document.getElementById("width-value") as HTMLOutputElement,
  widthGroup: document.getElementById("spatial-width-group") as HTMLDivElement,
  launch: document.getElementById("launch-input") as HTMLInputElement,
};

function pct(fraction: number): string {
  return `${Math.round(fraction * 100)}%`;
}

function setSliderFill(input: HTMLInputElement): void {
  const min = Number(input.min || 0);
  const max = Number(input.max || 100);
  const frac = (Number(input.value) - min) / (max - min || 1);
  input.style.setProperty("--fill", `${frac * 100}%`);
}

function isEditing(el: HTMLElement): boolean {
  return document.activeElement === el;
}

const sendPatch = throttle((patch: SettingsPatch) => {
  void updateSettings(patch);
}, THROTTLE_MS);

function wireSlider(
  input: HTMLInputElement,
  output: HTMLOutputElement,
  key: "volume" | "pitch_variation" | "spatial_width",
) {
  input.addEventListener("input", () => {
    setSliderFill(input);
    const fraction = Number(input.value) / 100;
    output.textContent = pct(fraction);
    sendPatch({ [key]: fraction } as SettingsPatch);
  });
}

function wireSwitch(input: HTMLInputElement, key: keyof SettingsPatch) {
  input.addEventListener("change", () => {
    void updateSettings({ [key]: input.checked } as SettingsPatch);
  });
}

export function initControls(): void {
  wireSlider(els.volume, els.volumeValue, "volume");
  wireSlider(els.pitch, els.pitchValue, "pitch_variation");
  wireSlider(els.width, els.widthValue, "spatial_width");
  wireSwitch(els.enabled, "enabled");
  wireSwitch(els.keyUp, "key_up_enabled");
  wireSwitch(els.launch, "launch_at_login");

  els.spatial.addEventListener("change", () => {
    void updateSettings({ spatial_enabled: els.spatial.checked });
    els.widthGroup.classList.toggle("disabled", !els.spatial.checked);
    els.width.disabled = !els.spatial.checked;
  });
}

/** Reflects a `Settings` snapshot into the controls, skipping any input the user is mid-drag on. */
export function renderControls(s: Settings): void {
  if (!isEditing(els.enabled)) els.enabled.checked = s.enabled;
  if (!isEditing(els.volume)) {
    els.volume.value = String(Math.round(s.volume * 100));
    els.volumeValue.textContent = pct(s.volume);
    setSliderFill(els.volume);
  }
  if (!isEditing(els.keyUp)) els.keyUp.checked = s.key_up_enabled;
  if (!isEditing(els.pitch)) {
    els.pitch.value = String(Math.round(s.pitch_variation * 100));
    els.pitchValue.textContent = pct(s.pitch_variation);
    setSliderFill(els.pitch);
  }
  if (!isEditing(els.spatial)) {
    els.spatial.checked = s.spatial_enabled;
    els.widthGroup.classList.toggle("disabled", !s.spatial_enabled);
    els.width.disabled = !s.spatial_enabled;
  }
  if (!isEditing(els.width)) {
    els.width.value = String(Math.round(s.spatial_width * 100));
    els.widthValue.textContent = pct(s.spatial_width);
    setSliderFill(els.width);
  }
  if (!isEditing(els.launch)) els.launch.checked = s.launch_at_login;
}
