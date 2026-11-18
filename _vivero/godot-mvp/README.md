# Vivero — MVP paso 1 (Godot 4)

Diorama musical en 3D: colocar objetos **es** componer. Un barrido gira sobre la isla y
hace sonar lo que toca.

**Siete ambientes, cuatro layouts.** El disco circular es *una opción*, no la forma del
juego. Añadir un ambiente es una entrada de diccionario en `Biomes`, no código.

| Bioma | Layout | Paso (tiempo) | Nivel (altura tonal) | Barrido |
|---|---|---|---|---|
| Bosque, Selva | `radial` — disco | ángulo | anillo | cuña giratoria |
| Acuario, Interior | `wall` — pared vertical | X | Y (los objetos flotan) | barra vertical |
| Mar, Ciudad | `field` — retícula en suelo | X | Z (crecen en Y) | barra que cruza |
| Cielo | `dome` — cúpula | azimut | elevación | meridiano giratorio |

40 objetos, 24 muestras, 22 formas procedurales. Las paletas no se solapan: cada ambiente
tiene su propio recorrido de grave a agudo.

## Requisitos y arranque

1. **Godot 4.7** o superior (`godotengine.org/download`). El proyecto usa el renderer
   *Forward Mobile* con Vulkan.
2. Generar el sample pack (una vez):

   ```bash
   cd godot-mvp
   python3 tools/gen_samples.py      # requiere numpy
   ```

3. Abrir `godot-mvp/project.godot` en Godot y darle a *Play* (F5).

Desde consola:

```bash
godot --path godot-mvp
```

La ventana arranca en retrato (540×1080 lógicos). Usa **audífonos**: el pad y el sub no
existen en bocina de laptop y son la mitad del efecto.

### Controles

| Acción | Ratón / teclado | Táctil |
|---|---|---|
| Plantar / quitar | click sobre una celda | toque |
| Orbitar | arrastrar | arrastrar un dedo |
| Zoom | rueda | pinza |
| Play / pausa | `espacio` | botón ▶ |
| Herramienta | `1`–`7` | paleta inferior (cambia con el bioma) |
| Apreciación | `A` / `Esc` | botón |
| Guardar / postal | `S` / `P` | botones |

Un gesto cuenta como toque solo si se movió menos de 14 px y duró menos de 0.45 s; en
cualquier otro caso es una órbita. Sin eso, orbitar plantaría árboles sin querer.

## El mapeo

Todo el mapeo vive en un único archivo: **`src/core/mapping.gd`**. Si cambia, se cambia
ahí y en ningún otro sitio.

| Dimensión visual | Dimensión musical |
|---|---|
| Paso (16) | Posición en el ciclo. 16 corcheas = 2 compases ≈ 7 s a 68 BPM |
| Nivel (5) | Grado de la escala: grave abajo/dentro, agudo arriba/fuera |
| Tipo de objeto | Timbre + octava base. La masa visual determina el registro |
| Densidad total | Filtro maestro + capas: drone (6+), pulso (14+), aire (24+) |
| Momento del día | Escala, tónica, tempo, luz, color del suelo |
| **Posición de cámara** | Perspectiva de escucha: filtro, reverb y nivel |
| **Tono** (control aparte) | Transposición global, ±12 semitonos |

La paleta de cada bioma cubre el registro completo de grave a agudo. En el bosque:
`hongo` (sub) → `árbol` (pad) → `arbusto`/`helecho` (medios) → `flor` (campana) →
`luciérnaga` (shimmer), más `piedra` (percusión, sin altura).

### La cámara es la posición de escucha

Mover la escena cambia el sonido, pero **no la composición**. Mirar desde arriba y desde
lejos es una vista de conjunto: se oye más apagado, más reverberado y algo más bajo.
Acercarse a ras deja el sonido seco y presente. Girar cambia el paneo, porque cada objeto
suena desde su posición real.

Deliberadamente **no** se ató el arrastre a la octava. Si arrastrar transpusiera, no se
podría reencuadrar la vista sin cambiar la pieza — y la transposición es una decisión de
composición, que se guarda con el jardín y debe ser estable. Por eso el tono tiene su
propio control (`Tono`, tercera fila) y la cámara solo mueve la perspectiva.

### Por qué no cansa

La primera versión se repetía **literalmente cada ciclo** — pico de autocorrelación
en 1.00 ciclos. Cualquier bucle de 7 s sin evolución cansa en menos de un minuto, dé
igual el material. Ahora hay tres capas de variación, todas **deterministas** (el
render offline reproduce exactamente lo que se oyó en vivo):

1. **Periodo por celda** — no todo suena en cada vuelta. Periodos coprimos.
2. **Arco de densidad** — la textura se abre y se cierra a lo largo de ~105 s.
3. **Deriva de intensidad** — cada celda con su propio ciclo lento de dinámica.

Medido: el patrón de disparos no se repite en 400 ciclos en ningún bioma — de
7 segundos a más de 17 minutos.

