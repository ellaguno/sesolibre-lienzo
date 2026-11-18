"""
Proveedor `musicgen`: MusicGen de Meta, ejecutado en local.

Es el que recomiendo para prototipar: pesos abiertos, sin coste por llamada, sin
dependencia de un tercero, y —lo importante— es el único de los candidatos con
**melody conditioning** real. Eso es lo que convierte el render en "tu jardín
producido" en vez de "una pieza parecida".

Requiere instalación aparte (no está en requirements.txt porque son varios GB):

    pip install torch torchaudio transformers

VRAM aproximada en fp16: `small` ~2.5 GB, `medium` ~5 GB, `melody` ~5 GB.
Una Quadro P1000 (4 GB) llega a `small` y no a `medium`/`melody`, así que para
usar condicionamiento melódico en serio hace falta otra GPU o alquilar una.

Nota: `facebook/musicgen-melody` condiciona con un WAV de referencia, no con MIDI.
Se rinde primero el .mid con el proveedor `local` y ese audio es el que guía al
modelo. Es la forma correcta de hacerlo: el modelo sigue el contorno melódico y
la armonía que el jugador construyó.
"""

from __future__ import annotations

import io
import os

from .base import Provider, ProviderError, RenderRequest, RenderResult

MODEL = os.environ.get("VIVERO_MUSICGEN_MODEL", "facebook/musicgen-melody")


class MusicGenProvider(Provider):
    name = "musicgen"
    supports_melody = True

    def __init__(self):
        self._pipe = None

    def available(self) -> tuple[bool, str]:
        try:
            import torch  # noqa: F401
            import transformers  # noqa: F401
        except ImportError:
            return False, ("faltan torch/transformers. "
                           "pip install torch torchaudio transformers")
        return True, "ok"

    def _load(self):
        if self._pipe is not None:
            return self._pipe
        import torch
        from transformers import AutoProcessor, MusicgenMelodyForConditionalGeneration

        dtype = torch.float16 if torch.cuda.is_available() else torch.float32
        device = "cuda" if torch.cuda.is_available() else "cpu"
        processor = AutoProcessor.from_pretrained(MODEL)
        model = MusicgenMelodyForConditionalGeneration.from_pretrained(
            MODEL, torch_dtype=dtype).to(device)
        self._pipe = (processor, model, device)
        return self._pipe

    def render(self, req: RenderRequest) -> RenderResult:
        ok, why = self.available()
        if not ok:
            raise ProviderError(why)

        import numpy as np
        import torch

        from .. import audio as au

        processor, model, device = self._load()
        prompt = req.payload.text_prompt()

        # Guía melódica: se rinde el jardín en local y se usa como referencia.
        guide, guide_sr = None, None
        try:
            from .local import LocalProvider
            guide_res = LocalProvider(req.sample_pack or None).render(
                RenderRequest(payload=req.payload, duration_s=30.0,
                              sample_pack=req.sample_pack))
            guide, guide_sr = au.read_wav(io.BytesIO(guide_res.audio))
        except Exception as exc:                      # noqa: BLE001
            # Sin guía el modelo sigue funcionando; solo pierde el condicionamiento.
            req.extra["melody_warning"] = f"sin guía melódica: {exc}"

        kwargs = {"text": [prompt], "padding": True, "return_tensors": "pt"}
        if guide is not None:
            target_sr = processor.feature_extractor.sampling_rate
            kwargs["audio"] = au.resample(guide, guide_sr / target_sr)
            kwargs["sampling_rate"] = target_sr

        inputs = processor(**kwargs).to(device)
        # MusicGen genera a 50 tokens/s.
        max_tokens = int(min(req.duration_s, 30.0) * 50)
        torch.manual_seed(req.seed)
        with torch.no_grad():
            wav = model.generate(**inputs, do_sample=True, guidance_scale=3.0,
                                 max_new_tokens=max_tokens)

        out = wav[0, 0].to(torch.float32).cpu().numpy()
        out_sr = model.config.audio_encoder.sampling_rate
        if out_sr != au.SR:
            out = au.resample(out, out_sr / au.SR)
        out = au.fade(au.normalize(np.asarray(out)), in_s=0.05, out_s=1.5)

        notes = [f"modelo {MODEL}", f"{'con' if guide is not None else 'sin'} guía melódica"]
        if "melody_warning" in req.extra:
            notes.append(req.extra["melody_warning"])
        if req.duration_s > 30.0:
            notes.append("MusicGen genera 30 s por pasada; para más hay que "
                         "encadenar con solapamiento (no implementado)")

        return RenderResult(
            audio=au.wav_bytes(out), mime="audio/wav", provider=self.name,
            prompt=prompt, duration_s=len(out) / au.SR, notes=notes)
