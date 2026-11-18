"""
Proveedor `local`: renderiza el payload con el sample pack del juego.

NO es un mock. Cumple dos funciones reales:

1. Hace el pipeline entero probable sin credenciales, sin GPU y sin red.
2. Es el **fallback de producción**. Si la llamada a la IA falla o expira, el
   jugador recibe igualmente su pieza — la misma que ya estaba oyendo, pero
   mezclada, con reverb de convolución y con cola en vez de cortada.

Es también la referencia contra la que comparar: si el render de IA no suena
mejor que esto, no merece el coste por llamada.
"""

from __future__ import annotations

import os

import numpy as np

from .. import audio
from ..catalog import CATALOG
from .base import Provider, RenderRequest, RenderResult

DEFAULT_PACK = os.path.join(
    os.path.dirname(__file__), "..", "..", "..", "godot-mvp", "assets", "audio")

# Anillo -> posición estéreo y nivel. El anillo exterior suena más abierto y algo
# más lejano; reproduce la sensación espacial del diorama.
RING_PAN = [0.0, -0.35, 0.35, -0.62, 0.62]
RING_DB = [0.0, -0.6, -0.6, -1.4, -1.4]


class LocalProvider(Provider):
    name = "local"
    supports_melody = True

    def __init__(self, pack_dir: str | None = None):
        self.pack_dir = os.path.abspath(pack_dir or DEFAULT_PACK)
        self._cache: dict[str, np.ndarray] = {}

    def available(self) -> tuple[bool, str]:
        if not os.path.isdir(self.pack_dir):
            return False, f"no existe el sample pack en {self.pack_dir}"
        missing = [
            f"{s}.wav" for s in
            CATALOG.samples()
            | {str(m.get("drone", "")) for m in CATALOG.music.values()}
            | {str(m.get("air", "")) for m in CATALOG.music.values()}
            | {"drone_loop", "air_loop"}
            if not os.path.isfile(os.path.join(self.pack_dir, f"{s}.wav"))
        ]
        if missing:
            return False, ("faltan muestras (%s). Corre: python3 tools/gen_samples.py"
                           % ", ".join(sorted(missing)[:4]))
        return True, "ok"

    def _sample(self, name: str) -> np.ndarray:
        if name not in self._cache:
            x, sr = audio.read_wav(os.path.join(self.pack_dir, f"{name}.wav"))
            if sr != audio.SR:
                x = audio.resample(x, sr / audio.SR)
            self._cache[name] = x
        return self._cache[name]

    def render(self, req: RenderRequest) -> RenderResult:
        p = req.payload
        ok, why = self.available()
        if not ok:
            from .base import ProviderError
            raise ProviderError(why)

        cycles = p.cycles_for(req.duration_s)
        step = p.step_seconds
        cycle_len = p.cycle_seconds
        # Cola extra para que la última nota del último ciclo no se corte.
        total_s = cycles * cycle_len + 6.0
        n = int(total_s * audio.SR)
        bus = np.zeros((n, 2), dtype=np.float32)

        placed = 0
        events = 0
        for cell in p.celdas:
            type_id = cell["tipo"]
            spec = CATALOG.objects[type_id]
            ring, slot = cell["nivel"], cell["paso"]
            midi = CATALOG.midi_of(p.bioma, type_id, ring, p.momento, p.transposicion)
            factor = CATALOG.pitch_scale(type_id, midi, ring)
            voice = audio.resample(self._sample(spec["sample"]), factor)

            # Misma humanización determinista que en el juego, para que el render
            # tenga el mismo relieve dinámico que se oyó al construir.
            base_db = (float(spec["db"])
                       + ((ring * 7 + slot * 13) % 13 - 6) * 0.25
                       + RING_DB[ring % len(RING_DB)])
            stereo = audio.pan(voice, RING_PAN[ring % len(RING_PAN)])

            for c in range(cycles):
                # MISMA variación que el juego. Sin esto el render sonaría a
                # bucle plano mientras la sesión en vivo evoluciona.
                if not CATALOG.should_trigger(p.bioma, type_id, ring, slot, c,
                                              cycle_len):
                    continue
                vel = CATALOG.velocity_scale(ring, slot, c, cycle_len)
                at = int((c * cycle_len + slot * step) * audio.SR)
                _add_stereo(bus, stereo, at, audio.to_db(base_db) * vel)
                events += 1
            placed += 1

        self._add_layers(bus, p, cycles, cycle_len)

        bus = audio.normalize(bus, 0.7)
        # La reverb la fija el bioma además del momento: un interior es seco y
        # un cielo abierto no tiene nada que refleje.
        bus = audio.reverb(bus, audio.make_ir(2.8, damping=0.30),
                           wet=CATALOG.reverb(p.bioma, p.momento))
        bus = audio.fade(audio.normalize(bus, 0.92), in_s=0.03, out_s=3.2)

        return RenderResult(
            audio=audio.wav_bytes(bus),
            mime="audio/wav",
            provider=self.name,
            prompt=p.text_prompt(),
            duration_s=len(bus) / audio.SR,
            notes=[f"{placed} objetos, {cycles} ciclos, {events} disparos "
                   f"({events / max(1, cycles):.1f}/ciclo)", *p.warnings],
        )

    def _add_layers(self, bus: np.ndarray, p, cycles: int, cycle_len: float) -> None:
        """Drone y aire: las capas que el juego activa por densidad."""
        total_s = cycles * cycle_len
        # Cada bioma trae SUS capas de fondo. Usar las del bosque en los siete era
        # lo que más los igualaba: el fondo continuo es lo que más se oye.
        mus = CATALOG.music_of(p.bioma)
        if p.capas_activas.get("drone"):
            root = CATALOG.root_of(p.bioma, p.momento) - 24
            loop = audio.resample(self._sample(str(mus.get("drone", "drone_loop"))),
                                  2.0 ** ((root - 33) / 12.0))
            _tile(bus, audio.pan(loop, 0.0), total_s, audio.to_db(-18.0))
        if p.capas_activas.get("aire"):
            loop = self._sample(str(mus.get("air", "air_loop")))
            _tile(bus, audio.pan(loop, 0.0), total_s, audio.to_db(-21.0))
        if p.capas_activas.get("pulso"):
            root = CATALOG.root_of(p.bioma, p.momento) - 24
            hit = audio.resample(self._sample("sub"), 2.0 ** ((root - 36) / 12.0))
            stereo = audio.pan(hit, 0.0)
            beat = 60.0 / p.bpm * 2.0        # una vez por blanca
            t = 0.0
            while t < total_s:
                _add_stereo(bus, stereo, int(t * audio.SR), audio.to_db(-16.0))
                t += beat


def _tile(bus: np.ndarray, loop: np.ndarray, total_s: float, gain: float) -> None:
    at = 0
    limit = int(total_s * audio.SR)
    while at < limit:
        _add_stereo(bus, loop, at, gain)
        at += len(loop)


def _add_stereo(bus: np.ndarray, src: np.ndarray, at: int, gain: float) -> None:
    if at >= len(bus):
        return
    m = min(len(src), len(bus) - at)
    if m > 0:
        bus[at:at + m] += src[:m] * gain


