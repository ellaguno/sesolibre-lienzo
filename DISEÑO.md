# Lienzo (sesolibre-lienzo) — documento de diseño

*Versión 1 · 18 de noviembre de 2026 · nombre provisional*

> **Pintas un paisaje y el paisaje se convierte en su propia banda sonora.**
> Cada trazo es a la vez un elemento visual y una frase musical. La obra terminada es un
> espacio tridimensional que suena, que evoluciona mientras lo recorres, y al que se vuelve.

Este documento reemplaza al proyecto *Vivero* (carpetas `prototype-web/`, `godot-mvp/`,
`ai-render/`, `shared/`), que queda como referencia histórica. Se empieza de cero.

---

## 1. Decisiones tomadas

| Tema | Decisión |
|---|---|
| Espacio | **3D**. Un volumen acotado que se recorre con la cámara para ver y oír la obra desde distintos ángulos. |
| Creación | **Pinceles semánticos**: se dibuja con el dedo y aparecen cosas (algas, cardúmenes, corales), no color. |
| Primer mundo | **Submarino**. Un solo mundo hasta que enganche. |
| Sonido | **Muestras reales** (instrumentos grabados y grabaciones de campo) tocadas por un motor generativo. Nada sintetizado como timbre principal, nada de IA en tiempo real. |
| Arte | Sprites **generados con IA y curados a mano** + elementos **procedurales** (luz, niebla, partículas, siluetas, kelp). |
| Tecnología | **Rust de punta a punta**: Bevy para render e input, `kira` para audio. |
| Plataformas | Linux (desarrollo y escucha diaria) → Android → web (galería) → iPhone cuando haya Mac. |
| Público | Yo, familia y amigos. Sin monetización. |
| Licencias | Solo CC0, CC-BY, CC-BY-SA o dominio público. Registro de autoría por cada asset y sección de créditos en la app. |
| Compartir | Las obras se suben a un sitio donde otros pueden gozarlas. |
| Ideas fuera de alcance | Van a `IDEAS.md`, no al código. |

---

## 2. La experiencia

### 2.1 Entrar

Se abre un **abismo vacío**: agua azul profunda, un fondo arenoso, rayos de luz que bajan de
la superficie, partículas suspendidas, un rumor grave de agua. Ya suena, aunque no haya
nada. La cámara orbita lento alrededor del centro.

### 2.2 Dibujar

Abajo hay una paleta de pinceles. Se elige uno y se arrastra el dedo por la pantalla. El
trazo se convierte en cosas: una línea de kelp que sigue la curva, un cardumen que nada a
lo largo del recorrido, una cresta de coral sobre la roca. Un toque suelto coloca un
elemento único (una medusa, un barco hundido, una ballena lejana).

**Desde el primer trazo suena algo nuevo.** El kelp trae un cello; los peces, un arpa; las
burbujas, una celesta. La frase que suena *tiene la forma del trazo*: si la línea sube, la
melodía sube. Nadie lo explica; se descubre a los dos o tres trazos. Ese descubrimiento es
el gancho.

### 2.3 Dibujar en 3D

El problema a resolver: un dedo en una pantalla plana tiene que colocar cosas en un
volumen.

- Hay un **plano de dibujo** paralelo a la cámara, a una profundidad ajustable con un
  control lateral (o con pinza). Se ve como una leve neblina luminosa, para saber dónde va
  a caer el trazo.
- Los pinceles de **suelo** (coral, roca, anémona, kelp) ignoran el plano y se **pegan al
  fondo o a la roca** bajo el trazo. Los de **agua** (peces, medusas, burbujas) usan el plano.
- Orbitar la cámara y volver a dibujar es la forma natural de llenar el volumen por capas:
  primero el fondo desde arriba, luego los cardúmenes desde el frente.
- Regla: **un gesto es trazo si empieza con un pincel activo; es órbita si se arrastra con
  dos dedos o desde fuera del lienzo.** Hay que probarlo con el dedo, no decidirlo en papel.

