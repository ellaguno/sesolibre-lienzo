# Vivero

Juego de móvil donde **construir es componer**: se puebla un diorama con objetos y cada
objeto colocado es a la vez una decisión visual y una decisión musical. El resultado es un
espacio que se puede dejar puesto y escuchar por sí mismo.

## Estado

| Carpeta | Qué es | Estado |
|---|---|---|
| `prototype-web/` | Prototipo del núcleo de audio en Tone.js | Listo. Solo valida el mapeo. |
| `godot-mvp/` | MVP: diorama 3D, 7 ambientes, 4 layouts | Listo, audio sin escuchar. |
| `ai-render/` | Fase 2: servicio de render de audio | Pipeline listo; proveedores de IA sin probar. |
| `shared/` | `catalog.json` — contrato entre juego y servicio | Generado por el juego. |

## El plan

**Paso 1 — sin IA.** Motor de audio cuantizado, cámara orbital, galería, export de postal.
Objetivo: comprobar que colocar se siente bien y suena bien. *Si esto no engancha, la IA no
lo salva.*

Generalizado a **siete ambientes sobre cuatro geometrías**, porque el disco circular es
una opción y no la forma del juego:

| Bioma | Layout | Paso (tiempo) | Nivel (altura tonal) |
|---|---|---|---|
| Bosque, Selva | disco radial | ángulo | anillo |
| Acuario, Interior | pared vertical | X | Y (los objetos flotan) |
| Mar, Ciudad | retícula en suelo | X | Z (los objetos crecen en Y) |
| Cielo | cúpula | azimut | elevación |

Añadir un ambiente es una entrada de diccionario en `Biomes` más su set de mallas y
muestras. Ninguna de las siete tocó el secuenciador, la persistencia ni la UI.

**Paso 2 — render de audio con IA.** *Pipeline construido* (`ai-render/`). El payload
estructurado + el `.mid` van a un modelo que produce la pieza que el jugador construyó.
El servicio es agnóstico: elegir proveedor es una variable de entorno.

Sigue pendiente **decidir el proveedor**, y para eso hay que escuchar. Solo `musicgen`
ofrece condicionamiento melódico real, que es lo que hace que el render sea "tu jardín
producido" y no "una pieza parecida" — pero necesita más VRAM de la que tiene la Quadro
P1000 (4 GB frente a ~5 GB). Stability y ElevenLabs tienen licencia comercial explícita
pero no condicionan con melodía.

**Paso 3 — más biomas + estilizador visual.** La IA como *estilizador*, no como generador:
img2img con la estructura fijada por el depth map de la propia escena. La composición sigue
siendo del jugador; la IA solo cambia el material.

## Decisiones tomadas

- **3D** con cámara orbital acotada, nunca libre. Resuelve la colocación por toque sin
  ambigüedad de perspectiva.
- **Godot 4** (Forward Mobile).
- **IA como estilizador visual**, no generación libre: no le quita autoría al jugador ni
  rompe la consistencia de estilo.
- El estado de una escena es un **diccionario de celdas**, no una lista de notas. La música
  se deriva en cada reproducción.
- **Sin fail state, sin timers de espera.** El modo apreciación es el producto, no el
  epílogo.
- **La cámara es la posición de escucha**, no un mezclador oculto: mover la escena cambia
  filtro, reverb y paneo, pero nunca la composición. La transposición tiene control propio
  porque es una decisión que se guarda con el jardín.

## El riesgo que decide el proyecto

No es técnico, es **composicional**: hace falta que un compositor diseñe los sample packs
por bioma como *stems que se apilan* — grabados capa contra capa, en la misma tonalidad y
tempo, probados en densidades altas y bajas. Es el 60% de la calidad percibida y ninguna de
las dos implementaciones de este repo lo sustituye: ambas usan placeholders sintetizados
para validar la estructura.

El segundo riesgo es la **latencia de audio en Android** y la precisión de los disparos
atada al frame. Ver las limitaciones conocidas en `godot-mvp/README.md`.
