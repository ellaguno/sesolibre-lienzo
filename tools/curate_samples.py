#!/usr/bin/env python3
"""Curación del material crudo → pack que lee el motor (`lienzo/core`).

Salida (todo 48 kHz, estéreo, Ogg Vorbis q5 — el motor lo decodifica con lewton):
  assets/audio/instruments/<instrumento>/<midi>.ogg   notas, RMS -20 dB, sin silencio inicial
  assets/audio/ambience/<nombre>.ogg                   camas largas, RMS -24 dB
  assets/audio/events/<nombre>_<nn>.ogg                ballena, crujidos, RMS -22 dB

Con `--wav` escribe además el WAV sin comprimir (para comparar a oído).

Determinista. Volver a correr sobreescribe. Uso: python3.11 tools/curate_samples.py
"""
import glob
import os
import re

import numpy as np
import soundfile as sf
from scipy import signal

HERE = os.path.dirname(__file__)
AUDIO = os.path.join(HERE, "..", "assets", "audio")
RAW = os.path.join(AUDIO, "raw")
SR = 48000
NOTE_RE = re.compile(r"_([A-G]#?)(\d)(?=[_.])")
NAMES = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"]

INSTRUMENTS = {
    # nombre → patrón en vsco-256. Un solo nivel dinámico por instrumento, a propósito:
    # la dinámica la pone el motor; la coherencia entre notas importa más.
    "cello_sus": "CelloEns_susvib_*",
    "cello_pizz": "CelloEns_pizzT_*",
    "viola_sus": "ViolaEns_susvib_*",
    "violin_sus": "VlnEns_susVib_*",
    "violin_harm": "VlnEns_Harm_*",
    "harp": "KSHarp_*",
    "vibes": "Vibes_soft_*",
    "flute_sus": "LDFlute_susvib_*",
    "tuba": "Tuba3_sfz_*",
    "marimba": "Marimba_hit_*",
}
AMBIENCE = {
    "hydrophone_flow": ("733812", None),
    "bubble_flow": ("852478", None),
    "waves_rocks": ("705058", None),
}
# (archivo, [(inicio, fin), ...]) en segundos del crudo
EVENTS = {
    "whale": ("797762", [(199.4, 203.4), (208.7, 212.8), (266.1, 271.9), (297.3, 302.3),
                         (358.8, 362.8), (395.7, 398.9)]),
    "creak": ("818587", [(12, 18), (40, 45), (52, 58), (62, 68)]),
}


def db(x):
    return 10 ** (x / 20)


def load(path):
    d, sr = sf.read(path, always_2d=True, dtype="float32")
    if d.shape[1] == 1:
        d = np.repeat(d, 2, axis=1)
    if sr != SR:
        d = signal.resample_poly(d, SR, sr, axis=0).astype(np.float32)
    return d


def trim_head(d, thresh_db=-48.0):
    a = np.abs(d).max(1)
    idx = np.argmax(a > db(thresh_db))
    idx = max(0, idx - int(0.003 * SR))
    return d[idx:]


def fade_tail(d, secs=0.05):
    n = min(len(d), int(secs * SR))
    d[-n:] *= np.linspace(1, 0, n)[:, None]
    return d


def normalize(d, rms_db, peak_db=-1.0):
    r = np.sqrt((d ** 2).mean()) + 1e-9
    d = d * (db(rms_db) / r)
    pk = np.abs(d).max()
    if pk > db(peak_db):
        d *= db(peak_db) / pk
    return d


def midi_of(fname):
    m = NOTE_RE.search(fname)
    return NAMES.index(m.group(1)) + 12 * (int(m.group(2)) + 1)


import subprocess
import sys

KEEP_WAV = "--wav" in sys.argv
VORBIS_Q = "5"


WRITTEN = []


def write(path, d):
    """`path` llega con .wav; se escribe .ogg (y el .wav solo con --wav)."""
    os.makedirs(os.path.dirname(path), exist_ok=True)
    WRITTEN.append(os.path.relpath(path[:-4] + ".ogg", AUDIO))
    tmp = path + ".tmp.wav"
    sf.write(tmp, np.clip(d, -1, 1), SR, subtype="PCM_16")
    ogg = path[:-4] + ".ogg"
    subprocess.run(["ffmpeg", "-v", "error", "-y", "-i", tmp, "-c:a", "libvorbis", "-q:a", VORBIS_Q, ogg], check=True)
    if KEEP_WAV:
        os.replace(tmp, path)
    else:
        os.remove(tmp)
        if os.path.exists(path):
            os.remove(path)


def main():
    n = 0
    for inst, pat in INSTRUMENTS.items():
        files = sorted(glob.glob(os.path.join(RAW, "vsco-256", pat)))
        if not files:
            print("  (sin archivos para", inst, ")")
            continue
        for f in files:
            d = fade_tail(normalize(trim_head(load(f)), -20.0))
            write(os.path.join(AUDIO, "instruments", inst, f"{midi_of(os.path.basename(f))}.wav"), d)
            n += 1
        print(f"{inst:12s} {len(files)} notas: {sorted(midi_of(os.path.basename(f)) for f in files)}")

    for name, (fid, _) in AMBIENCE.items():
        f = glob.glob(os.path.join(RAW, "freesound", fid + "*"))[0]
        write(os.path.join(AUDIO, "ambience", name + ".wav"), normalize(load(f), -24.0))
        n += 1
        print(f"ambience/{name}")

    for name, (fid, ranges) in EVENTS.items():
        f = glob.glob(os.path.join(RAW, "freesound", fid + "*"))[0]
        full = load(f)
        for i, (s, e) in enumerate(ranges):
            clip = full[int(s * SR):int(e * SR)].copy()
            r = int(0.25 * SR)
            clip[:r] *= np.linspace(0, 1, r)[:, None]
            clip[-r:] *= np.linspace(1, 0, r)[:, None]
            write(os.path.join(AUDIO, "events", f"{name}_{i + 1:02d}.wav"), normalize(clip, -22.0))
            n += 1
        print(f"events/{name}: {len(ranges)} clips")
    # El índice del pack: el motor lo lee en vez de listar carpetas, que en un
    # APK o en el navegador no se puede.
    with open(os.path.join(AUDIO, "pack.txt"), "w") as f:
        f.write("\n".join(sorted(WRITTEN)) + "\n")
    print(f"{n} archivos escritos · índice en pack.txt")


if __name__ == "__main__":
    main()