### 2.4 Contemplar

Un botón apaga la paleta y entra el **modo contemplación**: la cámara deriva despacio
(órbita lenta, o nado libre si se arrastra), la luz cambia con un ciclo lento, los
elementos viven (los peces nadan, el kelp se mece) y la música sigue evolucionando sin
repetirse. Se deja puesto. Es el producto, no el epílogo.

### 2.5 Interactuar

En contemplación, tocar un elemento lo hace **actuar**: la ballena canta, el cardumen se
dispersa y suelta su frase, la medusa pulsa y suena su acorde. Mover la cámara cambia el
punto de escucha: lo que está cerca suena presente y claro, lo lejano apagado y
reverberado. Nunca cambia la composición; solo la perspectiva.

### 2.6 Volver

Las obras se guardan en una galería local. Al abrir una, suena exactamente como se dejó y
se puede seguir dibujando o borrar trazos (cada trazo borrado se lleva su voz). Se puede
exportar una imagen, un video con audio o solo el audio largo.

### 2.7 Compartir

Desde la galería se sube la obra a un sitio. Ahí cualquiera, desde el navegador, la puede
**recorrer y escuchar**, no solo ver un video. Ver §6.

---

## 3. El mundo submarino

### 3.1 Dirección de arte: *teatro de papel bajo el agua*

Combina los dos recursos disponibles:

- **Sprites generados con IA**, curados a mano, colocados como **cartas planas en el
  espacio 3D** (billboards o cartas con ligera rotación). Un estilo único por prompt:
  ilustración de libro infantil, textura de acuarela, siluetas limpias, paleta restringida.
  Es la estética de un diorama de papel recortado o de un teatro de sombras, y es
  deliberada: hace que lo plano se vea bien dentro de lo tridimensional.
- **Lo procedural** pone la atmósfera, que es donde está el 70% de "se ve hermoso":
  niebla volumétrica azul por profundidad, rayos de luz desde la superficie, partículas
  (burbujas, sedimento), cáusticas en el fondo, bloom suave, grano, viñeta. El kelp y las
  rocas pueden ser procedurales (como en *abyssal-garden*, ver §8), lo que permite miles de
  hojas meciéndose sin un sprite por hoja.

Hora y profundidad definen la paleta: superficie turquesa y luminosa → abismo índigo con
bioluminiscencia. El ciclo lento de luz del modo contemplación recorre ese rango.

### 3.2 Pinceles iniciales (6–8)

| Pincel | Gesto | Se pega a | Instrumento | Registro | Sonido natural |
|---|---|---|---|---|---|
| **Kelp** | trazo | suelo | cello / viola | grave-medio | corriente entre hojas |
| **Coral** | trazo | suelo / roca | piano suave | medio | — |
| **Anémona** | toque | suelo | vibráfono | medio-agudo | — |
| **Cardumen** | trazo | agua | arpa | agudo | chapoteo lejano |
| **Medusa** | toque | agua | coro / pad de cuerdas | medio, acordes | — |
| **Burbujas** | trazo | agua (sube sola) | celesta / kalimba | muy agudo | burbujas reales |
| **Ballena** | toque | agua, lejos | contrabajo | muy grave | canto de ballena |
| **Barco hundido** | toque | suelo | — (no toca) | — | crujidos, metal |

Las muestras de cada instrumento deben pertenecer a la **misma familia sonora** (misma
sala, dinámica cercana, afinación 440) o el conjunto suena a collage. Ver §5.

### 3.3 Fuentes de arte y sonido candidatas

Solo licencias compartibles. Cada asset se registra en `assets/manifest.toml` (§7).

- Instrumentos: **VSCO 2 Community Edition** (orquesta, CC0), **Salamander Grand Piano**
  (CC-BY), **Pianobook** (verificar licencia por pack), **Philharmonia Orchestra samples**
  (verificar términos).
