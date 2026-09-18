/*
 * Clatterbox landing page — in-browser demo.
 *
 * Two packs (classic, tactile) play the same CC0 recordings the desktop app
 * ships with. Two packs (thock, click) are pure Web Audio synthesis, exactly
 * like the app's built-in procedural packs — no samples, generated live.
 *
 * Nothing plays until the first keystroke inside #type-surface: the
 * AudioContext is created and resumed lazily, inside that gesture handler.
 */

(() => {
  "use strict";

  const SAMPLE_PACKS = {
    classic: {
      kind: "sample",
      files: {
        default: { down: ["assets/samples/classic/down-1.wav", "assets/samples/classic/down-2.wav", "assets/samples/classic/down-3.wav", "assets/samples/classic/down-4.wav"] },
      },
    },
    tactile: {
      kind: "sample",
      files: {
        default: {
          down: ["assets/samples/tactile/down-1.mp3", "assets/samples/tactile/down-2.mp3", "assets/samples/tactile/down-3.mp3", "assets/samples/tactile/down-4.mp3"],
          up: ["assets/samples/tactile/up-1.mp3"],
        },
      },
    },
  };

  // Pitch/gain offsets applied to derived or class-specific sounds.
  // Mirrors docs/SPEC.md §5.2.1's derivation table.
  const CLASS_OFFSET = {
    default: { cents: 0, db: 0 },
    space: { cents: -200, db: 1 },
    enter: { cents: -150, db: 1 },
    backspace: { cents: -50, db: 0 },
    modifier: { cents: 100, db: -2 },
  };
  const DERIVED_UP = { cents: -300, db: -9, lengthRatio: 0.6, fadeMs: 5 };
  const PER_HIT_VARIATION_CENTS = 45;

  // Rough x-position (0 = far left, 1 = far right) of each key on a standard
  // QWERTY row, used to pan the click to where the key actually sits.
  const ROWS = ["1234567890", "qwertyuiop", "asdfghjkl", "zxcvbnm"];
  const KEY_X = {};
  ROWS.forEach((row, r) => {
    const inset = r * 0.02;
    [...row].forEach((ch, i) => {
      KEY_X[ch] = inset + (i / (row.length - 1)) * (1 - inset * 2);
    });
  });

  function keyInfo(key) {
    if (key === " " || key === "Spacebar") return { cls: "space", x: 0.5 };
    if (key === "Enter") return { cls: "enter", x: 0.97 };
    if (key === "Backspace") return { cls: "backspace", x: 0.97 };
    if (["Shift", "Control", "Alt", "Meta", "Tab", "CapsLock"].includes(key)) {
      return { cls: "modifier", x: 0.06 };
    }
    const lower = key.length === 1 ? key.toLowerCase() : "";
    if (lower in KEY_X) return { cls: "default", x: KEY_X[lower] };
    return { cls: "default", x: 0.5 };
  }

  class ClatterEngine {
    constructor() {
      this.ctx = null;
      this.master = null;
      this.buffers = new Map(); // packId -> { default: {down:[buf], up:[buf]}, ... }
      this.loading = new Map();
    }

    ensureContext() {
      if (this.ctx) return this.ctx;
      const AC = window.AudioContext || window.webkitAudioContext;
      this.ctx = new AC();
      this.master = this.ctx.createGain();
      this.master.gain.value = 0.85;
      this.master.connect(this.ctx.destination);
      return this.ctx;
    }

    resume() {
      const ctx = this.ensureContext();
      if (ctx.state === "suspended") ctx.resume();
      return ctx;
    }

    async loadSamplePack(packId) {
      if (this.buffers.has(packId)) return this.buffers.get(packId);
      if (this.loading.has(packId)) return this.loading.get(packId);

      const ctx = this.ensureContext();
      const def = SAMPLE_PACKS[packId].files;
      const promise = (async () => {
        const out = {};
        for (const [cls, sets] of Object.entries(def)) {
          out[cls] = {};
          for (const [dir, urls] of Object.entries(sets)) {
            out[cls][dir] = await Promise.all(
              urls.map(async (url) => {
                const res = await fetch(url);
                const arr = await res.arrayBuffer();
                return ctx.decodeAudioData(arr);
              })
            );
          }
        }
        this.buffers.set(packId, out);
        return out;
      })();

      this.loading.set(packId, promise);
      return promise;
    }

    randomVariation(cents) {
      return (Math.random() * 2 - 1) * cents;
    }

    centsToRate(cents) {
      return Math.pow(2, cents / 1200);
    }

    dbToGain(db) {
      return Math.pow(10, db / 20);
    }

    playBuffer(buffer, { pan, rateCents, gainDb, truncateRatio, fadeMs }) {
      const ctx = this.resume();
      const src = ctx.createBufferSource();
      src.buffer = buffer;
      src.playbackRate.value = this.centsToRate(rateCents);

      const gain = ctx.createGain();
      gain.gain.value = this.dbToGain(gainDb);

      const panner = ctx.createStereoPanner
        ? ctx.createStereoPanner()
        : null;

      src.connect(gain);
      if (panner) {
        panner.pan.value = Math.max(-1, Math.min(1, pan));
        gain.connect(panner);
        panner.connect(this.master);
      } else {
        gain.connect(this.master);
      }

      const now = ctx.currentTime;
      if (truncateRatio && truncateRatio < 1) {
        const stopAt = now + (buffer.duration / src.playbackRate.value) * truncateRatio;
        gain.gain.setValueAtTime(gain.gain.value, Math.max(now, stopAt - fadeMs / 1000));
        gain.gain.linearRampToValueAtTime(0, stopAt);
        src.start(now);
        src.stop(stopAt + 0.01);
      } else {
        src.start(now);
      }
    }

    async triggerSample(packId, char, dir) {
      const pack = await this.loadSamplePack(packId);
      const info = keyInfo(char);
      const offset = CLASS_OFFSET[info.cls] || CLASS_OFFSET.default;
      const pan = (info.x - 0.5) * 1.7;

      if (dir === "down") {
        const set = pack[info.cls] || pack.default;
        const variations = set.down;
        const buf = variations[(Math.random() * variations.length) | 0];
        this.playBuffer(buf, {
          pan,
          rateCents: offset.cents + this.randomVariation(PER_HIT_VARIATION_CENTS),
          gainDb: offset.db,
        });
        return;
      }

      // key-up: use an explicit up sample if the pack has one, otherwise
      // derive it from the down set (pitched down, quieter, truncated).
      const classSet = pack[info.cls] || pack.default;
      if (classSet.up && classSet.up.length) {
        const buf = classSet.up[(Math.random() * classSet.up.length) | 0];
        this.playBuffer(buf, { pan, rateCents: offset.cents, gainDb: offset.db });
        return;
      }
      const downSet = classSet.down || pack.default.down;
      const buf = downSet[(Math.random() * downSet.length) | 0];
      this.playBuffer(buf, {
        pan,
        rateCents: offset.cents + DERIVED_UP.cents,
        gainDb: offset.db + DERIVED_UP.db,
        truncateRatio: DERIVED_UP.lengthRatio,
        fadeMs: DERIVED_UP.fadeMs,
      });
    }

    // Procedural packs: no samples, generated per hit.
    triggerSynth(preset, char, dir) {
      const ctx = this.resume();
      const info = keyInfo(char);
      const offset = CLASS_OFFSET[info.cls] || CLASS_OFFSET.default;
      const pan = (info.x - 0.5) * 1.7;
      const detune = offset.cents + this.randomVariation(PER_HIT_VARIATION_CENTS);
      const rate = this.centsToRate(detune);
      const gainMul = this.dbToGain(offset.db) * (dir === "up" ? 0.35 : 1);

      const now = ctx.currentTime;
      const dur = preset === "thock" ? 0.09 : 0.045;

      // Noise burst = the transient "clack".
      const noiseLen = Math.ceil(ctx.sampleRate * dur);
      const noiseBuf = ctx.createBuffer(1, noiseLen, ctx.sampleRate);
      const data = noiseBuf.getChannelData(0);
      for (let i = 0; i < noiseLen; i++) {
        data[i] = (Math.random() * 2 - 1) * (1 - i / noiseLen);
      }
      const noise = ctx.createBufferSource();
      noise.buffer = noiseBuf;
      noise.playbackRate.value = rate;

      const filter = ctx.createBiquadFilter();
      if (preset === "thock") {
        filter.type = "lowpass";
        filter.frequency.value = dir === "up" ? 1600 : 900;
        filter.Q.value = 0.7;
      } else {
        filter.type = "bandpass";
        filter.frequency.value = dir === "up" ? 4200 : 2600;
        filter.Q.value = 1.1;
      }

      const body = ctx.createOscillator();
      body.type = preset === "thock" ? "sine" : "triangle";
      body.frequency.value = (preset === "thock" ? 140 : 380) * rate;

      const bodyGain = ctx.createGain();
      bodyGain.gain.setValueAtTime(gainMul * (preset === "thock" ? 0.5 : 0.18), now);
      bodyGain.gain.exponentialRampToValueAtTime(0.001, now + dur * 1.4);

      const noiseGain = ctx.createGain();
      noiseGain.gain.setValueAtTime(gainMul * 0.9, now);
      noiseGain.gain.exponentialRampToValueAtTime(0.001, now + dur);

      const panner = ctx.createStereoPanner ? ctx.createStereoPanner() : null;

      noise.connect(filter);
      filter.connect(noiseGain);
      body.connect(bodyGain);

      const bus = panner || this.master;
      noiseGain.connect(bus);
      bodyGain.connect(bus);
      if (panner) {
        panner.pan.value = Math.max(-1, Math.min(1, pan));
        panner.connect(this.master);
      }

      noise.start(now);
      noise.stop(now + dur + 0.02);
      body.start(now);
      body.stop(now + dur * 1.4);
    }

    trigger(packId, char, dir) {
      if (SAMPLE_PACKS[packId]) {
        this.triggerSample(packId, char, dir);
      } else {
        this.triggerSynth(packId, char, dir);
      }
    }
  }

  // ---------- UI wiring ----------

  const engine = new ClatterEngine();
  let activePack = "classic";
  let soundStarted = false;

  const led = document.getElementById("led");
  const boardFoot = document.getElementById("board-foot");
  const surface = document.getElementById("type-surface");
  const keyStrip = document.getElementById("key-strip");
  const packButtons = [...document.querySelectorAll(".pack-btn")];

  const STRIP_SIZE = 28;
  for (let i = 0; i < STRIP_SIZE; i++) {
    const dot = document.createElement("span");
    dot.className = "dot";
    keyStrip.appendChild(dot);
  }
  const dots = [...keyStrip.children];

  function lightStrip(x) {
    const center = Math.round(x * (STRIP_SIZE - 1));
    dots.forEach((dot, i) => {
      const dist = Math.abs(i - center);
      if (dist <= 1) {
        dot.classList.add("hit");
        setTimeout(() => dot.classList.remove("hit"), 130);
      }
    });
  }

  function markLive() {
    if (soundStarted) return;
    soundStarted = true;
    led.classList.add("is-live");
    boardFoot.textContent = "Sound is on. Switch packs above, or keep typing.";
  }

  packButtons.forEach((btn) => {
    btn.addEventListener("click", () => {
      packButtons.forEach((b) => {
        b.classList.toggle("is-active", b === btn);
        b.setAttribute("aria-checked", String(b === btn));
      });
      activePack = btn.dataset.pack;
      if (SAMPLE_PACKS[activePack]) engine.loadSamplePack(activePack);
    });
  });

  surface.addEventListener("keydown", (e) => {
    if (e.repeat) return;
    markLive();
    const info = keyInfo(e.key);
    lightStrip(info.x);
    engine.trigger(activePack, e.key, "down");
  });

  surface.addEventListener("keyup", (e) => {
    if (!soundStarted) return;
    engine.trigger(activePack, e.key, "up");
  });

  // Preload the default pack's samples as soon as the browser is idle, so
  // the very first keystroke doesn't wait on a network fetch. This loads
  // data only — it never produces sound on its own.
  const idle = window.requestIdleCallback || ((fn) => setTimeout(fn, 300));
  idle(() => {
    if (SAMPLE_PACKS[activePack]) {
      // Decoding needs an AudioContext; defer creation until a real gesture
      // has happened by only prefetching the raw bytes here.
      SAMPLE_PACKS[activePack].files &&
        Object.values(SAMPLE_PACKS[activePack].files).forEach((sets) => {
          Object.values(sets).forEach((urls) => urls.forEach((u) => fetch(u).catch(() => {})));
        });
    }
  });
})();
