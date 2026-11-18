# Vivero — render de audio (fase 2)

Convierte un jardín en una pieza producida. El juego exporta un **payload
estructurado** + un **`.mid`**; esto los recibe y devuelve audio.

La idea central: **la IA no inventa la pieza, la produce.** El jugador ya compuso
algo al plantar; el modelo lo interpreta con mejor sonido. Por eso el
condicionamiento melódico no es un extra, es el punto.

## Estado: el proveedor está sin decidir a propósito

No has elegido proveedor, así que el servicio es agnóstico: **cambiar de
proveedor es una variable de entorno, no un cambio de código.**

```bash
python3 render.py estado        # qué hay disponible ahora mismo
```

| Proveedor | Melody conditioning | Coste | Licencia | Estado aquí |
|---|---|---|---|---|
| `local` | sí (es el propio jardín) | 0 | tuya | **funciona, probado** |
| `musicgen` | **sí** (vía audio de guía) | 0 tras la GPU | pesos abiertos | escrito, sin probar (falta torch) |
| `stability` | no | por llamada | comercial explícita | escrito, sin probar (falta API key) |
| `elevenlabs` | no | por llamada | comercial explícita | escrito, sin probar (falta API key) |

**Mi recomendación sigue siendo empezar por `musicgen` en local**: es el único
candidato con condicionamiento melódico real, no tiene coste por llamada y no te
ata a un tercero. Suno y Udio dan mejor resultado subjetivo pero tienen litigios
de copyright abiertos y API restringida — construir el producto encima es asumir
un riesgo de dependencia que no controlas.

**Tu GPU no llega.** La Quadro P1000 tiene 4 GB; `musicgen-melody` necesita ~5 GB
en fp16. Llega a `musicgen-small` (sin melodía). Para usar condicionamiento
melódico en serio hace falta otra GPU o alquilar una por horas.

## El proveedor `local` no es un mock

Renderiza el payload con el sample pack del juego: transposición por remuestreo,
paneo por anillo, la misma humanización determinista de intensidad, capas de
densidad y reverb de convolución. Cumple tres funciones reales:

1. Hace el pipeline entero probable **sin credenciales, sin GPU y sin red**.
2. Es el **fallback de producción**. Si la IA falla o expira, el jugador recibe
   igualmente su pieza — la misma que oía al construir, pero mezclada y con cola.
   Devolverle un error cuando tenemos con qué renderizar es la peor opción.
3. Es la **referencia contra la que comparar**. Si el render de IA no suena mejor
   que esto, no merece el coste por llamada. Úsalo con `--strict` para comparar
   proveedores a oído sin que el fallback te engañe.

## Uso

Sin dependencias que instalar más allá de numpy y scipy (ya los tienes):

```bash
python3 render.py estado                              # proveedores disponibles
python3 render.py payload.json --info                 # inspeccionar, sin renderizar
python3 render.py melodia.mid --info                  # inspeccionar un .mid
python3 render.py payload.json -d 150 --mp3           # renderizar
python3 render.py payload.json -p musicgen --strict   # sin fallback
```

Los payloads que exporta el juego están en
`~/.local/share/godot/app_userdata/Vivero/payloads/`.

### Servicio

```bash
python3 server.py                  # http://127.0.0.1:8900
```

| Ruta | Qué hace |
|---|---|
| `GET /health` | estado de los proveedores |
| `POST /render` | `{payload, midi_b64?, duration_s?, seed?, provider?, strict?, force?}` → `202 {job_id, huella, cacheado}` |
| `GET /jobs/<id>` | estado del trabajo |
| `GET /jobs/<id>/audio` | el audio |

**Asíncrono a propósito:** un render de IA tarda 30-90 s, más de lo que puede
esperar una petición HTTP en móvil. El juego encola, guarda el id y vuelve a
preguntar.

**Caché por huella de escena.** La huella es un hash de lo que afecta al
resultado sonoro (bioma, momento, bpm, escala, densidad, celdas) e ignora título
y marcas de tiempo. Renderizar dos veces el mismo jardín no cuesta dos veces —
importante cuando el coste es por llamada.

**Seguridad:** escucha solo en localhost por defecto. `--host 0.0.0.0` lo expone y
**no trae autenticación**: si lo despliegas, pon algo delante.

## Integración con el juego

`../godot-mvp/src/core/ai_render.gd` (autoload `AIRender`) + el botón **Render IA**.
Encola, sondea cada 2 s, descarga, decodifica el WAV y lo reproduce; pausa el
secuenciador en vivo y entra en modo apreciación, porque en ese momento se está
escuchando la pieza terminada, no construyéndola. Al acabar devuelve el control
al secuenciador.

