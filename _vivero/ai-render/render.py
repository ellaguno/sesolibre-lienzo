#!/usr/bin/env python3
"""
CLI del render. Un payload del juego entra, un audio sale.

    python3 render.py estado                      # qué proveedores hay
    python3 render.py payload.json                # rinde con el proveedor por defecto
    python3 render.py payload.json -p musicgen    # con uno concreto
    python3 render.py payload.json --info         # solo inspecciona, sin renderizar
    python3 render.py melodia.mid --info          # inspecciona un .mid
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from vivero import midi, payload as payload_mod, providers  # noqa: E402

CACHE_DIR = os.environ.get("VIVERO_CACHE", os.path.join(
    os.path.dirname(os.path.abspath(__file__)), "out"))


def cmd_status() -> int:
    print(f"proveedor por defecto: {providers.DEFAULT}\n")
    for s in providers.status():
        mark = "✓" if s["disponible"] else "·"
        star = " (por defecto)" if s["por_defecto"] else ""
        mel = " [melody conditioning]" if s["melodia"] else ""
        print(f"  {mark} {s['proveedor']:11s}{mel}{star}")
        if not s["disponible"]:
            print(f"      {s['motivo']}")
    return 0


def cmd_info(path: str) -> int:
    if path.lower().endswith(".mid"):
        m = midi.parse_file(path)
        print(f"MIDI  ppq={m.ppq}  bpm={m.bpm:.1f}  notas={len(m.notes)}  "
              f"duración={m.duration_s:.1f}s")
        if m.notes:
            lo = min(n.pitch for n in m.notes)
            hi = max(n.pitch for n in m.notes)
            print(f"      rango midi {lo}..{hi}")
            for n in m.notes[:8]:
                print(f"      t={n.start_s(m.ppq, m.bpm):6.2f}s  midi={n.pitch:3d}  "
                      f"dur={n.dur_s(m.ppq, m.bpm):.2f}s  vel={n.velocity}")
            if len(m.notes) > 8:
                print(f"      … {len(m.notes) - 8} más")
        return 0

    p = payload_mod.load_file(path)
    print(f"bioma={p.bioma}  momento={p.momento}  bpm={p.bpm:.0f}  escala={p.escala}")
    print(f"objetos={sum(p.objetos.values())}  densidad={p.densidad:.3f}  "
          f"ciclo={p.cycle_seconds:.2f}s")
    print(f"capas={ {k: v for k, v in p.capas_activas.items() if v} }")
    print(f"huella={p.fingerprint()}")
    print(f"\nprompt (es, del juego):\n  {p.prompt_texto}")
    print(f"\nprompt (en, para el modelo):\n  {p.text_prompt()}")
    print(f"\nnegativo:\n  {p.negative_prompt()}")
    for w in p.warnings:
        print(f"\n  AVISO: {w}")
    return 0


def cmd_render(args) -> int:
    p = payload_mod.load_file(args.payload)
    for w in p.warnings:
        print(f"AVISO: {w}", file=sys.stderr)

    midi_bytes = None
    if args.midi:
        with open(args.midi, "rb") as f:
            midi_bytes = f.read()
        midi.parse(midi_bytes)          # valida antes de mandarlo a ningún sitio

    os.makedirs(CACHE_DIR, exist_ok=True)
    provider = args.provider or providers.DEFAULT
    stem = f"{p.fingerprint()}-{provider}"
    cached = os.path.join(CACHE_DIR, stem + ".wav")
    if os.path.exists(cached) and not args.force:
        print(f"en caché: {cached}  (--force para regenerar)")
        return 0

    req = providers.RenderRequest(
        payload=p, midi=midi_bytes, duration_s=args.duration,
        seed=args.seed, sample_pack=args.pack or "")

    t0 = time.time()
    result = providers.render(req, provider, allow_fallback=not args.strict)
    elapsed = time.time() - t0

    ext = ".mp3" if "mpeg" in result.mime else ".wav"
    out = os.path.join(CACHE_DIR, stem + ext)
    with open(out, "wb") as f:
        f.write(result.audio)

    meta = {
        "proveedor": result.provider,
        "fallback": result.fallback,
        "prompt": result.prompt,
        "duracion_s": round(result.duration_s, 2),
        "segundos_de_render": round(elapsed, 2),
        "huella": p.fingerprint(),
        "notas": result.notes,
        "payload": os.path.basename(args.payload),
    }
    with open(os.path.join(CACHE_DIR, stem + ".json"), "w", encoding="utf-8") as f:
        json.dump(meta, f, indent=2, ensure_ascii=False)

    print(f"{out}  ({len(result.audio) / 1024:.0f} KiB, "
          f"{result.duration_s:.1f}s, en {elapsed:.1f}s)")
    if result.fallback:
        print("  !! FALLBACK al proveedor local")
    for n in result.notes:
        print(f"  · {n}")

    if args.mp3 and ext == ".wav":
        mp3 = out[:-4] + ".mp3"
        try:
            subprocess.run(["ffmpeg", "-y", "-loglevel", "error", "-i", out,
                            "-b:a", "192k", mp3], check=True)
            print(f"  · mp3: {mp3}")
        except (OSError, subprocess.CalledProcessError) as exc:
            print(f"  · no se pudo crear el mp3: {exc}", file=sys.stderr)
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description="Render de audio de Vivero")
    ap.add_argument("payload", nargs="?", help="payload .json (o .mid con --info)")
    ap.add_argument("-p", "--provider", help="local | musicgen | stability | elevenlabs")
    ap.add_argument("-m", "--midi", help="melodia .mid para condicionamiento")
    ap.add_argument("-d", "--duration", type=float, default=150.0,
                    help="duración objetivo en segundos (por defecto 150)")
    ap.add_argument("-s", "--seed", type=int, default=0)
    ap.add_argument("--pack", help="carpeta del sample pack (proveedor local)")
    ap.add_argument("--info", action="store_true", help="solo inspeccionar")
    ap.add_argument("--strict", action="store_true", help="sin fallback a local")
    ap.add_argument("--force", action="store_true", help="ignorar la caché")
    ap.add_argument("--mp3", action="store_true", help="además, convertir a mp3")
    args = ap.parse_args()

    if args.payload in (None, "estado", "status"):
        return cmd_status()
    if not os.path.exists(args.payload):
        print(f"no existe: {args.payload}", file=sys.stderr)
        return 2
    try:
        if args.info:
            return cmd_info(args.payload)
        return cmd_render(args)
    except (payload_mod.PayloadError, midi.MidiError) as exc:
        print(f"payload/midi inválido: {exc}", file=sys.stderr)
        return 2
    except providers.ProviderError as exc:
        print(f"error del proveedor: {exc}", file=sys.stderr)
        return 3


if __name__ == "__main__":
    sys.exit(main())
