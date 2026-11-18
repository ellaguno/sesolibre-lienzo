# Vivero — prototipo del núcleo de audio

Prototipo desechable, en web, del mapeo **espacio → música**. Existe para responder una
sola pregunta antes de escribir una línea de Godot:

> ¿Colocar objetos se siente bien y el resultado suena a música (no a juguete)?

Si la respuesta es no, el problema es el mapeo o los timbres, y ningún motor 3D ni modelo
de IA lo arregla. No mide rendimiento, ni assets, ni monetización.

## Cómo correrlo

Abre `index.html` en Chrome, Safari o Firefox. Tone.js está vendorizado en `vendor/`,
así que funciona sin red y sin servidor (`file://` incluido).

Si prefieres servidor:

```bash
cd prototype-web && python3 -m http.server 8777   # → http://localhost:8777
```

Usa **audífonos**. Los graves del pad y del sub no existen en la bocina de una laptop, y
son la mitad del efecto.

Teclas: `espacio` play · `1`–`7` herramienta · `A` modo apreciación · `C` vaciar.
Click sobre un objeto para quitarlo.

## El mapeo

El disco es un secuenciador circular. El barrido gira una vez por ciclo y hace sonar lo
que toca — el jugador *ve* qué suena y por qué.

| Dimensión visual | Dimensión musical |
|---|---|
| Ángulo (16 sectores) | Posición en el ciclo. 16 corcheas = 2 compases ≈ 7 s a 68 BPM |
| Anillo (5) | Grado de la escala: interior = grave del modo, exterior = agudo |
| Tipo de objeto | Timbre + octava base. La masa visual determina el registro |
| Densidad total | Abre el filtro maestro y activa capas: drone (6+), pulso (14+), shimmer (24+) |
| Momento del día | Escala, tónica, tempo, reverb y paleta visual |

Siete objetos cubren el registro completo: `hongo` (sub) → `árbol` (pad) →
`arbusto`/`helecho` (medios) → `flor` (campana) → `luciérnaga` (shimmer), más `piedra`
(percusión, sin altura).

### Por qué no puede sonar mal

1. Todas las notas salen de **una sola escala** por momento del día.
2. Todo está **cuantizado** al grid de corcheas.
3. El sub-bajo (`hongo`) está restringido a **tónica y quinta**: nunca ensucia el fondo armónico.
4. Cada objeto tiene una duración fija por tipo, así que las capas se solapan por diseño.

El resultado es que la densidad es la única variable de calidad: más objetos = más tapiz,
menos = más aire. Nunca disonancia. Eso es lo que hace que apps como *Bloom* funcionen.

## Qué hay que decidir después de escucharlo

- **¿El ciclo de ~7 s es demasiado corto?** Si se siente repetitivo, la solución no es
  alargarlo sino que el barrido complete una vuelta cada 4 compases y los objetos disparen
  en subdivisiones distintas.
- **¿5 anillos dan suficiente variedad melódica?** Quizá haga falta que el anillo module
  también la octava, no solo el grado.
- **¿Los timbres aguantan 10 minutos?** Estos son sintetizados a propósito, para probar
  la *estructura*. En producción se reemplazan por sample packs grabados por un compositor,
  capa contra capa, en la misma tonalidad y tempo. **Ese es el trabajo que decide el
  proyecto**, y este prototipo no lo sustituye.

## Handoff a la fase 2 (IA)

La pestaña **Payload IA** genera, en vivo, los dos artefactos que necesita el render final:

- **JSON estructurado** — bioma, momento, BPM, escala, densidad, conteo de objetos, capas
  activas y un `prompt_texto` derivado de la escena real, no escrito a mano.
- **`.mid`** — la melodía del ciclo repetida 4 veces, para usarse como *melody
  conditioning*. La IA no inventa la pieza: produce la que el jugador construyó.

La percusión se excluye del MIDI a propósito (va en el JSON como capa), porque condicionar
melodía con eventos sin altura degrada el resultado.

## Notas de implementación

- `reverb.generate()` renderiza la respuesta al impulso en un contexto offline y **puede no
  resolver nunca** en Safari/iOS y en headless. El arranque lo corre en `Promise.race` con
  un timeout; la reverb se conecta sola cuando termina.
- La inclinación del disco (`TILT`) se calcula según el aspecto de la pantalla: en móvil la
  vista se vuelve más cenital para no leerse como una raya plana.
- Micro-humanización determinista de ±6 ms por celda (`ring*7 + slot*13`), suficiente para
  que no suene a máquina sin romper la cuantización.

## Lo que se traslada a Godot

El estado de una escena es un **diccionario de celdas** (`{anillo, paso, tipo}`), no una
lista de notas. La música se deriva de la escena en cada reproducción. Eso mantiene los
guardados diminutos, hace el undo trivial y permite cambiar el mapeo o los samples sin
invalidar lo que el jugador ya creó.
