#!/usr/bin/env python3
"""
Servicio de render. Solo stdlib: nada que instalar, nada que desplegar.

    python3 server.py                 # 127.0.0.1:8900
    python3 server.py --port 9000 --host 0.0.0.0

    GET  /health                 estado de los proveedores
    POST /render                 {payload, midi_b64?, duration_s?, seed?, provider?}
                                 -> 202 {job_id, huella, cacheado}
    GET  /jobs/<id>              estado del trabajo
    GET  /jobs/<id>/audio        el audio
    GET  /jobs                   últimos trabajos

Modelo asíncrono a propósito: un render de IA tarda 30-90 s, más de lo que puede
esperar una petición HTTP de móvil. El juego lanza el trabajo, guarda el id y
vuelve a preguntar (o recibe una push). Ver ../godot-mvp/src/core/ai_render.gd.

Por defecto escucha SOLO en localhost. Exponerlo a la red requiere --host
explícito, y entonces hace falta autenticación delante: no la trae.
"""

from __future__ import annotations

import argparse
import base64
import json
import os
import sys
import threading
import time
import uuid
from concurrent.futures import ThreadPoolExecutor
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from vivero import midi as midi_mod, payload as payload_mod, providers  # noqa: E402

MAX_BODY = 4 * 1024 * 1024          # un payload denso ronda los 10 KiB
CACHE_DIR = os.environ.get("VIVERO_CACHE", os.path.join(
    os.path.dirname(os.path.abspath(__file__)), "out"))
# Uno por defecto: los renders con GPU no se paralelizan bien y competir por VRAM
# es peor que hacer cola.
WORKERS = int(os.environ.get("VIVERO_WORKERS", "1"))

_jobs: dict[str, dict] = {}
_lock = threading.Lock()
_pool = ThreadPoolExecutor(max_workers=WORKERS)


def _now() -> float:
    return time.time()


def _set(job_id: str, **fields) -> None:
    with _lock:
        _jobs[job_id].update(fields)


def _cache_path(fingerprint: str, provider: str, ext: str = ".wav") -> str:
    return os.path.join(CACHE_DIR, f"{fingerprint}-{provider}{ext}")


def _duration_of(path: str) -> float:
    """
    Duración real del archivo en caché. Sin esto, un acierto de caché devolvía
    `duracion_s = 0` y el juego mostraba "0 s" en el aviso.
    """
    if path.lower().endswith(".wav"):
        try:
            import wave
            with wave.open(path, "rb") as w:
                return round(w.getnframes() / float(w.getframerate()), 2)
        except Exception:                              # noqa: BLE001
            return 0.0
    return 0.0                                         # mp3: haría falta un decoder


def _work(job_id: str, req: providers.RenderRequest, provider: str,
          allow_fallback: bool) -> None:
    _set(job_id, estado="renderizando", inicio=_now())
    try:
        result = providers.render(req, provider, allow_fallback=allow_fallback)
    except Exception as exc:                          # noqa: BLE001
        _set(job_id, estado="error", error=str(exc), fin=_now())
        return

    ext = ".mp3" if "mpeg" in result.mime else ".wav"
    os.makedirs(CACHE_DIR, exist_ok=True)
    path = _cache_path(req.payload.fingerprint(), result.provider, ext)
    with open(path, "wb") as f:
        f.write(result.audio)
    _set(job_id, estado="listo", fin=_now(), archivo=path, mime=result.mime,
         proveedor_real=result.provider, fallback=result.fallback,
         prompt=result.prompt, duracion_s=round(result.duration_s, 2),
         bytes=len(result.audio), notas=result.notes)


