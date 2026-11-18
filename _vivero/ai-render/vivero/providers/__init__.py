"""
Registro de proveedores y política de fallback.

Elegir proveedor es una variable de entorno (`VIVERO_PROVIDER`), no un cambio de
código: la decisión sigue abierta y este servicio no debería forzarla.
"""

from __future__ import annotations

import os

from .base import Provider, ProviderError, RenderRequest, RenderResult
from .http_api import ElevenLabsMusicProvider, StabilityAudioProvider
from .local import LocalProvider
from .musicgen import MusicGenProvider

__all__ = ["Provider", "ProviderError", "RenderRequest", "RenderResult",
           "get", "render", "status"]

_CLASSES = {
    "local": LocalProvider,
    "musicgen": MusicGenProvider,
    "stability": StabilityAudioProvider,
    "elevenlabs": ElevenLabsMusicProvider,
}

_instances: dict[str, Provider] = {}

DEFAULT = os.environ.get("VIVERO_PROVIDER", "local")


def get(name: str | None = None) -> Provider:
    key = (name or DEFAULT).strip().lower()
    if key not in _CLASSES:
        raise ProviderError(
            f"proveedor desconocido '{key}'. Disponibles: {', '.join(_CLASSES)}")
    if key not in _instances:
        _instances[key] = _CLASSES[key]()
    return _instances[key]


def status() -> list[dict]:
    """Para el endpoint de salud y la CLI: qué se puede usar ahora mismo."""
    out = []
    for name in _CLASSES:
        p = get(name)
        ok, why = p.available()
        out.append({"proveedor": name, "disponible": ok, "motivo": why,
                    "melodia": p.supports_melody,
                    "por_defecto": name == DEFAULT})
    return out


def render(req: RenderRequest, name: str | None = None,
           allow_fallback: bool = True) -> RenderResult:
    """
    Rinde con el proveedor pedido. Si falla y `allow_fallback`, cae a `local`.

    La política de fallback es de producto, no de comodidad: el jugador acabó de
    construir algo y espera oírlo. Devolverle un error en vez de su pieza —cuando
    tenemos con qué renderizarla— es la peor opción posible.
    """
    chosen = (name or DEFAULT).strip().lower()
    try:
        return render_strict(req, chosen)
    except (ProviderError, Exception) as exc:         # noqa: BLE001
        if not allow_fallback or chosen == "local":
            raise
        result = render_strict(req, "local")
        result.fallback = True
        result.notes.insert(0, f"fallback: '{chosen}' falló ({exc})")
        return result


def render_strict(req: RenderRequest, name: str | None = None) -> RenderResult:
    """Sin fallback: útil para comparar proveedores a oído."""
    p = get(name)
    ok, why = p.available()
    if not ok:
        raise ProviderError(f"{p.name} no disponible: {why}")
    return p.render(req)
