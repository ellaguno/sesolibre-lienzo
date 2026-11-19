# sesolibre-lienzo

Pintas un paisaje y el paisaje se convierte en su propia banda sonora. Un lienzo 3D con
pinceles semánticos; cada trazo es un elemento visual y una frase musical tocada con
instrumentos y sonidos reales. La obra se contempla, se recorre, se modifica y se comparte.

- [DISEÑO.md](DISEÑO.md) — el diseño completo y las fases.
- [IDEAS.md](IDEAS.md) — estacionamiento de ideas para después.
- `assets/` — sprites, muestras y su registro de autoría (`manifest.toml`).
- `_vivero/` — el intento anterior (Godot + Tone.js + servicio de IA). Referencia
  histórica; no se extiende.

## Estado

**Fase 0 — gusto: cerrada (18 nov 2026).** Estilo *teatro de papel bajo el agua* aprobado
con la imagen [postal_fase0.png](assets/images/postal_fase0.png) y la maqueta de audio
`assets/audio/fase0/abismo_60s.ogg` (generados por `tools/fase0_postal.py` y
`tools/fase0_mock.py`). 18 sprites recortados en `assets/sprites/cut/`.

**Fase 1 — core en Rust:** hecha. `lienzo/core`: escena → motivos → director → motor en
tiempo real / WAV.

**Fase 2 — app de escritorio (Bevy):** hecha. Abismo 3D, pinceles, plano de dibujo, audio
en vivo con el mismo motor, contemplación, galería, export de audio.

**Fase 3 — Android:** corre en un Pixel 9 Pro (GLES, material propio, assets desde el APK,
táctil, 0 cortes de audio, ~700 MB). Pendiente: afinar CPU y memoria, y diseñar la
interfaz táctil definitiva — la actual es andamio técnico.