class Handler(BaseHTTPRequestHandler):
    server_version = "ViveroRender/1.0"

    # ---- utilidades ----

    def _json(self, code: int, obj) -> None:
        body = json.dumps(obj, ensure_ascii=False, indent=1).encode("utf-8")
        self.send_response(code)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def _err(self, code: int, msg: str) -> None:
        self._json(code, {"error": msg})

    def log_message(self, fmt, *args):
        sys.stderr.write("  %s %s\n" % (self.address_string(), fmt % args))

    # ---- rutas ----

    def do_GET(self) -> None:                         # noqa: N802
        path = self.path.split("?", 1)[0].rstrip("/") or "/"

        if path == "/health":
            self._json(200, {"ok": True, "por_defecto": providers.DEFAULT,
                             "trabajadores": WORKERS,
                             "proveedores": providers.status()})
        elif path == "/jobs":
            with _lock:
                items = [{"job_id": k, **{f: v for f, v in j.items() if f != "notas"}}
                         for k, j in list(_jobs.items())[-25:]]
            self._json(200, {"trabajos": items})
        elif path.startswith("/jobs/"):
            rest = path[len("/jobs/"):]
            if rest.endswith("/audio"):
                self._serve_audio(rest[:-len("/audio")])
            else:
                with _lock:
                    job = _jobs.get(rest)
                if job is None:
                    self._err(404, "trabajo desconocido")
                else:
                    self._json(200, {"job_id": rest, **job})
        else:
            self._err(404, "ruta desconocida")

    def _serve_audio(self, job_id: str) -> None:
        with _lock:
            job = _jobs.get(job_id)
        if job is None:
            return self._err(404, "trabajo desconocido")
        if job.get("estado") != "listo":
            return self._err(409, f"el trabajo está en estado '{job.get('estado')}'")
        path = job.get("archivo")
        if not path or not os.path.exists(path):
            return self._err(410, "el audio ya no está en disco")
        size = os.path.getsize(path)
        self.send_response(200)
        self.send_header("Content-Type", job.get("mime", "audio/wav"))
        self.send_header("Content-Length", str(size))
        self.send_header("Content-Disposition",
                         f'attachment; filename="{os.path.basename(path)}"')
        self.end_headers()
        with open(path, "rb") as f:
            while chunk := f.read(64 * 1024):
                self.wfile.write(chunk)

    def do_POST(self) -> None:                        # noqa: N802
        if self.path.split("?", 1)[0].rstrip("/") != "/render":
            return self._err(404, "ruta desconocida")

        length = int(self.headers.get("Content-Length") or 0)
        if length <= 0:
            return self._err(400, "cuerpo vacío")
        if length > MAX_BODY:
            return self._err(413, f"cuerpo demasiado grande (máx {MAX_BODY} bytes)")

        try:
            body = json.loads(self.rfile.read(length).decode("utf-8"))
        except (UnicodeDecodeError, json.JSONDecodeError) as exc:
            return self._err(400, f"JSON inválido: {exc}")
        if not isinstance(body, dict):
            return self._err(400, "se esperaba un objeto JSON")

        # El juego puede enviar {payload: {...}} o el payload directamente.
        raw = body.get("payload", body)
        try:
            p = payload_mod.load(raw)
        except payload_mod.PayloadError as exc:
            return self._err(422, f"payload inválido: {exc}")

        midi_bytes = None
        if body.get("midi_b64"):
            try:
                midi_bytes = base64.b64decode(body["midi_b64"], validate=True)
                midi_mod.parse(midi_bytes)            # se valida antes de encolar
            except Exception as exc:                  # noqa: BLE001
                return self._err(422, f"midi inválido: {exc}")

        provider = str(body.get("provider") or providers.DEFAULT)
        try:
            providers.get(provider)
        except providers.ProviderError as exc:
            return self._err(422, str(exc))

        duration = float(body.get("duration_s") or 150.0)
        if not 5.0 <= duration <= 600.0:
            return self._err(422, "duration_s fuera de rango (5..600)")

        job_id = uuid.uuid4().hex[:12]
        fingerprint = p.fingerprint()

        # Caché: renderizar dos veces el mismo jardín no debe costar dos veces.
        for ext in (".wav", ".mp3"):
            hit = _cache_path(fingerprint, provider, ext)
            if os.path.exists(hit) and not body.get("force"):
                with _lock:
                    _jobs[job_id] = {
                        "estado": "listo", "huella": fingerprint, "archivo": hit,
                        "mime": "audio/mpeg" if ext == ".mp3" else "audio/wav",
                        "proveedor_real": provider, "fallback": False,
                        "desde_cache": True, "bytes": os.path.getsize(hit),
                        "duracion_s": _duration_of(hit),
                        "prompt": p.text_prompt(), "creado": _now(),
                    }
                return self._json(202, {"job_id": job_id, "huella": fingerprint,
                                        "cacheado": True})

        with _lock:
            _jobs[job_id] = {"estado": "pendiente", "huella": fingerprint,
                             "proveedor": provider, "creado": _now(),
                             "avisos": p.warnings}
        req = providers.RenderRequest(
            payload=p, midi=midi_bytes, duration_s=duration,
            seed=int(body.get("seed") or 0),
            sample_pack=str(body.get("sample_pack") or ""))
        _pool.submit(_work, job_id, req, provider, not body.get("strict"))
        return self._json(202, {"job_id": job_id, "huella": fingerprint,
                                "cacheado": False})


def main() -> int:
    ap = argparse.ArgumentParser(description="Servicio de render de Vivero")
    ap.add_argument("--host", default="127.0.0.1",
                    help="127.0.0.1 por defecto; cambiarlo expone el servicio")
    ap.add_argument("--port", type=int, default=8900)
    args = ap.parse_args()

    os.makedirs(CACHE_DIR, exist_ok=True)
    srv = ThreadingHTTPServer((args.host, args.port), Handler)
    print(f"Vivero render en http://{args.host}:{args.port}")
    print(f"  proveedor por defecto: {providers.DEFAULT}   trabajadores: {WORKERS}")
    print(f"  caché: {CACHE_DIR}")
    for s in providers.status():
        if s["disponible"]:
            print(f"  ✓ {s['proveedor']}")
    if args.host not in ("127.0.0.1", "localhost"):
        print("  !! escuchando fuera de localhost y SIN autenticación")
    try:
        srv.serve_forever()
    except KeyboardInterrupt:
        print("\nadiós")
    finally:
        srv.server_close()
        _pool.shutdown(wait=False)
    return 0


if __name__ == "__main__":
    sys.exit(main())