- Grabaciones de campo: **Freesound** filtrando CC0 / CC-BY (agua, burbujas, ballenas,
  hidrófono, superficie), **archivos NOAA** (dominio público en EE. UU., cantos de ballena).
- Sprites: generados por nosotros. Se guarda el prompt y el modelo con cada sprite.

---

## 4. Del trazo a la música

### 4.1 El trazo es la partitura

Cada trazo se guarda como una polilínea 3D más su pincel. De ahí se deriva un **motivo**:

| Propiedad del trazo | Parámetro musical |
|---|---|
| Altura (Y) a lo largo del trazo | contorno melódico (grado de la escala dentro del registro del instrumento) |
| Longitud total | duración de la frase (en pulsos, redondeada) |
| Velocidad del gesto | dinámica (lento = suave y ligado; rápido = marcato) |
| Curvatura local | ornamentos (una vuelta cerrada = mordente o apoyatura) |
| Número de elementos generados | densidad rítmica de la frase |
| Posición X / Z | paneo y distancia (espacialización 3D real) |
| Profundidad (Y absoluta en el volumen) | filtro: más hondo, más oscuro |
| Pincel | instrumento, registro, sonido natural asociado |

Un toque suelto (sin trazo) genera un motivo corto a partir de la posición y de un
generador determinista por semilla.

### 4.2 Forma, no bucle

No hay cuadrícula de pasos. Hay un **pulso lento** (50–70 BPM, con rubato) y un
**director** que trabaja por frases de 4 a 16 pulsos y lleva la pieza por **secciones**
—introducción, crecimiento, meseta, adelgazamiento— durante 5 a 10 minutos, y vuelve a
empezar sin repetirse nunca igual.

Un **ciclo de acordes lento** (minutos por vuelta, modal, pentatónico de base como en
*abyssal-garden*: Re pentatónico mayor funciona bien bajo el agua) da dirección armónica.
Los motivos se **reajustan al acorde del momento** llevando las notas fuertes al tono de
acorde más cercano.

### 4.3 Variaciones

Cada vez que un motivo vuelve a sonar, el director le aplica una o dos operaciones,
elegidas de forma determinista por semilla y número de aparición:

transposición dentro de la escala · inversión · retrógrado · aumentación / disminución ·
adelgazamiento (quitar notas) · ornamentación · cambio de octava · eco (repetición suave
desfasada) · respuesta (otro instrumento contesta el motivo).

Es literalmente *tema y variaciones*. La obra grande no repite más que la pequeña; solo
tiene más voces disponibles.

### 4.4 Capas de la mezcla

1. **Ambiente** — grabaciones largas de agua, mezcladas por profundidad y densidad de la
   escena. Dos copias a velocidades distintas para que el bucle no se perciba.
2. **Colchón** — un acorde sostenido con muestras reales (cuerdas, coro) que sigue el ciclo
   armónico; es el pegamento.
3. **Motivos** — las frases de los trazos, por instrumento, con límite de voces por pool.
4. **Eventos naturales** — la ballena canta, las burbujas suben, el barco cruje. Afinados a
   la escala cuando tienen altura; libres cuando no.
5. **Espacio y master** — cada elemento suena desde su posición 3D; filtro paso-bajo global
   por profundidad; reverb larga de convolución (IR submarina real o diseñada); limitador
   en el master para que apilar muchas voces nunca sature.

### 4.5 Determinismo

**Escena + semilla → la misma música, siempre.** Esto hace trivial renderizar video con
audio fuera de línea, garantiza que lo compartido suena igual que lo creado, y permite
probar el motor sin oírlo (pero *se va a oír*: §9).

---

## 5. Lo que decide el proyecto: gusto, no código

Dos riesgos, ninguno técnico:

- **Sonido.** Un conjunto de muestras incoherente suena a collage por muy buena que sea la
  composición. Hay que elegir *una* familia de instrumentos que convivan, nivelarlos,
  afinarlos y probarlos apilados en densidades altas y bajas.
