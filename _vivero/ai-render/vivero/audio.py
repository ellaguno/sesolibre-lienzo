"""E/S de WAV y DSP mínimo. Solo numpy + scipy: nada que instalar."""

from __future__ import annotations

import io
import wave

import numpy as np
from scipy.signal import fftconvolve

SR = 44100


# --------------------------------------------------------------------------
# E/S
# --------------------------------------------------------------------------

def read_wav(path: str) -> tuple[np.ndarray, int]:
    """Devuelve (mono float32 en [-1,1], sample rate)."""
    with wave.open(path, "rb") as w:
        n, ch, sw, sr = w.getnframes(), w.getnchannels(), w.getsampwidth(), w.getframerate()
        raw = w.readframes(n)
    if sw != 2:
        raise ValueError(f"{path}: solo se soportan WAV de 16 bits (tiene {sw * 8})")
    x = np.frombuffer(raw, dtype="<i2").astype(np.float32) / 32768.0
    if ch > 1:
        x = x.reshape(-1, ch).mean(axis=1)
    return x, sr


def write_wav(path_or_buf, x: np.ndarray, sr: int = SR) -> None:
    """Escribe estéreo si x es (n,2), mono si es (n,)."""
    x = np.asarray(x, dtype=np.float32)
    stereo = x.ndim == 2
    pcm = np.clip(x, -1.0, 1.0)
    pcm = (pcm * 32767.0).astype("<i2")
    with wave.open(path_or_buf, "wb") as w:
        w.setnchannels(2 if stereo else 1)
        w.setsampwidth(2)
        w.setframerate(sr)
        w.writeframes(pcm.tobytes())


def wav_bytes(x: np.ndarray, sr: int = SR) -> bytes:
    buf = io.BytesIO()
    write_wav(buf, x, sr)
    return buf.getvalue()


# --------------------------------------------------------------------------
# DSP
# --------------------------------------------------------------------------

def resample(x: np.ndarray, factor: float) -> np.ndarray:
    """
    Transposición por remuestreo lineal. Cambia la duración además de la altura
    — igual que `pitch_scale` en Godot, y eso es deliberado: el render tiene que
    sonar como lo que el jugador oyó al construir, no "mejor".
    """
    if abs(factor - 1.0) < 1e-6:
        return x
    n_out = max(1, int(round(len(x) / factor)))
    idx = np.arange(n_out, dtype=np.float64) * factor
    idx = np.clip(idx, 0, len(x) - 1)
    lo = idx.astype(np.int64)
    hi = np.minimum(lo + 1, len(x) - 1)
    frac = (idx - lo).astype(np.float32)
    return x[lo] * (1.0 - frac) + x[hi] * frac


def add_at(dst: np.ndarray, src: np.ndarray, offset: int, gain: float = 1.0) -> None:
    """Suma `src` sobre `dst` en `offset`, recortando lo que se sale."""
    if offset >= len(dst):
        return
    if offset < 0:
        src = src[-offset:]
        offset = 0
    n = min(len(src), len(dst) - offset)
    if n > 0:
        dst[offset:offset + n] += src[:n] * gain


def make_ir(seconds: float = 2.6, sr: int = SR, damping: float = 0.32,
            seed: int = 5) -> np.ndarray:
    """
    Respuesta al impulso estéreo: ruido con caída exponencial y filtrado. Los dos
    canales usan ruido distinto para que la cola tenga anchura real y no sea un
    mono duplicado.
    """
    rng = np.random.default_rng(seed)
    n = int(sr * seconds)
    t = np.arange(n) / sr
    env = np.exp(-t / (seconds * damping))
    ir = np.stack([rng.uniform(-1, 1, n) * env, rng.uniform(-1, 1, n) * env], axis=1)
    # Pasa-bajos de un polo: una cola brillante suena a lata.
    a = 1.0 - np.exp(-2.0 * np.pi * 2600.0 / sr)
    for c in range(2):
        prev = 0.0
        col = ir[:, c]
        for i in range(n):
            prev += a * (col[i] - prev)
            col[i] = prev
    # Predelay corto: separa la fuente de la cola.
    pre = int(sr * 0.018)
    ir = np.concatenate([np.zeros((pre, 2), dtype=ir.dtype), ir])
    peak = np.max(np.abs(ir))
    return (ir / peak * 0.5).astype(np.float32) if peak > 0 else ir


def reverb(x: np.ndarray, ir: np.ndarray, wet: float = 0.45) -> np.ndarray:
    """Convolución. `x` mono o estéreo -> estéreo."""
    if x.ndim == 1:
        x = np.stack([x, x], axis=1)
    wet_sig = np.stack(
        [fftconvolve(x[:, c], ir[:, c], mode="full")[: len(x)] for c in range(2)],
        axis=1,
    )
    peak = np.max(np.abs(wet_sig))
    if peak > 0:
        wet_sig = wet_sig / peak * np.max(np.abs(x))
    return x * (1.0 - wet * 0.4) + wet_sig * wet


def pan(x: np.ndarray, position: float) -> np.ndarray:
    """position en [-1,1]. Ley de potencia constante."""
    p = (np.clip(position, -1.0, 1.0) + 1.0) * 0.25 * np.pi
    return np.stack([x * np.cos(p), x * np.sin(p)], axis=1)


def normalize(x: np.ndarray, peak: float = 0.92) -> np.ndarray:
    m = np.max(np.abs(x))
    return x * (peak / m) if m > 1e-9 else x


def fade(x: np.ndarray, in_s: float = 0.02, out_s: float = 2.5,
         sr: int = SR) -> np.ndarray:
    x = x.copy()
    n_in, n_out = int(sr * in_s), int(sr * out_s)
    if n_in:
        ramp = np.linspace(0, 1, n_in, dtype=np.float32)
        x[:n_in] *= ramp[:, None] if x.ndim == 2 else ramp
    if n_out and n_out < len(x):
        ramp = np.linspace(1, 0, n_out, dtype=np.float32)
        x[-n_out:] *= ramp[:, None] if x.ndim == 2 else ramp
    return x


def to_db(gain_db: float) -> float:
    return float(10.0 ** (gain_db / 20.0))
