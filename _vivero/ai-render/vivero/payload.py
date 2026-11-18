"""
Carga y validación del payload que produce el juego, y construcción del prompt.

El payload es el contrato entre el juego y el render. Godot lo escribe
(`Garden.ai_payload()`), esto lo consume. Si cambia el esquema, se sube
`SCHEMA_MIN` y se rechaza lo viejo con un mensaje claro en vez de renderizar
basura silenciosamente.
"""

from __future__ import annotations

import hashlib
import json
from dataclasses import dataclass, field

from .catalog import CATALOG

SCHEMA_MIN = 1

# El catálogo (objetos, momentos, biomas) se lee de shared/catalog.json, que
# exporta el juego. Aquí solo queda lo que es propio del render: cómo se describe
# cada ambiente EN INGLÉS para el modelo.
TYPES = CATALOG.objects

# Descriptores de bioma para el prompt. Un bioma sin entrada usa el genérico:
# preferimos un prompt algo vago a que el servicio se caiga por un ambiente
# nuevo que aún no se ha descrito.
BIOME_EN = {
    "bosque": {"en": "ambient generative folk",
               "instruments": "nylon-string plucks, marimba, warm low pad",
               "avoid": "no drums, no vocals, no synth lead"},
    "selva": {"en": "humid jungle ambient, organic and layered",
              "instruments": "bamboo flute, hand percussion, bird calls, deep pad",
              "avoid": "no drums, no vocals, no synth lead"},
    "acuario": {"en": "submerged ambient, slow and weightless",
                "instruments": "glass bells, water drops, deep swelling pad",
                "avoid": "no drums, no vocals, no percussion loop"},
    "mar": {"en": "open sea ambient, wide and distant",
            "instruments": "foghorn drone, buoy bells, wave wash, gulls",
            "avoid": "no drums, no vocals, no synth lead"},
    "cielo": {"en": "celestial ambient drone, weightless and vast",
              "instruments": "glass tones, long swells, high shimmer",
              "avoid": "no drums, no vocals, no rhythm"},
    "ciudad": {"en": "urban minimalism, pulsed and mechanical",
               "instruments": "marimba pattern, metallic plucks, low pad",
               "avoid": "no drums, no vocals, no synth lead"},
    "interior": {"en": "intimate room ambient, close and dry",
                 "instruments": "music box, soft electric piano, clock ticks",
                 "avoid": "no drums, no vocals, no reverb wash"},
}
GENERIC_EN = {"en": "ambient generative instrumental",
              "instruments": "soft sustained tones and gentle plucks",
              "avoid": "no drums, no vocals"}

# Solo carácter y luz. La densidad se describe aparte y a partir de la escena,
# así que estos descriptores NO deben hablar de cuánto suena: "night, sparse" y
# "dense continuous tapestry" en el mismo prompt se contradicen.
MOOD_EN = {
    "amanecer": "dawn, hazy and hopeful",
    "dia": "midday, open and clear",
    "atardecer": "dusk, warm and melancholic",
    "noche": "night, cold and still",
}

# El resto del prompt va en inglés; dejar la escala en español metía dos idiomas
# en la misma frase y los modelos de audio responden peor.
SCALE_EN = {
    "D lidio pentatónico": "D lydian pentatonic",
    "D dórico": "D dorian",
    "A menor pentatónica": "A minor pentatonic",
    "G eólico": "G aeolian",
}

LAYER_EN = {
    "pad": "sustained low pad",
    "sub": "deep sub pulse",
    "arpa": "fingerpicked arpeggio",
    "marimba": "wooden marimba",
    "campana": "glassy bells",
    "shimmer": "high shimmer",
    "percusión": "soft wooden percussion",
}


class PayloadError(ValueError):
    pass


@dataclass
class Payload:
    raw: dict
    bioma: str
    momento: str
    bpm: float
    tonica: str
    escala: str
    densidad: float
    objetos: dict
    melodia: list
    celdas: list
    capas_activas: dict
    prompt_texto: str
    registro_medio_midi: int
    pasos_por_ciclo: int = 16
    niveles: int = 5
    transposicion: int = 0
    ciclo_compases: int = 2
    warnings: list = field(default_factory=list)

    # ---- geometría temporal ----

    @property
    def step_seconds(self) -> float:
        """La subdivisión la fija el bioma: 16ª en la ciudad, negra en el cielo."""
        beats = float(self.raw.get("paso_en_pulsos", 0.0)) or CATALOG.step_beats_of(self.bioma)
        return 60.0 / self.bpm * beats

    @property
    def cycle_seconds(self) -> float:
        return self.step_seconds * self.pasos_por_ciclo

    def cycles_for(self, duration_s: float) -> int:
        return max(1, int(round(duration_s / self.cycle_seconds)))

    # ---- identidad ----

    def fingerprint(self) -> str:
        """
        Hash estable de lo que afecta al resultado sonoro. Sirve de clave de
        caché: renderizar dos veces el mismo jardín no debe costar dos veces.
        Se ignoran título y marcas de tiempo a propósito.
        """
        keys = ["bioma", "momento", "bpm", "tonica", "escala", "densidad", "celdas"]
        canon = json.dumps({k: self.raw.get(k) for k in keys},
                           sort_keys=True, separators=(",", ":"))
        return hashlib.sha256(canon.encode()).hexdigest()[:16]

    # ---- prompt ----

    def text_prompt(self) -> str:
        """
        Prompt en inglés para el modelo. El `prompt_texto` que trae el payload
        está en español y es para depurar/mostrar; los modelos de audio responden
        bastante mejor en inglés, así que se reconstruye aquí.
        """
        b = BIOME_EN.get(self.bioma, GENERIC_EN)
        parts = [
            b["en"],
            MOOD_EN.get(self.momento, self.momento),
            f"{self.bpm:.0f} BPM",
            "4/4",
            SCALE_EN.get(self.escala, self.escala),
        ]
        layers = [LAYER_EN[k] for k in self.dominant_layers() if k in LAYER_EN]
        parts.append(", ".join(layers) if layers else b["instruments"])

        if self.densidad < 0.15:
            parts.append("extremely sparse, long silences")
        elif self.densidad < 0.35:
            parts.append("sparse and contemplative")
        else:
            parts.append("dense continuous tapestry")

        if self.capas_activas.get("drone"):
            parts.append("continuous drone underneath")
        if self.capas_activas.get("aire"):
            parts.append("airy high-frequency bed")

        parts += ["wide reverb", "loopable", b["avoid"]]
        return ", ".join(parts)

    def dominant_layers(self) -> list:
        """Familias instrumentales ordenadas por cuántos objetos las usan."""
        fams = {}
        for type_id, count in self.objetos.items():
            fam = _FAMILY.get(type_id)
            if fam:
                fams[fam] = fams.get(fam, 0) + int(count)
        return [k for k, _ in sorted(fams.items(), key=lambda kv: -kv[1])][:4]

    def negative_prompt(self) -> str:
        return "drums, percussion loop, vocals, speech, applause, distortion, clipping"


