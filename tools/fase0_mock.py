#!/usr/bin/env python3
"""Fase 0 — maqueta de 60 s del abismo, armada a mano con las muestras reales.

No es el motor. Es una prueba de GUSTO: ¿estas muestras, apiladas así, suenan a lo
que queremos? Todo determinista. Salida: assets/audio/fase0/abismo_60s.wav (+ .ogg)

Uso: python3.11 tools/fase0_mock.py
"""
import glob
import os
import re

import numpy as np
import soundfile as sf
from scipy import signal

ROOT = os.path.join(os.path.dirname(__file__), "..", "assets", "audio")
RAW = os.path.join(ROOT, "raw")
OUT = os.path.join(ROOT, "fase0")
SR = 48000
DUR = 64.0
NOTE_RE = re.compile(r"_([A-G]#?)(\d)(?=[_.])")
NAMES = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"]

rng = np.random.default_rng(7)


def db(x):
    return 10 ** (x / 20)


def load(path):
    d, sr = sf.read(path, always_2d=True, dtype="float32")
    if d.shape[1] == 1:
        d = np.repeat(d, 2, axis=1)
    if sr != SR:
        d = signal.resample_poly(d, SR, sr, axis=0).astype(np.float32)
    return d


def rms_norm(d, target_db=-20.0):
    r = np.sqrt((d ** 2).mean()) + 1e-9
    return d * (db(target_db) / r)


def midi_of_name(fname):
    m = NOTE_RE.search(fname)
    return NAMES.index(m.group(1)) + 12 * (int(m.group(2)) + 1)


def pitch(d, semis):
    if semis == 0:
        return d
    ratio = 2 ** (semis / 12)
    n = int(len(d) / ratio)
    idx = np.linspace(0, len(d) - 1, n)
    return np.stack([np.interp(idx, np.arange(len(d)), d[:, c]) for c in range(2)], 1)


def env(d, attack=0.01, release=0.5, hold=None):
    """Fade de entrada; si hold, corta a hold s con release."""
    n = len(d)
    if hold is not None:
        n = min(n, int((hold + release) * SR))
        d = d[:n]
    e = np.ones(n, np.float32)
    a = min(n, int(attack * SR))
    e[:a] = np.linspace(0, 1, a)
    r = min(n, int(release * SR))
    e[n - r:] *= np.linspace(1, 0, r)
    return d * e[:, None]


def lowpass(d, hz, order=2):
    sos = signal.butter(order, hz, "lowpass", fs=SR, output="sos")
    return signal.sosfilt(sos, d, axis=0)


def highpass(d, hz, order=2):
    sos = signal.butter(order, hz, "lowpass" if False else "highpass", fs=SR, output="sos")
    return signal.sosfilt(sos, d, axis=0)


class Mix:
    def __init__(self):
        self.buf = np.zeros((int(DUR * SR), 2), np.float32)

    def add(self, d, t, gain_db=0.0, pan=0.0):
        g = db(gain_db)
        l, r = g * np.cos((pan + 1) * np.pi / 4), g * np.sin((pan + 1) * np.pi / 4)
        i = int(t * SR)
        n = min(len(d), len(self.buf) - i)
        if n <= 0:
            return
        self.buf[i:i + n, 0] += d[:n, 0] * l
        self.buf[i:i + n, 1] += d[:n, 1] * r


class Instrument:
    def __init__(self, pattern, level_db=-20.0, pre=None):
        self.samples = {}
        for f in sorted(glob.glob(os.path.join(RAW, "vsco-256", pattern))):
            d = rms_norm(load(f), level_db)
            if pre:
                d = pre(d)
            self.samples[midi_of_name(os.path.basename(f))] = d
        assert self.samples, pattern

    def note(self, midi, hold=None, attack=0.01, release=0.6):
        near = min(self.samples, key=lambda m: abs(m - midi))
        return env(pitch(self.samples[near], midi - near), attack, release, hold)


# ---- escala: Re pentatónica mayor (D E F# A B). Todo lo tonal sale de aquí. ----
D3 = 50
PENTA = [0, 2, 4, 7, 9]


