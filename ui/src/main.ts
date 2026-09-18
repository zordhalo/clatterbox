// Settings window entry point (SPEC §9). Fetches initial state, wires controls, subscribes to
// the three Rust → UI events so tray-driven changes stay in sync.
import "./styles.css";
import { getSettings, getStatus, listPacks, onPacksChanged, onSettingsChanged, onStatusChanged } from "./api";
import { initControls, renderControls } from "./controls";
import { initPacks, renderPacks } from "./packs";
import { renderStatus } from "./status";

const versionEl = document.getElementById("version-text") as HTMLSpanElement;

async function main() {
  initControls();
  initPacks();

  const [settings, status, packs] = await Promise.all([getSettings(), getStatus(), listPacks()]);

  let currentPackId = settings.pack;
  let cachedPacks = packs;

  renderControls(settings);
  renderStatus(status);
  renderPacks(cachedPacks, currentPackId);
  versionEl.textContent = `v${status.version}`;

  await onSettingsChanged((s) => {
    currentPackId = s.pack;
    renderControls(s);
    renderPacks(cachedPacks, currentPackId);
  });
  await onStatusChanged(renderStatus);
  await onPacksChanged((p) => {
    cachedPacks = p;
    renderPacks(cachedPacks, currentPackId);
  });
}

void main();
