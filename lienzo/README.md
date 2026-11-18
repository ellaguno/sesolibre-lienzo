# lienzo — el código

```
core/   lienzo-core: escena → motivos → director → render. Sin UI. Se prueba headless.
cli/    `lienzo`: demo, describe, render. Para escuchar sin app.
app/    `lienzo-app`: Bevy. El abismo en 3D, cámara orbital, cartas de papel, audio en vivo.
```

## La app

```bash
cd lienzo && cargo run -p lienzo-app --release            # abre la escena demo
cd lienzo && cargo run -p lienzo-app --release -- obra.json
```

| Acción | Control |
|---|---|
| Dibujar | botón izquierdo (arrastrar = trazo, clic = toque) |
| Orbitar / acercar | botón derecho o central / rueda |
| Pincel | `1`–`8`: Kelp, Coral, Anémona, Cardumen, Medusa, Burbujas, Ballena, Barco |
| Plano de dibujo (pinceles de agua) | `[` más cerca · `]` más lejos |
| Deshacer / vaciar | `Z` / `C` |
| Guardar / nueva / galería | `S` / `N` / `G` (o los botones). Las obras viven en `~/.local/share/lienzo/obras/` (o `$LIENZO_OBRAS`) |
| Título | `T`, escribe, `Enter` |
| Exportar audio | `E` (o el botón de la galería): guarda y renderiza 3 min a WAV junto al JSON, en segundo plano |
| Contemplar | `A` o `Esc`, o el botón de la paleta. Sin paleta, la cámara deriva; tocar un elemento lo hace actuar |

La paleta de pinceles está abajo; el texto de ayuda arriba a la izquierda. Si el archivo
dado como argumento no existe, abre una obra vacía y la guardará ahí. Busca la raíz del
proyecto subiendo desde el directorio actual hasta encontrar `assets/manifest.toml`.

El audio en vivo es **el mismo `Engine` del core** corriendo en el hilo de la tarjeta de
sonido (`app/src/audio.rs`). La app no mezcla nada: le manda escenas y el motor compone y
suena. Lo que se oye en vivo y lo que exporta `lienzo render` es el mismo código.

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

## Android (verificación, Fase 3)

El core y la app compilan y enlazan para `aarch64-linux-android` (NDK 27, API 28 o
superior porque `cpal` usa AAudio):

```bash
rustup target add aarch64-linux-android && cargo install cargo-ndk cargo-apk
export ANDROID_HOME=~/Android/Sdk ANDROID_NDK_HOME=$ANDROID_HOME/ndk/27.1.12297006
cd lienzo && cargo ndk --platform 28 -t arm64-v8a build -p lienzo-app --release
```

En Android la app es una `cdylib` con `#[bevy_main]` (`app/src/lib.rs`) y la actividad
nativa de Bevy. **Pendiente para que corra en un teléfono:** cargar muestras y sprites
desde el APK (hoy se leen del disco buscando `assets/manifest.toml`), comprimir el pack de
audio (167 MB en WAV → Opus/Vorbis), gestos táctiles y empaquetar con `cargo apk`.