def deg(i, octave=0):
    return D3 + 12 * (octave + i // 5) + PENTA[i % 5]


def stroke_to_motif(ys, n_notes, lo_deg, span_deg):
    """Un trazo (alturas 0..1 a lo largo del gesto) → grados de la escala."""
    xs = np.linspace(0, 1, n_notes)
    yy = np.interp(xs, np.linspace(0, 1, len(ys)), ys)
    return [lo_deg + int(round(y * span_deg)) for y in yy]


def main():
    os.makedirs(OUT, exist_ok=True)
    mix = Mix()
    fs = lambda name: glob.glob(os.path.join(RAW, "freesound", name + "*"))[0]

    # 1. AMBIENTE ---------------------------------------------------------
    flow = rms_norm(load(fs("733812")), -25)
    flow = lowpass(flow, 1800)
    # dos copias a velocidades distintas: el bucle no se percibe
    mix.add(env(flow, 4, 6), 0, 0, -0.2)
    mix.add(env(pitch(flow, -3), 4, 6), 1.5, -4, 0.3)
    bub = lowpass(rms_norm(load(fs("852478")), -28), 6000)
    mix.add(env(bub, 6, 8), 8, 0)

    # 2. COLCHÓN: cuerdas sosteniendo el acorde; retriggers cruzados ---------
    viola = Instrument("ViolaEns_susvib_*", -24)
    vln = Instrument("VlnEns_susVib_*", -26)
    cello_sus = Instrument("CelloEns_susvib_*", -22)

    def pad_chord(notes, t0, t1, inst_for):
        t = t0
        while t < t1:
            for m in notes:
                inst = inst_for(m)
                mix.add(inst.note(m, hold=7.5, attack=2.5, release=3.0), t,
                        -10 + rng.uniform(-1.5, 0), rng.uniform(-0.5, 0.5))
            t += 6.0

    pick = lambda m: cello_sus if m < 55 else (viola if m < 67 else vln)
    # D2 · A2 · E3 · A3 — quintas abiertas, como el drone de abyssal-garden
    pad_chord([deg(0, -1), deg(3, -1), deg(1, 0), deg(3, 0)], 4, 30, pick)
    # B2 · F#3 · D4 · A4 — relativo menor, la pieza se oscurece
    pad_chord([deg(4, -1), deg(2, 0), deg(0, 1), deg(3, 1)], 30, 52, pick)
    pad_chord([deg(0, -1), deg(3, -1), deg(1, 0), deg(3, 0)], 52, 64, pick)

    # 3. KELP → cello. Un trazo que sube y baja, lento. --------------------
    trazo_kelp = [0.1, 0.35, 0.7, 0.9, 0.75, 0.4, 0.2]
    motif = stroke_to_motif(trazo_kelp, 7, 0, 6)     # D3..B3
    beat = 60 / 56
    for i, g in enumerate(motif):
        mix.add(cello_sus.note(deg(g), hold=beat * 1.6, attack=0.4, release=1.2),
                8 + i * beat * 2, -7, -0.3)
    # variación: inversión, un poco más grave, a los 38 s
    inv = [motif[0] - (g - motif[0]) for g in motif]
    for i, g in enumerate(inv):
        mix.add(cello_sus.note(deg(g), hold=beat * 1.6, attack=0.4, release=1.2),
                38 + i * beat * 2, -9, -0.3)

    # 4. CARDUMEN → arpa. Trazo rápido y ondulado. ---------------------------
    harp = Instrument("KSHarp_*", -22)
    trazo_peces = [0.2, 0.5, 0.8, 0.6, 0.9, 0.7, 0.4, 0.5, 0.3]
    motif_h = stroke_to_motif(trazo_peces, 9, 5, 7)   # D4..
    step = beat / 2
    for i, g in enumerate(motif_h):
        mix.add(harp.note(deg(g), release=1.5), 16 + i * step + rng.uniform(-0.01, 0.01),
                -11 + rng.uniform(-2, 0), 0.35)
    # variación: transpuesta una tercera (dos grados) arriba, con eco, a los 44 s
    for i, g in enumerate(motif_h):
        t = 44 + i * step
        mix.add(harp.note(deg(g + 2), release=1.5), t, -11, 0.4)
        mix.add(harp.note(deg(g + 2), release=1.5), t + step * 1.5, -20, -0.4)

    # 5. ANÉMONA → vibráfono. Notas sueltas, lejanas. -------------------------
    vibes = Instrument("Vibes_soft_*", -24)
    for t, g, p in [(21.5, 11, -0.6), (27.0, 13, 0.5), (34.5, 12, -0.2),
                    (50.0, 14, 0.6), (56.5, 11, -0.5)]:
        mix.add(vibes.note(deg(g), release=3.0), t, -14, p)

    # 6. BALLENA: grabación real, lejos y filtrada. ----------------------------
    whale_all = load(fs("797762"))
    for (s, e), t, p in [((199.4, 203.4), 24.0, -0.7), ((266.1, 271.9), 46.0, 0.6)]:
        clip = whale_all[int(s * SR):int(e * SR)]
        clip = lowpass(rms_norm(clip, -21), 1100, 4)
        mix.add(env(clip, 0.4, 0.8), t, 0, p)

    # 7. BARCO: un crujido, muy quedo. -------------------------------------
    creak = load(fs("818587"))
    c = lowpass(rms_norm(creak[int(12 * SR):int(18 * SR)], -27), 2500)
    mix.add(env(c, 0.5, 1.0), 36.0, 0, 0.7)
    c2 = lowpass(rms_norm(creak[int(40 * SR):int(45 * SR)], -29), 2500)
    mix.add(env(c2, 0.5, 1.0), 57.0, 0, -0.6)

    # ---- master: bajo el agua, reverb larga, limitador ----------------------
    dry = lowpass(mix.buf, 7000)
    ir_len = int(4.5 * SR)
    t = np.arange(ir_len) / SR
    ir = rng.standard_normal((ir_len, 2)).astype(np.float32) * np.exp(-t / 1.3)[:, None]
    ir = lowpass(ir, 2500, 2)
    ir /= np.abs(ir).sum(0).max() / 6
    wet = np.stack([signal.fftconvolve(dry[:, c], ir[:, c])[:len(dry)] for c in range(2)], 1)
    out = dry + 0.45 * wet
    out = np.tanh(out * 1.4) / 1.4
    out *= db(-1.0) / np.abs(out).max()
    out = env(out, 0.0, 3.0)

    wav = os.path.join(OUT, "abismo_60s.wav")
    sf.write(wav, out, SR, subtype="PCM_16")
    rms = 20 * np.log10(np.sqrt((out ** 2).mean()))
    print(f"{wav}: {len(out)/SR:.0f} s, pico -1.0 dB, rms {rms:.1f} dB")
    os.system(f'ffmpeg -v error -y -i "{wav}" -c:a libvorbis -q:a 6 "{wav[:-4]}.ogg"')


if __name__ == "__main__":
    main()
