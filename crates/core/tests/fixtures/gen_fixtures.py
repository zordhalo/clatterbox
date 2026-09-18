"""Regenerates the tiny WAV fixtures used by crates/core tests. Run from this directory.

Deterministic, stdlib only. Every file stays well under 20 KB.
"""
import math
import os
import struct
import wave


def write(path, rate, channels, frames, width=2):
    """frames: list of per-frame tuples of floats in -1..1."""
    os.makedirs(os.path.dirname(path) or ".", exist_ok=True)
    with wave.open(path, "wb") as w:
        w.setnchannels(channels)
        w.setsampwidth(width)
        w.setframerate(rate)
        data = bytearray()
        for f in frames:
            for x in f:
                if width == 2:
                    data += struct.pack("<h", int(round(max(-1.0, min(1.0, x)) * 32767)))
                else:
                    data += struct.pack("<B", int(round(max(-1.0, min(1.0, x)) * 127 + 128)))
        w.writeframes(bytes(data))


def click(rate, ms=60, freq=900.0, lead_ms=0.0, amp=0.6):
    lead = int(rate * lead_ms / 1000)
    n = int(rate * ms / 1000)
    out = [(0.0,)] * lead
    for i in range(n):
        t = i / rate
        out.append((amp * math.exp(-t / 0.012) * math.sin(2 * math.pi * freq * t),))
    return out


R = 22050
# decode fixtures
write("audio/stereo-dc.wav", R, 2, [(0.5, 0.25)] * 1000)
write("audio/lead-silence.wav", R, 1, click(R, lead_ms=20.0))
write("audio/too-long.wav", 8000, 1, [(0.3 * math.sin(i * 0.2),) for i in range(int(8000 * 2.1))], width=1)

# packs
write("packs/valid-min/a.wav", R, 1, click(R))
full = "packs/valid-full"
for i, f in enumerate([800.0, 950.0]):
    write(f"{full}/default-down-{i + 1}.wav", R, 1, click(R, freq=f))
write(f"{full}/default-up-1.wav", R, 1, click(R, ms=30, freq=1400.0, amp=0.2))
write(f"{full}/sub/space-down-1.wav", R, 1, click(R, ms=80, freq=500.0))
write(f"{full}/enter-down-1.wav", R, 1, click(R, freq=600.0))
write("packs/bad-schema/a.wav", R, 1, click(R))
write("packs/bad-traversal/a.wav", R, 1, click(R))
write("packs/bad-too-long/long.wav", 8000, 1, [(0.3 * math.sin(i * 0.2),) for i in range(int(8000 * 2.1))], width=1)
