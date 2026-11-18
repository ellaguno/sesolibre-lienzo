"""
Catálogo compartido: objetos, momentos y biomas.

**Única fuente de verdad: `godot-mvp/src/core/biomes.gd`.** Este módulo lee el
JSON que exporta `godot-mvp/tools/export_catalog.gd`.

Antes estas tablas estaban duplicadas a mano aquí. Al añadir cuatro biomas la
copia quedó obsoleta de inmediato y el servicio empezó a rechazar sus payloads
con "ninguna celda válida" — un fallo silencioso desde fuera, porque el juego
exportaba bien y el error aparecía al otro lado. El archivo generado convierte
esa divergencia en imposible.

Si el catálogo falta, se usa un mínimo de emergencia con el bosque para que el
servicio arranque y diga qué pasa, en vez de reventar al importar.
"""

from __future__ import annotations

import json
import os

CATALOG_ENV = "VIVERO_CATALOG"
DEFAULT_PATH = os.path.join(
    os.path.dirname(os.path.abspath(__file__)), "..", "..", "shared", "catalog.json")

# Mínimo de emergencia: solo lo necesario para arrancar y reportar el problema.
_FALLBACK = {
    "version": 0,
    "paso_en_pulsos": 0.5,
    "capas": {"drone_frac": 0.05, "aire_frac": 0.09, "pulso_frac": 0.28,
              "drone_min": 4, "aire_min": 6, "pulso_min": 12},
    "objetos": {
        "arbol": {"sample": "pad", "ref": 51, "dur": 8, "db": -10.0,
                  "pitched": True, "oct": -1, "sub": "pad · grave"},
    },
    "momentos": {
        "atardecer": {"nm": "Atardecer", "root": 57, "deg": [0, 3, 5, 7, 10],
                      "scale_nm": "A menor pentatónica", "bpm": 68, "reverb": 0.46},
    },
    "orden_momentos": ["atardecer"],
    "biomas": {
        "bosque": {"nm": "Bosque", "layout": "radial", "levels": 5, "steps": 16,
                   "objects": ["arbol"], "reverb_bonus": 0.0},
    },
    "orden_biomas": ["bosque"],
}


