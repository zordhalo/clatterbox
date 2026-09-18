// Pack list, preview, import, open folder, reload (SPEC §9.4).
import {
  importPack,
  listPacks,
  openPacksDir,
  pickPackFolder,
  previewPack,
  reloadPacks,
  updateSettings,
  type PackInfo,
} from "./api";

const listEl = document.getElementById("pack-list") as HTMLUListElement;
const importBtn = document.getElementById("import-pack") as HTMLButtonElement;
const openDirBtn = document.getElementById("open-packs-dir") as HTMLButtonElement;
const reloadBtn = document.getElementById("reload-packs") as HTMLButtonElement;

let currentPackId = "";

function metaLine(p: PackInfo): string {
  if (!p.valid) return p.error ?? "Invalid pack";
  const bits = [p.author, p.license].filter(Boolean);
  return bits.join(" · ");
}

function buildRow(p: PackInfo): HTMLLIElement {
  const li = document.createElement("li");
  li.className = "pack-row";

  const radio = document.createElement("input");
  radio.type = "radio";
  radio.name = "pack";
  radio.className = "pack-radio";
  radio.value = p.id;
  radio.disabled = !p.valid;
  radio.checked = p.id === currentPackId;
  radio.id = `pack-${p.id}`;
  radio.addEventListener("change", () => {
    if (radio.checked) void updateSettings({ pack: p.id });
  });

  const info = document.createElement("div");
  info.className = "pack-info";

  const name = document.createElement("label");
  name.className = "pack-name" + (p.valid ? "" : " invalid");
  name.htmlFor = radio.id;
  name.textContent = p.name;
  info.appendChild(name);

  const meta = document.createElement("span");
  meta.className = p.valid ? "pack-meta" : "pack-error";
  meta.textContent = metaLine(p);
  meta.title = metaLine(p);
  info.appendChild(meta);

  const preview = document.createElement("button");
  preview.type = "button";
  preview.className = "pack-preview";
  preview.textContent = "▶ Preview";
  preview.disabled = !p.valid;
  preview.addEventListener("click", () => void previewPack(p.id));

  li.append(radio, info, preview);
  return li;
}

export function renderPacks(packs: PackInfo[], selectedPackId: string): void {
  currentPackId = selectedPackId;
  listEl.replaceChildren(...packs.map(buildRow));
}

export function initPacks(): void {
  importBtn.addEventListener("click", async () => {
    const dir = await pickPackFolder();
    if (!dir) return;
    try {
      await importPack(dir);
      const packs = await listPacks();
      renderPacks(packs, currentPackId);
    } catch (err) {
      console.error("import_pack failed", err);
    }
  });

  openDirBtn.addEventListener("click", () => void openPacksDir());

  reloadBtn.addEventListener("click", async () => {
    const packs = await reloadPacks();
    renderPacks(packs, currentPackId);
  });
}