- **Arte.** Sin una dirección de arte clara y sprites consistentes entre sí, la niebla y el
  bloom no salvan nada.

Mitigación única para ambos: **la Fase 0** (§9). Una imagen fija y 60 segundos de audio,
hechos a mano, antes de escribir una línea de motor. Si eso no emociona, se cambia el
estilo, no el código.

---

## 6. Compartir: la galería

Objetivo: que familia y amigos **recorran** la obra, no solo vean un video.

- **Formato de obra**: un JSON pequeño (trazos, pinceles, semilla, versión del motor) +
  una miniatura + opcionalmente un video de 30 s renderizado por la app.
- **El visor web es la misma app**: Bevy compila a WebAssembly y WebGPU. *abyssal-garden*
  demuestra que un abismo submarino 3D corre bien en el navegador. El visor carga la obra
  y la toca en vivo con las mismas muestras.
- **Sitio**: un backend mínimo en Rust (`axum`) con subida de obras, listado y páginas
  públicas, o incluso un sitio estático regenerado al subir. Se aloja junto a los sitios
  de SesoLibre. Sin cuentas al principio: un enlace por obra; "mis obras" se identifica por
  un token local.
- **Peso**: las muestras de audio son el cuello de botella en web. Un pack por mundo,
  comprimido (Ogg/Opus), cargado una vez y cacheado. Presupuesto objetivo: < 40 MB por mundo.
- Entra en la Fase 4; el formato de obra se diseña desde la Fase 1 pensando en esto.

---

## 7. Licencias y créditos

- Solo **CC0, CC-BY, CC-BY-SA y dominio público**. Nada con cláusula NC si se puede evitar
  (aunque el proyecto no sea comercial, el sitio público lo complica).
- Cada asset se registra en `assets/manifest.toml`:

  ```toml
  [[asset]]
  id = "cello_sustain_c3"
  path = "audio/instruments/cello/c3.ogg"
  kind = "instrument"
  author = "Versilian Studios"
  source = "https://vis.versilstudios.com/vsco-community.html"
  license = "CC0-1.0"
  modified = "recortado, normalizado a -18 LUFS, convertido a Opus"

  [[asset]]
  id = "sprite_medusa_01"
  path = "sprites/medusa_01.png"
  kind = "sprite"
  author = "Eduardo Llaguno (generado con <modelo>, prompt en prompts/medusa.txt)"
  license = "CC-BY-4.0"
  ```

- El manifiesto **genera** `CREDITOS.md` y la **sección de créditos de la app** (y del
  sitio). Un asset sin registro no entra al build.
- Las obras subidas al sitio llevan licencia elegida por el autor al subir (por defecto
  CC-BY).

---

## 8. Tecnología

```
lienzo/
├── core/        Rust puro, sin UI. Modelo de escena, intérprete de trazos, director,
│                motor de variaciones, mezcla y render offline a WAV. Se prueba headless.
├── app/         Bevy: cámara, plano de dibujo, pinceles, sprites-carta, kelp procedural,
│                niebla, rayos, partículas, post-proceso. Audio en vivo con `kira`.
├── gallery/     Backend `axum` + visor (la app compilada a wasm).
├── assets/      sprites, muestras, IRs, con `manifest.toml`.
└── tools/       curación de muestras (normalizar, afinar, convertir), generación de créditos.
```

- **Bevy** da render 3D con wgpu (Vulkan en Linux/Android, Metal en iOS/macOS, WebGPU en
  navegador), input táctil, y un ecosistema de partículas y post-proceso.
- **`kira`** programa eventos de audio con precisión de muestra desde un reloj propio, no
  desde el frame. Era el riesgo técnico número uno del intento anterior; aquí desaparece.
  Tiene espacialización, tweens y efectos (filtro, reverb, compresor).
- **El `core` no conoce a Bevy.** La app lo consulta; el render offline y las pruebas lo
  usan directo. Esto es lo que permite escuchar la música *antes* de que exista una
  pantalla.