class Catalog:
    def __init__(self, data: dict, source: str, stale: bool = False):
        self.data = data
        self.source = source
        self.stale = stale
        self.objects: dict = data["objetos"]
        self.moods: dict = data["momentos"]
        self.biomes: dict = data["biomas"]
        self.step_beats: float = float(data.get("paso_en_pulsos", 0.5))
        self.layers: dict = data.get("capas", {})
        self.music: dict = data.get("musica", {})
        self.var_cfg: dict = data.get("variacion", {})

    # ---- consultas ----

    def has_object(self, type_id: str) -> bool:
        return type_id in self.objects

    def biome(self, biome_id: str) -> dict:
        return self.biomes.get(biome_id) or next(iter(self.biomes.values()))

    def levels(self, biome_id: str) -> int:
        return int(self.biome(biome_id).get("levels", 5))

    def steps(self, biome_id: str) -> int:
        return int(self.biome(biome_id).get("steps", 16))

    ## Umbrales de capa para una rejilla dada. Relativos, no fijos.
    def layer_thresholds(self, total_cells: int) -> dict:
        c = self.layers
        return {
            "drone": max(int(c.get("drone_min", 4)),
                         round(float(c.get("drone_frac", 0.05)) * total_cells)),
            "aire": max(int(c.get("aire_min", 6)),
                        round(float(c.get("aire_frac", 0.09)) * total_cells)),
            "pulso": max(int(c.get("pulso_min", 12)),
                         round(float(c.get("pulso_frac", 0.28)) * total_cells)),
        }

    def active_layers(self, biome_id: str, count: int) -> dict:
        th = self.layer_thresholds(self.levels(biome_id) * self.steps(biome_id))
        return {k: count >= v for k, v in th.items()}

    def reverb(self, biome_id: str, mood_key: str) -> float:
        mood = self.moods.get(mood_key, {})
        base = float(mood.get("reverb", 0.45))
        bonus = float(self.biome(biome_id).get("reverb_bonus", 0.0))
        return max(0.05, min(0.95, base + bonus))

    # ---- gramática del bioma ----

    def music_of(self, biome_id: str) -> dict:
        return self.music.get(biome_id) or next(
            iter(self.music.values()), {"step_beats": 0.5, "bpm_scale": 1.0,
                                        "scale": [], "root_shift": 0,
                                        "feel": "natural"})

    def step_beats_of(self, biome_id: str) -> float:
        return float(self.music_of(biome_id).get("step_beats", 0.5))

    def bpm_for(self, biome_id: str, mood_key: str) -> float:
        mood = self.moods.get(mood_key) or next(iter(self.moods.values()))
        return float(mood["bpm"]) * float(self.music_of(biome_id).get("bpm_scale", 1.0))

    def degrees_of(self, biome_id: str, mood_key: str) -> list:
        own = self.music_of(biome_id).get("scale") or []
        if own:
            return list(own)
        mood = self.moods.get(mood_key) or next(iter(self.moods.values()))
        return list(mood["deg"])

    def root_of(self, biome_id: str, mood_key: str, transpose: int = 0) -> int:
        mood = self.moods.get(mood_key) or next(iter(self.moods.values()))
        return (int(mood["root"])
                + int(self.music_of(biome_id).get("root_shift", 0)) + transpose)

    ## Reimplementa `Mapping.midi_of`. Sigue duplicada a propósito: es una
    ## fórmula de cuatro términos, no una tabla que crezca.
    def midi_of(self, biome_id: str, type_id: str, level: int, mood_key: str,
                transpose: int = 0) -> int:
        spec = self.objects.get(type_id)
        if spec is None or not spec.get("pitched", False):
            return -1
        degrees = spec.get("deg") or self.degrees_of(biome_id, mood_key)
        return (self.root_of(biome_id, mood_key, transpose) + 12 * int(spec["oct"])
                + int(degrees[level % len(degrees)]))

    # ---- variación ----
    #
    # Espejo EXACTO de `Mapping`. Si esto se desvía, el render suena distinto de
    # lo que el jugador oyó al construir, que es justo lo que no puede pasar.
    # El hash es FNV-1a truncado a 31 bits, idéntico al de GDScript.

    @staticmethod
    def hash_of(values) -> int:
        h = 2166136261
        for v in values:
            h = ((h ^ (int(v) & 0xFFFFFFFF)) * 16777619) & 0x7FFFFFFF
        return h

    def trigger_period(self, type_id: str, level: int, step: int,
                       cycle_seconds: float = 7.0) -> int:
        dur = int(self.objects[type_id]["dur"])
        v = self.var_cfg
        if dur >= 8:
            pool = v.get("periodos_largo", [2, 3, 5])
        elif dur >= 4:
            pool = v.get("periodos_medio", [1, 2, 3])
        else:
            pool = v.get("periodos_corto", [1, 2])
        # El intervalo entre repeticiones se acota en TIEMPO, no en vueltas.
        target = float(v.get("repeticion_objetivo_s", 35.0))
        cap = max(1, min(5, round(target / max(0.5, cycle_seconds))))
        allowed = [p for p in pool if int(p) <= cap] or [cap]
        return int(allowed[self.hash_of([level, step, dur]) % len(allowed)])

    def density_arc(self, cycle: int, cycle_seconds: float = 7.0) -> float:
        import math
        n = max(3.0, float(self.var_cfg.get("arco_segundos", 105.0))
                / max(0.5, cycle_seconds))
        return 0.55 + 0.45 * (0.5 - 0.5 * math.cos(2 * math.pi * cycle / n))

    def should_trigger(self, biome_id: str, type_id: str, level: int,
                       step: int, cycle: int, cycle_seconds: float = 7.0) -> bool:
        period = self.trigger_period(type_id, level, step, cycle_seconds)
        phase = self.hash_of([step, level, 7]) % period
        if cycle % period != phase:
            return False
        feel = str(self.music_of(biome_id).get("feel", "natural"))
        probs = self.var_cfg.get("prob_caracter", {})
        p = float(probs.get(feel, 0.8)) * self.density_arc(cycle, cycle_seconds)
        return (self.hash_of([level, step, cycle, 11]) % 1000) / 1000.0 < p

    def velocity_scale(self, level: int, step: int, cycle: int,
                       cycle_seconds: float = 7.0) -> float:
        import math
        n = max(3.0, float(self.var_cfg.get("deriva_segundos", 74.0))
                / max(0.5, cycle_seconds))
        phase = (self.hash_of([level, step, 3]) % 360) / 360.0 * 2 * math.pi
        return 0.72 + 0.28 * (0.5 + 0.5 * math.sin(2 * math.pi * cycle / n + phase))

    def pitch_scale(self, type_id: str, midi: int, level: int) -> float:
        spec = self.objects[type_id]
        if not spec.get("pitched", False):
            return 0.90 + 0.07 * (level % 3)
        return 2.0 ** ((midi - int(spec["ref"])) / 12.0)

    def samples(self) -> set:
        return {str(o["sample"]) for o in self.objects.values()}


def load(path: str | None = None) -> Catalog:
    target = path or os.environ.get(CATALOG_ENV) or DEFAULT_PATH
    target = os.path.abspath(target)
    if not os.path.isfile(target):
        return Catalog(_FALLBACK, target, stale=True)
    try:
        with open(target, "r", encoding="utf-8") as f:
            data = json.load(f)
    except (OSError, json.JSONDecodeError):
        return Catalog(_FALLBACK, target, stale=True)
    for key in ("objetos", "momentos", "biomas"):
        if key not in data:
            return Catalog(_FALLBACK, target, stale=True)
    return Catalog(data, target)


CATALOG = load()