_FAMILY = {
    "arbol": "pad", "hongo": "sub", "helecho": "arpa", "arbusto": "marimba",
    "flor": "campana", "luciernaga": "shimmer", "piedra": "percusión",
}


def _need(d: dict, key: str, kind):
    if key not in d:
        raise PayloadError(f"falta el campo obligatorio '{key}'")
    v = d[key]
    if not isinstance(v, kind):
        kinds = kind if isinstance(kind, tuple) else (kind,)
        names = "/".join(k.__name__ for k in kinds)
        raise PayloadError(f"'{key}' debería ser {names}, es {type(v).__name__}")
    return v


def load(data: dict) -> Payload:
    """Valida y normaliza. Lanza PayloadError con un mensaje utilizable."""
    if not isinstance(data, dict):
        raise PayloadError("el payload debe ser un objeto JSON")

    version = int(data.get("version", SCHEMA_MIN))
    if version < SCHEMA_MIN:
        raise PayloadError(
            f"payload de esquema v{version}; este servicio requiere v{SCHEMA_MIN} o superior")

    bpm = float(_need(data, "bpm", (int, float)))
    if not 20.0 <= bpm <= 300.0:
        raise PayloadError(f"bpm fuera de rango: {bpm}")

    bioma = str(data.get("bioma", "bosque"))
    # Los límites los da el bioma, no una constante: no todos los ambientes
    # tienen por qué repartir el ciclo igual.
    n_levels = int(data.get("niveles", CATALOG.levels(bioma)))
    n_steps = int(data.get("pasos_por_ciclo", CATALOG.steps(bioma)))

    celdas = _need(data, "celdas", list)
    warnings = []
    clean_cells = []
    unknown = set()
    for c in celdas:
        if not isinstance(c, dict):
            warnings.append("celda no-objeto ignorada")
            continue
        type_id = str(c.get("tipo", ""))
        if not CATALOG.has_object(type_id):
            unknown.add(type_id)
            continue
        # v1 usaba anillo/sector; v2 usa nivel/paso. Se aceptan ambos.
        level = int(c.get("nivel", c.get("anillo", -1)))
        step = int(c.get("paso", c.get("sector", -1)))
        if not (0 <= level < n_levels and 0 <= step < n_steps):
            warnings.append(f"celda fuera de rango ({level},{step}) ignorada")
            continue
        clean_cells.append({"nivel": level, "paso": step, "tipo": type_id})

    if unknown:
        warnings.append("tipos desconocidos ignorados: " + ", ".join(sorted(unknown)))

    if not clean_cells:
        extra = ""
        if unknown:
            # Este era EL fallo silencioso: el juego exporta bien y el servicio
            # rechaza todo porque su catálogo está viejo. Ahora lo dice.
            extra = (" Ninguno de los tipos usados existe en el catálogo (%s), "
                     "que se leyó de %s. Vuelve a exportarlo: godot --headless "
                     "--path godot-mvp --script tools/export_catalog.gd"
                     % (", ".join(sorted(unknown)), CATALOG.source))
        raise PayloadError(
            "el payload no tiene ninguna celda válida: nada que renderizar." + extra)

    return Payload(
        raw=data,
        bioma=bioma,
        momento=str(data.get("momento", "atardecer")),
        bpm=bpm,
        tonica=str(data.get("tonica", "?")),
        escala=str(data.get("escala", "")),
        densidad=float(data.get("densidad", 0.0)),
        objetos=dict(data.get("objetos", {})),
        melodia=list(data.get("melodia", [])),
        celdas=clean_cells,
        capas_activas=dict(data.get("capas_activas", {})),
        prompt_texto=str(data.get("prompt_texto", "")),
        registro_medio_midi=int(data.get("registro_medio_midi", 0)),
        pasos_por_ciclo=n_steps,
        niveles=n_levels,
        transposicion=int(data.get("transposicion", 0)),
        ciclo_compases=int(data.get("ciclo_compases", 2)),
        warnings=warnings,
    )


def load_file(path: str) -> Payload:
    with open(path, "r", encoding="utf-8") as f:
        return load(json.load(f))