- **Rendimiento**: 2D-en-3D con instancing para kelp y partículas, LOD por calidad
  (bajo/medio/alto con fallback automático, como *abyssal-garden*), presupuesto de 60 fps
  en un teléfono de gama media de 2020.
- **Plataformas**: Linux ahora. Android con `cargo-ndk` / `xbuild`. Web con `wasm-bindgen`
  + WebGPU. iOS requiere Mac con Xcode y cuenta de Apple: **aplazado**.

### Lo que se toma de *abyssal-garden* (MIT)

Reloj de audio separado del frame con *lookahead* · pools de voces con round-robin · drone
de quintas sobre Re pentatónico · profundidad → filtro y balance de capas · velocidad de
cámara → "rush" de agua · dos bucles del mismo ambiente a velocidades distintas · limitador
agresivo en el master · kelp, rocas y rayos procedurales · calibración de calidad antes de
entrar. Lo que *no* se toma: síntesis como timbre (aquí todo es muestra real) y la
ausencia de creación (ahí solo se nada).

---

## 9. Fases

| Fase | Entregable | Pregunta que responde |
|---|---|---|
| **0 · Gusto** ✅ *cerrada 18 nov 2026* | Una imagen fija del abismo con la dirección de arte (sprites IA + atmósfera) y **60 s de audio** armados a mano con las muestras reales elegidas. `manifest.toml` con todo lo usado. | ¿Esto emociona? Si no, cambiar el estilo antes de construir. |
| **1 · Core** ✅ | `core/`: escena → motivos → director → mezcla → WAV. Un CLI que toma una escena de prueba y rinde 3 minutos. | ¿Trazo→melodía y las variaciones suenan a música? **Se escucha todos los días.** |
| **2 · App Linux** ✅ | Lienzo 3D, plano de dibujo, 6–8 pinceles, audio en vivo, guardar/abrir. | ¿Dibujar se siente bien? ¿Se descubre solo que el trazo es la melodía? |
| **3 · Android** 🔧 *corre en Pixel 9 Pro desde el 18 nov 2026* | Build táctil, gestos, rendimiento. | ¿Funciona con el dedo y en un teléfono real? |
| **4 · Regreso y galería** | Contemplación, interacción, export de imagen/video/audio, sitio con visor web. | ¿Se vuelve a la obra? ¿La gozan otros? |
| **5 · Más** | Segundo mundo, iPhone, lo que salga de `IDEAS.md`. | — |

### Regla de trabajo

Nadie escuchó el intento anterior mientras se construía y se notó. Aquí:

1. Cada cambio al `core` produce un WAV que **tú** escuchas antes de seguir.
2. Cada cambio visual produce una captura que **tú** miras.
3. Las métricas (espectro, autocorrelación) sirven para encontrar bugs, nunca para decidir
   si algo es bello.

---

## 10. Preguntas abiertas (se resuelven haciendo, no discutiendo)

- Gesto exacto para distinguir trazo de órbita en pantalla táctil.
- Si el plano de dibujo necesita control de profundidad explícito o basta con los pinceles
  de suelo/agua más la órbita.
- Cuántas voces simultáneas aguanta la mezcla antes de embarrarse (se mide en Fase 1).
- Familia de instrumentos definitiva (se decide oyendo en Fase 0).

---

## 11. Versión 2 — luz, agua y dos vistas

*19 de noviembre de 2026. Tras la validación técnica en Android, Eduardo pidió más inmersión
sin perder el teatro de papel. Esto redefine la experiencia; el motor musical y el core se
quedan.*

### 11.1 Qué cambia y qué no

- **No cambia:** cada trazo es a la vez imagen y frase musical; el estilo de libro infantil;
  el motor determinista; la galería; que la cámara sea la posición de escucha.
