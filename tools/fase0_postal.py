#!/usr/bin/env python3
"""Fase 0 — imagen fija del abismo: sprites recortados + atmósfera procedural.

Prueba de GUSTO de la dirección de arte "teatro de papel bajo el agua". No es el
motor; es lo que el motor debería poder producir. Salida: assets/images/postal_fase0.png

Uso: python3.11 tools/fase0_postal.py
"""
import os

import numpy as np
from PIL import Image, ImageFilter

HERE = os.path.dirname(__file__)
CUT = os.path.join(HERE, "..", "assets", "sprites", "cut")
OUT = os.path.join(HERE, "..", "assets", "images", "postal_fase0.png")
W, H = 1920, 1080
rng = np.random.default_rng(3)

TOP = np.array([28, 92, 128], float)      # agua cerca de la superficie
BOTTOM = np.array([3, 10, 28], float)     # abismo


def gradient():
    y = np.linspace(0, 1, H)[:, None, None] ** 1.3
    img = TOP * (1 - y) + BOTTOM * y
    x = np.linspace(-1, 1, W)[None, :, None]
    img = img * (1 - 0.08 * x ** 2)           # un poco más oscuro en los bordes
    return img


def light_rays(img):
    rays = Image.new("L", (W, H), 0)
    px = rays.load()
    arr = np.zeros((H, W), float)
    for _ in range(9):
        x0 = rng.uniform(300, 1500)
        width = rng.uniform(40, 140)
        tilt = rng.uniform(-0.35, 0.15)
        strength = rng.uniform(0.35, 1.0)
        ys = np.arange(H)[:, None]
        xs = np.arange(W)[None, :]
        cx = x0 + ys * tilt
        d = np.abs(xs - cx) / (width * (1 + ys / H * 1.2))
        arr += strength * np.clip(1 - d, 0, 1) ** 2 * (1 - ys / H) ** 2.2
    arr = np.clip(arr / arr.max(), 0, 1)
    r = Image.fromarray((arr * 255).astype(np.uint8)).filter(ImageFilter.GaussianBlur(18))
    a = np.array(r, float)[:, :, None] / 255
    glow = np.array([120, 190, 210], float)
    return img + a * glow * 0.55


def sand(img):
    ys = np.arange(H)[:, None]
    floor = np.clip((ys - 0.78 * H) / (0.07 * H), 0, 1)
    sandcol = np.array([52, 84, 108], float)
    noise = rng.standard_normal((H, W))
    noise = np.array(Image.fromarray(((noise - noise.min()) / np.ptp(noise) * 255).astype(np.uint8))
                     .filter(ImageFilter.GaussianBlur(6)), float) / 255
    xs = np.arange(W)[None, :]
    caustic = (np.sin(xs / 37 + ys / 23) * np.sin(xs / 53 - ys / 31) * np.sin(xs / 19 + 2)) ** 2
    tex = sandcol * (0.85 + 0.3 * noise + 0.5 * caustic)[:, :, None]
    return img * (1 - floor[:, :, None]) + tex * floor[:, :, None]


def to_image(arr):
    return Image.fromarray(np.clip(arr, 0, 255).astype(np.uint8), "RGB").convert("RGBA")


def sprite(name, height, flip=False, rot=0.0, fog=0.0, blur=0.0, tint=None):
    im = Image.open(os.path.join(CUT, name + ".png")).convert("RGBA")
    s = height / im.height
    im = im.resize((max(1, int(im.width * s)), int(height)), Image.LANCZOS)
    if flip:
        im = im.transpose(Image.FLIP_LEFT_RIGHT)
    if rot:
        im = im.rotate(rot, Image.BICUBIC, expand=True)
    a = np.array(im).astype(float)
    if fog > 0:
        # la niebla acerca el color al del agua; las capas lejanas se funden
        fogcol = tint if tint is not None else np.array([24, 60, 92])
        a[..., :3] = a[..., :3] * (1 - fog) + fogcol * fog
    im = Image.fromarray(a.astype(np.uint8), "RGBA")
    if blur > 0:
        im = im.filter(ImageFilter.GaussianBlur(blur))
    return im


def place(canvas, im, x, y, anchor="center"):
    if anchor == "bottom":
        pos = (int(x - im.width / 2), int(y - im.height))
    else:
        pos = (int(x - im.width / 2), int(y - im.height / 2))
    canvas.alpha_composite(im, pos)


