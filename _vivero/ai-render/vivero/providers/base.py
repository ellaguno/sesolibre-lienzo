"""Contrato de proveedor. Elegir proveedor es configuración, no código."""

from __future__ import annotations

import os
from dataclasses import dataclass, field

from .. import payload as payload_mod


@dataclass
class RenderRequest:
    payload: payload_mod.Payload
    midi: bytes | None = None
    duration_s: float = 150.0
    seed: int = 0
    sample_pack: str = ""
    extra: dict = field(default_factory=dict)


@dataclass
class RenderResult:
    audio: bytes
    mime: str
    provider: str
    prompt: str
    duration_s: float
    fallback: bool = False
    notes: list = field(default_factory=list)


class ProviderError(RuntimeError):
    pass


class Provider:
    name = "base"
    #: Variables de entorno necesarias. Si falta alguna, `available()` lo dice.
    credentials: tuple = ()
    #: True si el modelo acepta condicionamiento melódico con el .mid.
    supports_melody = False

    def available(self) -> tuple[bool, str]:
        missing = [k for k in self.credentials if not os.environ.get(k)]
        if missing:
            return False, "faltan variables de entorno: " + ", ".join(missing)
        return True, "ok"

    def render(self, req: RenderRequest) -> RenderResult:
        raise NotImplementedError
