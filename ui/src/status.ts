// Permission / audio status banner (SPEC §9.1). One banner at a time, most actionable first.
import { requestInputPermission, restartApp, type Status } from "./api";

const bannerEl = document.getElementById("banner") as HTMLDivElement;
const textEl = document.getElementById("banner-text") as HTMLParagraphElement;
const actionEl = document.getElementById("banner-action") as HTMLButtonElement;

const LINUX_INPUT_GROUP_HINT =
  "On Wayland, Clatterbox can only hear keys if your user can read keyboard devices " +
  "(the input group). Clatterbox will not change this for you.";

interface Banner {
  text: string;
  action?: { label: string; run: () => void };
}

function bannerFor(status: Status): Banner | null {
  const { hook, audio, platform } = status;

  if (hook.state === "needs_permission") {
    if (platform === "linux") {
      return { text: hook.hint || LINUX_INPUT_GROUP_HINT };
    }
    return {
      text: "Clatterbox needs Input Monitoring access to hear keystrokes.",
      action: { label: "Allow keyboard access", run: () => void requestInputPermission() },
    };
  }
  if (hook.state === "needs_restart") {
    return {
      text: "Access was granted. Relaunch Clatterbox to start hearing keystrokes.",
      action: { label: "Relaunch", run: () => void restartApp() },
    };
  }
  if (hook.state === "unsupported") {
    return { text: hook.reason };
  }
  if (hook.state === "failed") {
    return { text: `Keyboard capture failed: ${hook.reason}` };
  }

  if (audio.state === "no_device") {
    return { text: "No audio output device found. Sound is paused until one is available." };
  }
  if (audio.state === "failed") {
    return { text: `Audio device error: ${audio.reason}` };
  }

  return null;
}

export function renderStatus(status: Status): void {
  const banner = bannerFor(status);
  if (!banner) {
    bannerEl.hidden = true;
    return;
  }
  bannerEl.hidden = false;
  textEl.textContent = banner.text;
  if (banner.action) {
    actionEl.hidden = false;
    actionEl.textContent = banner.action.label;
    actionEl.onclick = banner.action.run;
  } else {
    actionEl.hidden = true;
    actionEl.onclick = null;
  }
}