**Periodo y arco se miden en SEGUNDOS, no en ciclos.** Con ciclos de 32 s, un
periodo de 5 vueltas dejaba un objeto callado 160 s y el arco tardaba 13 minutos
en completarse: el cielo daba 4 notas por minuto, que es silencio, no ambient.

### Por qué cada ambiente suena distinto

No basta con cambiar timbres. Los siete compartían escala, tónica, tempo, longitud
de ciclo, subdivisión y las mismas dos capas de fondo — la distancia entre sus
perfiles espectrales bajaba a 0.06. Cada bioma tiene ahora su propia gramática
(`Biomes.MUSIC`): subdivisión, multiplicador de tempo, escala, desplazamiento de
tónica, carácter y **sus propias capas de drone y aire**.

| Bioma | Ciclo | Subdivisión | Carácter | Notas/min |
|---|---|---|---|---|
| Ciudad | 2.6 s | semicorchea | pulsed | 168 |
| Selva | 4.6 s | corchea | pulsed | 78 |
| Bosque | 7.1 s | corchea | natural | 77 |
| Interior | 7.8 s | corchea | natural | 57 |
| Mar | 17.6 s | negra | sparse | 17 |
| Acuario | 18.8 s | negra | floating | 21 |
| Cielo | 23.5 s | negra | floating | 19 |

Distancia media entre biomas: **0.33** (era 0.18); mínima **0.15** (era 0.06).

### Por qué no puede sonar mal

1. Todas las notas salen de **una sola escala** por momento del día.
2. Todo está **cuantizado** al grid de corcheas.
3. El sub-bajo (`hongo`) está restringido a **tónica y quinta**: nunca ensucia el fondo
   armónico.
4. Cada tipo tiene su propia duración, así que las capas se solapan por diseño.

La densidad es la única variable de calidad: más objetos = más tapiz, menos = más aire.

## Arquitectura

```
src/core/biomes.gd       CATÁLOGO. Objetos, momentos y ambientes: todo dato.
src/core/mapping.gd      Lo puramente musical: notas, escalas, humanización.
src/core/conductor.gd    Reloj musical (autoload). Emite step_advanced.
src/core/sound_bank.gd   Muestras, buses, capas por densidad, perspectiva.
src/core/garden.gd       Estado + persistencia + payload de IA + .mid.
src/core/ai_render.gd    Cliente del servicio de render (autoload).
src/world/layouts.gd     GEOMETRÍA. Radial, Wall y Field como estrategias.
src/world/mesh_lib.gd    Formas procedurales, indexadas por `shape`.
src/world/diorama.gd     Espejo 3D del Garden. Delega geometría en el layout.
src/world/plant.gd       Malla + voz de un objeto.
src/world/sweep.gd       Cuña giratoria o barra, según el layout.
src/world/orbit_camera.gd Órbita acotada por el layout + raycast a su plano.
src/ui/main_ui.gd        UI construida por código.
src/main.gd              Punto de entrada, entrada de usuario, persistencia.
tools/gen_samples.py     Generador del sample pack placeholder.
```

### Cómo añadir un ambiente

1. Objetos nuevos en `Biomes.OBJECTS` (timbre, octava, forma, color).
2. Formas nuevas en `MeshLib.build()`.
3. Muestras nuevas en `tools/gen_samples.py`.
4. Una entrada en `Biomes.BIOMES` diciendo qué layout usa y qué objetos ofrece.

Nada de eso toca el secuenciador, la cámara, la UI ni la persistencia.

**Regla dura al añadir objetos:** la `ref` de la muestra debe caer en el centro del rango
de uso, que es `~63 + 12*oct`. Fuera de ahí hay que remuestrear más de una octava y el
timbre se deshace — es exactamente el error que cometí al copiar las referencias del
bosque al acuario, y lo caza la prueba de rango descrita abajo.

Regla estructural: **el `Garden` es la verdad, el `Diorama` solo la representa.** Un jardín
se guarda como un diccionario de celdas (`{anillo, sector, tipo}`), no como una lista de
notas — la música se deriva en cada reproducción. Eso mantiene los guardados diminutos,
hace el undo trivial y permite cambiar el mapeo o los samples sin invalidar lo que el
jugador ya creó. `Garden.from_dict()` ignora tipos desconocidos, así que retirar un objeto
en una versión futura no rompe los guardados viejos.

Sin assets externos: las mallas son primitivas compuestas por código y las muestras se
sintetizan con Python.

## Qué está implementado y qué no

Implementado: colocación y borrado, cámara orbital con toque/pinza, secuenciador
cuantizado, 7 objetos, 4 momentos del día, capas por densidad, modo apreciación con órbita
automática, galería (guardar / listar / cargar en `user://jardines/`), export de **postal
PNG**, y export del **payload de IA** (JSON + `.mid`).

**No implementado (y era parte del alcance del paso 1): export de vídeo de 15 s.** Godot
no puede grabar vídeo desde dentro de la app; su Movie Maker es un modo de línea de
comandos que renderiza fuera de tiempo real:

```bash
godot --path godot-mvp --write-movie salida.avi --fixed-fps 60
```

