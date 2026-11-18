"""
Proveedores HTTP comerciales: Stability Audio y ElevenLabs Music.

Son los candidatos con **licencia comercial explícita**, que es la razón para
preferirlos sobre Suno/Udio en producción pese a que estos den mejor resultado
subjetivo: Suno y Udio tienen litigios de copyright abiertos y acceso a API
restringido, y construir el producto encima es asumir un riesgo de dependencia
que no controlas.

Ninguno de los dos ofrece melody conditioning con MIDI. Eso significa que con
ellos el render es "una pieza generada a partir de la descripción de tu jardín",
no "tu jardín producido". Es una diferencia de producto, no solo técnica, y hay
que decidirla a oído comparando contra el proveedor `local`.

Los endpoints y nombres de campo cambian; están centralizados arriba de cada
clase para que ajustarlos sea una línea. NO se han podido probar contra la API
real: hacen falta credenciales.
"""

from __future__ import annotations

import os

from .base import Provider, ProviderError, RenderRequest, RenderResult

TIMEOUT = float(os.environ.get("VIVERO_HTTP_TIMEOUT", "180"))


def _post(url: str, headers: dict, timeout: float = TIMEOUT, **kwargs):
    import requests
    try:
        r = requests.post(url, headers=headers, timeout=timeout, **kwargs)
    except Exception as exc:                          # noqa: BLE001
        raise ProviderError(f"fallo de red: {exc}") from exc
    if r.status_code == 401:
        raise ProviderError("credenciales rechazadas (401)")
    if r.status_code == 429:
        raise ProviderError("límite de tasa alcanzado (429)")
    if not r.ok:
        raise ProviderError(f"HTTP {r.status_code}: {r.text[:300]}")
    return r


class StabilityAudioProvider(Provider):
    name = "stability"
    credentials = ("STABILITY_API_KEY",)
    supports_melody = False

    URL = os.environ.get(
        "VIVERO_STABILITY_URL",
        "https://api.stability.ai/v2beta/audio/stable-audio-2/text-to-audio")

    def render(self, req: RenderRequest) -> RenderResult:
        ok, why = self.available()
        if not ok:
            raise ProviderError(why)
        prompt = req.payload.text_prompt()
        r = _post(
            self.URL,
            headers={"authorization": f"Bearer {os.environ['STABILITY_API_KEY']}",
                     "accept": "audio/*"},
            files={"none": ""},
            data={"prompt": prompt,
                  "negative_prompt": req.payload.negative_prompt(),
                  "duration": int(max(10, min(190, req.duration_s))),
                  "seed": int(req.seed),
                  "output_format": "wav",
                  "steps": 8},
        )
        return RenderResult(
            audio=r.content,
            mime=r.headers.get("content-type", "audio/wav"),
            provider=self.name, prompt=prompt, duration_s=req.duration_s,
            notes=["sin condicionamiento melódico: el modelo no lo soporta"])


class ElevenLabsMusicProvider(Provider):
    name = "elevenlabs"
    credentials = ("ELEVENLABS_API_KEY",)
    supports_melody = False

    URL = os.environ.get("VIVERO_ELEVENLABS_URL",
                         "https://api.elevenlabs.io/v1/music")

    def render(self, req: RenderRequest) -> RenderResult:
        ok, why = self.available()
        if not ok:
            raise ProviderError(why)
        prompt = req.payload.text_prompt()
        r = _post(
            self.URL,
            headers={"xi-api-key": os.environ["ELEVENLABS_API_KEY"],
                     "accept": "audio/mpeg",
                     "content-type": "application/json"},
            json={"prompt": prompt,
                  "music_length_ms": int(max(10_000, min(300_000, req.duration_s * 1000)))},
        )
        return RenderResult(
            audio=r.content,
            mime=r.headers.get("content-type", "audio/mpeg"),
            provider=self.name, prompt=prompt, duration_s=req.duration_s,
            notes=["sin condicionamiento melódico: el modelo no lo soporta"])