- **Cambia:** hay **dos vistas** (dibujar desde arriba, bucear en 3D); los pinceles se
  organizan en **cinco familias** (elementos, color, luz, agua, ruta); el agua tiene
  **carácter propio** (del abismo índigo al Caribe cristalino); y la **inmersión** se
  construye con luz procedural (cáusticas, rayos, arcoíris, refracción, superficie), con
  *abyssal-garden* como referencia de técnica.

### 11.2 Las dos vistas

| | **Vista de dibujo** (lienzo) | **Vista de buceo** (escena) |
|---|---|---|
| Cámara | Cenital/isométrica, ortográfica, desde arriba. El suelo es el papel. | Perspectiva, dentro del agua. Órbita acotada o nado por la ruta. |
| Qué se hace | Dibujar: elementos, color, luz, agua, ruta. | Mirar, escuchar, tocar elementos para que actúen. Seguir la ruta de buceo. |
| Profundidad | Una **capa de altura** elegible (fondo / media agua / cerca de la superficie) con un deslizador; lo dibujado cae en esa capa. | Se ve el volumen completo. |
| Música | Suena lo que se dibuja; la escucha es "desde arriba", de conjunto. | La posición en la ruta es la posición de escucha: lo cercano presente, lo lejano velado. |

Un gesto (botón o pinza extrema) pasa de una a otra con una transición de cámara continua:
la vista cenital **se inclina y desciende** hasta meterse en el agua. Es la misma escena,
no dos pantallas.

### 11.3 Cinco familias de pinceles

1. **Elementos** (lo que ya existe): kelp, coral, anémona, cardumen, medusa, burbujas,
   ballena, barco… Siguen siendo sprites de papel sobre trazo o toque. Hay que ampliar el
   catálogo (tortuga, manta, pulpo, caballito de mar, estrella, roca, ánfora, buzo).
2. **Color**: trazos libres de acuarela que se quedan en el agua o sobre el fondo como
   manchas de papel pintado (nubes de plancton, bancos de arena rosada, sombras). Se
   renderizan como cintas texturizadas, con el mismo grano de papel que los sprites.
   *Musicalmente* son **colchón**: el tono define el modo y la textura (cálidos = cuerdas
   y coro; fríos = pads de cristal), el tamaño su peso en la mezcla.
3. **Luz**: fuentes de luz de color que se colocan como toques o trazos: faroles de
   bioluminiscencia, un haz que entra por la superficie, un arcoíris sutil en el agua,
   luciérnagas marinas. *Musicalmente* son lo **agudo y brillante**: celesta, arpa en
   armónicos, campanas, y su intensidad modula el brillo del máster.
4. **Agua**: no se dibuja; se elige. Ver 11.4.
5. **Ruta**: un trazo especial en la vista cenital, la **ruta de buceo** (11.6).

### 11.4 Las aguas

Un preset de agua define color de niebla, color de superficie y fondo, claridad (alcance
de la niebla), intensidad de cáusticas y rayos, y **el modo musical**. Los primeros cuatro:

| Agua | Luz y color | Música |
|---|---|---|
| **Abismo** (actual) | Índigo, niebla densa, rayos lejanos, bioluminiscencia | Re pentatónica, tempo lento, reverb larga |
| **Caribe** | Turquesa claro, arena blanca, cáusticas fuertes, mucha luz | Mayor luminoso, más tempo, arpa y marimba al frente |
| **Hawái** | Azul cobalto, rocas volcánicas oscuras, rayos nítidos | Pentatónica con color lidio, flauta |
| **Cristalina** (cenote/laguna) | Casi transparente, verde-azul, superficie muy presente | Modo claro, pocas voces, mucho aire |

El agua se elige al crear la obra y se puede cambiar: la misma obra "amanece" en otra agua.
Añadir un agua es una entrada de datos (colores, parámetros, escala), no código.

### 11.5 Inmersión: la luz procedural