Eso graba la sesión completa, no un clip de 15 s del modo apreciación, y produce AVI sin
comprimir (gigabytes por minuto) que hay que pasar por ffmpeg. Un export de vídeo *dentro*
de la app requiere capturar el viewport frame a frame a un encoder — trabajo real, no un
ajuste. La postal PNG cubre el caso de compartir mientras tanto.

## Dos decisiones de geometría que costaron una prueba cada una

**El cielo se mira desde dentro.** Con la cámara fuera de la cúpula, la mitad cercana
ocluye a la lejana: solo 25 de 80 celdas resultaban alcanzables con un toque. Metiendo la
cámara dentro de la esfera, cada rayo la corta exactamente una vez, no hay oclusión posible
y todas las celdas son alcanzables orbitando — y de paso es lo que de verdad significa
mirar el cielo. `dist_max` queda limitado por debajo del radio para que la cámara no se
salga.

**El picking lo decide el layout, no el juego.** Era un `Plane` fijo; una cúpula no cabía.
Ahora `Layouts.Base.pick(from, dir)` recibe el rayo y cada layout lo resuelve contra lo que
corresponda: plano horizontal, plano vertical o esfera.

## Limitaciones conocidas

**Precisión de los disparos.** El `Conductor` resuelve los pasos en `_process`, así que su
precisión está atada al frame (~16 ms a 60 fps). Godot no ofrece "reproduce este sample en
el instante T" desde el hilo de audio. Por eso el mapeo **no** humaniza el tiempo (un
desfase de 6 ms no sobrevive a una cuantización de 16 ms); en su lugar varía la intensidad
por celda, que sí es audible. En móvil de gama baja la latencia de salida de Android suma
otros 40-80 ms. **Es el riesgo técnico número uno del proyecto** y este MVP no lo resuelve:
la solución es FMOD o Wwise, y hay que evaluarla antes de comprometerse con más biomas.

**Glow en el renderer de compatibilidad.** En OpenGL3 el glow ignora `glow_hdr_threshold` y
multiplica toda la imagen (~5x), lo que lava el fondo a pastel. En Vulkan/Forward Mobile —el
renderer configurado— se comporta correctamente. Si en algún momento se añade un fallback
de compatibilidad para Android antiguo, **hay que desactivar el glow ahí**.

**`keep_aspect` de la cámara.** Está en `KEEP_WIDTH` a propósito. Con el `KEEP_HEIGHT` por
defecto, `fov` se aplica al eje vertical y en una pantalla 9:18 el campo horizontal se
estrecha tanto que la isla se sale del encuadre.

**Avisos de fuga al cerrar.** Godot reporta ~10 `ObjectDB instances leaked` /
5 `resources still in use` al salir, todos `AudioStreamWAV` y `AudioStreamPlaybackWAV`. El
servidor de audio retiene los playbacks activos más allá de la comprobación de la caché de
recursos. **No acumula:** el conteo es idéntico tras 2 s y tras 20 s de ejecución, y está
acotado por el número de samples, no por el tiempo. `SoundBank._exit_tree()` para las capas
en loop antes de soltar el diccionario, que es lo que sí se puede controlar desde el juego.

**Las muestras son placeholders.** Sintetizadas para validar la estructura. El pack real lo
graba un compositor, capa contra capa, en la misma tonalidad y tempo. Ese trabajo es el 60%
de la calidad percibida y esto no lo sustituye — ver `tools/gen_samples.py`.

## Handoff a las fases siguientes

El botón **Payload IA** escribe dos archivos en `user://payloads/`:

- **`payload-*.json`** — el prompt estructurado derivado de la escena real (bioma, momento,
  BPM, escala, densidad, conteo de objetos, capas activas, `prompt_texto`) más la melodía
  del ciclo nota a nota.
- **`melodia-*.mid`** — Standard MIDI File formato 0, el ciclo repetido 4 veces, para usar
  como *melody conditioning*. La IA no inventa la pieza: produce la que el jugador
  construyó.

La percusión se excluye del `.mid` a propósito: condicionar melodía con eventos sin altura
degrada el resultado del modelo. Va en el JSON como capa.

Para la **fase 3 (estilizador visual)** el camino es exportar el depth map de esta misma
escena (`SubViewport` con un shader de profundidad) y fijar con él la estructura del
img2img. Así la composición sigue siendo del jugador y la IA solo cambia el material.

## Verificación

Comprobado con Godot 4.7.1 headless y bajo Xvfb con GL/Vulkan software (llvmpipe/lavapipe):
carga sin errores de script, los tres estados visuales (construcción, apreciación, noche
densa) renderizan, las capas por densidad se activan en sus umbrales, y el ciclo completo
de persistencia (guardar → vaciar → cargar) restaura celdas, momento y BPM.

**No verificado: el audio.** No hay dispositivo de sonido en el entorno de verificación, así
que las rutas de reproducción se ejercitaron pero nadie las ha oído. Eso lo tienes que
hacer tú, con audífonos, y es justo la pregunta que el MVP existe para responder.