def particles(canvas, n, far):
    layer = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    px = np.zeros((H, W, 4), np.uint8)
    for _ in range(n):
        x, y = int(rng.uniform(0, W)), int(rng.uniform(0, H * 0.82))
        r = int(rng.uniform(1, 2 if far else 4))
        a = int(rng.uniform(40, 90 if far else 160))
        px[max(0, y - r):y + r, max(0, x - r):x + r] = (180, 220, 230, a)
    layer = Image.fromarray(px, "RGBA").filter(ImageFilter.GaussianBlur(0.8 if far else 2.0))
    canvas.alpha_composite(layer)


def bloom_vignette_grain(canvas):
    rgb = np.array(canvas.convert("RGB")).astype(float)
    lum = rgb.mean(2)
    bright = np.clip((lum - 120) / 135, 0, 1)[:, :, None] * rgb
    b = np.array(Image.fromarray(bright.astype(np.uint8)).filter(ImageFilter.GaussianBlur(28)), float)
    rgb = rgb + 0.45 * b
    ys, xs = np.mgrid[0:H, 0:W]
    v = 1 - 0.55 * (((xs - W / 2) / (W / 2)) ** 2 + ((ys - H / 2) / (H / 2)) ** 2) ** 1.2
    rgb *= np.clip(v, 0, 1)[:, :, None]
    rgb += rng.standard_normal((H, W, 1)) * 3.5
    return Image.fromarray(np.clip(rgb, 0, 255).astype(np.uint8), "RGB")


def main():
    bg = gradient()
    bg = light_rays(bg)
    bg = sand(bg)
    c = to_image(bg)
    floor = int(H * 0.86)

    # ---- fondo: lo lejano se funde con el agua ----
    particles(c, 900, far=True)
    place(c, sprite("ballena", 150, fog=0.62, blur=1.2), 330, 250)
    place(c, sprite("roca_b", 420, fog=0.5, blur=0.8), 1560, floor - 40, "bottom")
    place(c, sprite("roca_a_2", 230, fog=0.5, blur=0.8), 1300, floor - 30, "bottom")
    for i in range(7):                       # bosque de kelp lejano
        x = 120 + i * 95 + rng.uniform(-20, 20)
        place(c, sprite("kelp_hoja", rng.uniform(320, 460), flip=i % 2 == 0,
                        rot=rng.uniform(-6, 6), fog=0.45, blur=0.9), x, floor - 20, "bottom")
    place(c, sprite("manta", 260, fog=0.5, blur=1.0), 1150, 300)

    # ---- medio ----
    place(c, sprite("barco", 420, fog=0.28, blur=0.4), 1480, floor + 10, "bottom")
    place(c, sprite("roca_a_1", 300, fog=0.25), 980, floor + 15, "bottom")
    place(c, sprite("coral_b", 200, fog=0.22), 1090, floor + 5, "bottom")
    place(c, sprite("coral_a_2", 290, fog=0.22), 1215, floor + 8, "bottom")
    for i in range(5):                       # kelp cercano
        x = 230 + i * 110 + rng.uniform(-25, 25)
        place(c, sprite("kelp_hoja", rng.uniform(520, 700), flip=i % 2 == 1,
                        rot=rng.uniform(-5, 5), fog=0.15), x, floor + 10, "bottom")
    # cardumen: un trazo ondulado convertido en peces
    t = np.linspace(0, 1, 14)
    for i, u in enumerate(t):
        x = 560 + u * 760
        y = 430 + 70 * np.sin(u * 2 * np.pi * 1.1) + rng.uniform(-18, 18)
        name = ["pez_a_1", "pez_a_2", "pez_a_3"][i % 3]
        place(c, sprite(name, rng.uniform(34, 52), flip=True, rot=rng.uniform(-8, 8), fog=0.2), x, y)
    particles(c, 350, far=False)

    # ---- frente ----
    place(c, sprite("coral_a_1", 360), 560, floor + 30, "bottom")
    place(c, sprite("anemona", 230), 780, floor + 28, "bottom")
    place(c, sprite("medusa", 300), 1500, 520)
    place(c, sprite("pez_c", 150, flip=True, rot=6), 1180, 640)
    place(c, sprite("pez_b", 120, rot=-5), 860, 600)

    final = bloom_vignette_grain(c)
    final.save(OUT)
    print(OUT, final.size)


if __name__ == "__main__":
    main()
