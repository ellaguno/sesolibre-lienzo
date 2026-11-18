# lienzo — el código

```
core/   lienzo-core: escena → motivos → director → render. Sin UI. Se prueba headless.
cli/    `lienzo`: demo, describe, render. Para escuchar sin app.
```

## Correr

```bash
cd lienzo && cargo build --release
./target/release/lienzo demo -o ../assets/escenas/demo.json
./target/release/lienzo describe ../assets/escenas/demo.json
./target/release/lienzo render ../assets/escenas/demo.json -o ../assets/audio/render/demo.wav -s 180
```

Necesita el pack curado en `assets/audio/{instruments,ambience,events}`; se genera con
`python3.11 tools/curate_samples.py` a partir del material crudo registrado en
`assets/manifest.toml`.

## Módulos del core

| Módulo | Qué hace |
|---|---|
| `scene` | La obra: trazos (pincel, puntos 3D, duración del gesto). JSON. |
| `brush` | Catálogo de pinceles: instrumento, registro, articulación, sonido natural. Dato. |
| `theory` | Re pentatónica, ciclo de acordes, ajuste al acorde. |
| `motif` | Trazo → motivo: contorno = melodía, longitud = duración, velocidad = dinámica. |
| `variation` | Tema y variaciones: transposición, inversión, retrógrado, aumentación, adelgazamiento, eco, octava. |
| `director` | Forma por secciones, cuándo vuelve cada motivo y con qué variación, colchón, ambiente, eventos naturales. Produce eventos. |
| `render` | Eventos + muestras → audio: remuestreo, envolventes, filtro por profundidad, reverb, limitador. |
| `rng` | Azar determinista sembrado por la obra. |

Regla: **el `core` no conoce a Bevy.** La app lo consulta; el render offline y las pruebas
lo usan directo.