Todo esto es trabajo de shader y de partículas, no de sprites, y por eso escala a
cualquier agua. Referencia técnica: *abyssal-garden* (rayos, niebla por profundidad,
kelp instanciado, superficie, paleta de agua por profundidad).

- **Cáusticas** animadas sobre el suelo y sobre las cartas: textura de cáusticas que se
  desplaza y se suma según la claridad del agua.
- **Rayos de luz** volumétricos desde la superficie, que se mueven con lentitud.
- **Superficie** vista desde abajo: ondulación, reflejo (efecto Snell: el mundo exterior
  cabe en un círculo) cuando la cámara mira arriba.
- **Refracción**: distorsión suave de toda la imagen con un campo de ondas lento, más
  fuerte cerca de la superficie y en aguas claras. Es un post-proceso de pantalla completa.
- **Arcoíris**: un arco tenue de color espectral, aditivo, que aparece donde hay un haz de
  luz y agua clara. Se puede dibujar (familia Luz) o dejar que el agua lo ponga.
- **Partículas**: sedimento, plancton bioluminiscente, burbujas; su color sigue al agua.

Restricción conocida: en Android vamos sobre GLES con material propio; cada efecto se
escribe en nuestro shader (`card.wesl` y sucesores), sin depender del PBR de Bevy. Es más
trabajo, pero es lo que ya funciona en Mali.

### 11.6 La ruta de buceo

En la vista cenital se traza un camino (familia Ruta). En la vista de buceo, **"bucear"**
recorre ese camino con la cámara a velocidad de nado, mirando hacia donde avanza con
suavidad, pasando entre lo dibujado. Como la cámara es la posición de escucha, la ruta
es también una **interpretación**: la pieza suena distinta según por dónde se nada. La
ruta se guarda con la obra; una obra puede tener varias. Al terminar, la cámara queda
libre en órbita.

El buceo es además el **vídeo natural** para compartir: ruta + obra = un recorrido
renderizable con su audio.

### 11.7 Qué le pasa a la música

- Nada del core cambia de contrato: la escena sigue siendo trazos; solo hay más familias
  de pincel, y cada una tiene instrumento, registro y papel en la mezcla.
- El **agua** aporta escala, tempo, reverb y balance; es el "momento del día" de Vivero
  reencarnado, pero con identidad sonora y visual.
- La **ruta** mueve el punto de escucha de forma continua (paneo, filtro, nivel por
  distancia a cada elemento); ya existe en el motor (`depth`, `pan`), solo que ahora lo
  gobierna la cámara en movimiento.

### 11.8 Orden de construcción propuesto

Cada paso produce algo que se ve y se oye; el usuario valida antes del siguiente.

1. **Aguas**: el preset como datos (colores, niebla, claridad, modo musical) y el cambio en
   caliente. Caribe y Abismo primero. *Es lo que más cambia la sensación por menos código.*
2. **Luz procedural básica**: cáusticas animadas en suelo y cartas, rayos que se mueven,
   partículas con el color del agua.
3. **Vista cenital de dibujo** con capa de altura, y la transición cenital ↔ buceo.
4. **Familia Color** (cintas de acuarela) y **familia Luz** (faroles, haz, arcoíris), con su
   música.
5. **Refracción y superficie** (post-proceso).
6. **Ruta de buceo**: trazo, reproducción, guardado; exportar el recorrido como vídeo.
7. Más elementos (sprites nuevos con Gemini, mismo estilo).

### 11.9 Decisiones que faltan

- ¿La vista de dibujo es **cenital pura** (como un mapa) o **isométrica** (se ve algo de
  altura)? Propongo cenital con ligera inclinación (~25°) para que los elementos se lean.
- ¿La capa de altura al dibujar es un **deslizador** o **tres botones** (fondo, agua,
  superficie)? Propongo tres botones; es más claro con el dedo.
- ¿El agua se elige **al crear** la obra o se puede **cambiar siempre**? Propongo siempre.
- ¿La ruta de buceo se reproduce **una vez** o **en bucle** durante la contemplación?
