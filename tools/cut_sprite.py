#!/usr/bin/env python3
"""Recorta un sprite generado sobre fondo blanco: quita el fondo, conserva solo la
pieza más grande (elimina pececitos y adornos sueltos), recorta al contenido y
guarda PNG con alpha.   Uso: python3.11 tools/cut_sprite.py entrada.jpg salida.png"""
import sys
import numpy as np
from PIL import Image
from rembg import remove
from scipy import ndimage

src, dst = sys.argv[1], sys.argv[2]
im = remove(Image.open(src).convert("RGB"))
a = np.array(im)
alpha = a[..., 3]
labels, n = ndimage.label(alpha > 40)
if n > 1:
    sizes = ndimage.sum(np.ones_like(alpha), labels, range(1, n + 1))
    keep = labels == (1 + int(np.argmax(sizes)))
    alpha = np.where(keep, alpha, 0).astype(np.uint8)
    a[..., 3] = alpha
ys, xs = np.where(alpha > 0)
pad = 8
box = (max(0, xs.min() - pad), max(0, ys.min() - pad),
       min(a.shape[1], xs.max() + pad), min(a.shape[0], ys.max() + pad))
Image.fromarray(a).crop(box).save(dst)
print(dst, Image.open(dst).size, f"piezas descartadas: {n - 1}")
