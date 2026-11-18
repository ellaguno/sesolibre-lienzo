# Prompts de sprites — Fase 0 (mundo submarino)

Objetivo de esta tanda: **fijar el estilo** y tener material para la imagen fija de la
Fase 0. No es el set definitivo; es para saber si el *teatro de papel bajo el agua* nos gusta.

## Cómo generar

1. Genera primero **solo la medusa** con 4–6 variantes. Elige la que más te guste; esa es
   la referencia de estilo. Si tu herramienta admite imagen de referencia (Midjourney
   `--sref`, Flux/SD con IP-Adapter), úsala para todas las demás.
2. Luego el resto, 3–4 variantes por elemento, y quédate con 1–2.
3. **Tamaño:** 1024×1024, PNG. **Fondo blanco liso** — yo lo quito con `rembg` en local;
   no pidas transparencia porque casi ningún generador la hace bien.
4. Guarda cada imagen como `assets/sprites/raw/<elemento>_<nn>.png` y anota en
   `assets/sprites/prompts/registro.md` el modelo/herramienta y la fecha. Eso va al
   manifiesto de créditos.

Cualquier herramienta sirve (Midjourney, Flux, Ideogram, DALL-E, Stable Diffusion). Si
usas SD en tu máquina, la Quadro P1000 de 4 GB solo aguanta SD 1.5 a 512px; para esta
fase conviene una herramienta en línea.

## Bloque de estilo (se antepone a todos)

```
paper cutout illustration of {ELEMENTO}, children's picture book style, flat layered
paper shapes with soft watercolor texture and subtle paper grain, clean simple silhouette,
limited palette of deep blues, teal and one warm accent, soft rim light from above,
single subject, side view, centered, isolated on plain white background, no text,
no watermark, no frame
```

**Negative prompt** (si la herramienta lo admite):

```
photorealistic, 3d render, glossy, complex details, busy background, scene, water,
bubbles, text, watermark, frame, multiple subjects, cropped
```

## Elementos

| Archivo | {ELEMENTO} | Notas |
|---|---|---|
| `medusa` | a translucent jellyfish with long flowing tentacles, faint inner glow, pale violet and teal | **Primero.** Fija el estilo. |
| `pez_a` | a small sardine-like fish, slender, silver-blue, swimming to the right | Para el cardumen; necesito 3 formas distintas |
| `pez_b` | a small round reef fish, orange and white accent, swimming to the right | |
| `pez_c` | a small angelfish, tall thin body, deep blue, swimming to the right | |
| `coral_a` | a branching fan coral, warm coral-orange, growing upward from a small rock base | Se pega al suelo |
| `coral_b` | a round brain coral cluster, muted pink and teal | |
| `anemona` | a sea anemone with many soft tentacles, pale pink with teal tips, on a small rock | |
| `kelp_hoja` | a single tall kelp blade, long wavy leaf, olive green and teal, growing upward from the bottom edge | Una sola hoja; el bosque se arma por código |
| `ballena` | a humpback whale seen from the side, swimming to the left, dark indigo silhouette with pale belly | Irá lejos y pequeña; silueta limpia |
| `barco` | a small sunken wooden sailing ship resting on sand, broken mast, dark teak and rust | Elemento único |
| `roca_a` | a rounded underwater boulder, dark blue-grey with faint teal moss | 2 variantes |
| `roca_b` | a tall jagged rock pillar, dark blue-grey | |
| `manta` | a manta ray seen from below, gliding, dark top and pale belly | Opcional; para contemplación |

## Qué buscamos y qué no

- **Sí:** formas limpias que se lean como silueta a tamaño pequeño; textura de papel y
  acuarela sutil; la misma paleta en todos; luz desde arriba en todos.
- **No:** fondos, agua o burbujas dentro del sprite (eso lo pone el motor); detalle fino
  que desaparece al reducir; brillos de render 3D; estilos distintos entre elementos.

Si al ver la medusa el estilo no te convence, cambia el bloque de estilo (por ejemplo
`stained glass` o `ink and watercolor`) y vuelve a generar solo la medusa hasta que te
guste. Es más barato cambiar aquí que después.