Si el servicio no está levantado, falla en ~14 ms con un mensaje accionable y el
juego sigue sonando en directo como siempre.

Godot no trae decodificador de WAV en tiempo de ejecución, así que
`AIRender.wav_from_bytes()` recorre los chunks RIFF a mano — buscándolos por
nombre, no asumiendo que `fmt ` y `data` sean los dos primeros, porque muchos
codificadores insertan `LIST`/`INFO` en medio.

## Estructura

```
render.py              CLI
server.py              servicio HTTP (solo stdlib)
vivero/payload.py      contrato con el juego, validación y construcción del prompt
vivero/midi.py         lector de SMF (formatos 0 y 1)
vivero/audio.py        E/S de WAV y DSP (remuestreo, reverb de convolución, paneo)
vivero/providers/      local, musicgen, stability, elevenlabs + registro y fallback
```

### El prompt se deriva, no se escribe

El payload trae un `prompt_texto` en español para depurar; el que va al modelo se
reconstruye en inglés en `Payload.text_prompt()` (los modelos de audio responden
bastante mejor en inglés). Los descriptores de momento del día hablan solo de
carácter y luz, **nunca de densidad**: eso se deriva de la escena, y meter las dos
cosas producía prompts que se contradecían (`night, sparse` junto a
`dense continuous tapestry`).

### Catálogo compartido, no duplicado

`../shared/catalog.json` es el contrato: lo genera el juego, lo lee este servicio.

```bash
godot --headless --path godot-mvp --script tools/export_catalog.gd
```

**Hay que volver a exportarlo cada vez que se toca `Biomes.gd`.**

Antes estas tablas estaban duplicadas a mano en Python. Duró exactamente hasta el
siguiente cambio: al añadir cuatro biomas la copia quedó obsoleta al instante y el
servicio empezó a rechazar sus payloads con *"ninguna celda válida"* — un fallo
silencioso visto desde fuera, porque el juego exportaba correctamente y el error
solo aparecía al otro lado. Ahora el mensaje de error nombra los tipos
desconocidos y dice cómo regenerar el catálogo.

Si el archivo falta, `vivero/catalog.py` cae a un mínimo de emergencia con el
bosque para que el servicio arranque y pueda explicar qué pasa, en vez de reventar
al importar.

Lo único que sigue duplicado es `midi_of` / `pitch_scale`: fórmulas de cuatro
términos, no tablas que crezcan.

## Verificación

Probado con payloads reales exportados por el juego (13 objetos atardecer, 43
objetos noche):

- Render end-to-end por CLI y por HTTP de **los siete biomas**, con payloads
  exportados por el juego. Audio sin clipping (pico 0.92), estéreo real
  (correlación L/R 0.41-0.73) y huella espectral distinta por ambiente: el mar
  sale 9% graves / 56% agudos, el acuario 19% / 24%, el interior 14% / 36%.
- **Fidelidad espectral**, con escenas de control: solo árboles y hongos → 62%
  graves / 7% agudos; solo flores y luciérnagas → 18% / 64%. El renderer sigue el
  contenido, no impone un color.
- Todas las rutas de error: cuerpo vacío y JSON roto → 400; payload sin celdas,
  sin bpm, bpm absurdo, midi corrupto, proveedor inexistente, duración fuera de
  rango → 422; rutas y trabajos inexistentes → 404. Todos con mensaje legible.
- Caché, política de fallback (`stability` no disponible → cae a `local` y lo
  marca) y modo `--strict` (falla en vez de caer).
- El `.mid` del juego contra este lector: formato 0, 44 note-on emparejados,
  tempo correcto, 8 compases.
- Cliente de Godot: decodificación de WAV (33.7 s / 44.1 kHz / estéreo), rechazo
  de basura y de entrada vacía, render nuevo, acierto de caché con duración
  correcta, y fallo limpio con el servicio caído.

**No verificado:** los tres proveedores de IA. `musicgen` necesita torch (varios
GB) y una GPU con más VRAM; `stability` y `elevenlabs` necesitan credenciales. Sus
endpoints y nombres de campo están centralizados arriba de cada clase para que
ajustarlos sea una línea, pero **hay que darlos por no probados** hasta que
alguien los corra con llaves reales.

Y como siempre: nadie ha escuchado nada de esto. Yo no tengo dispositivo de audio.
