"""Write a noisy, speech-like test file for the smoke test: a gliding voiced
tone with syllable-rate amplitude modulation, plus broadband noise and hum."""
import math, random, struct, sys, wave

path = sys.argv[1] if len(sys.argv) > 1 else "noisy.wav"
sr, seconds = 44_100, 6
random.seed(1)
frames = bytearray()
for i in range(sr * seconds):
    t = i / sr
    f0 = 140 + 30 * math.sin(2 * math.pi * 0.7 * t)
    phase = 2 * math.pi * f0 * t
    voice = sum(math.sin(k * phase) / k for k in range(1, 12))
    envelope = max(0.0, math.sin(2 * math.pi * 3.5 * t)) ** 2
    noise = random.gauss(0, 0.08) + 0.05 * math.sin(2 * math.pi * 50 * t)
    sample = 0.25 * voice * envelope + noise
    frames += struct.pack("<h", int(max(-1, min(1, sample)) * 32767))
with wave.open(path, "wb") as w:
    w.setnchannels(1)
    w.setsampwidth(2)
    w.setframerate(sr)
    w.writeframes(bytes(frames))
print(f"wrote {path}")
