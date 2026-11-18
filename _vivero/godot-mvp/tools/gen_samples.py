#!/usr/bin/env python3
"""
Genera el sample pack del MVP paso 1.

NO es el sample pack de producción. Es un placeholder sintetizado para poder
probar la ESTRUCTURA (mapeo, cuantización, densidad) sin depender de assets.
El pack real lo graba un compositor, capa contra capa, en la misma tonalidad y
tempo — ese trabajo es el 60% de la calidad percibida y esto no lo sustituye.

Uso:  python3 tools/gen_samples.py
Salida: assets/audio/*.wav  (mono, 44.1 kHz, 16 bits)
"""

import math
import os
import wave

import numpy as np

SR = 44100
OUT = os.path.join(os.path.dirname(__file__), "..", "assets", "audio")

# Cada instrumento se genera a una altura de referencia y Godot lo transpone con
# pitch_scale. Las referencias están en el centro del rango de uso para que el
# desplazamiento nunca pase de ~9 semitonos (más allá el timbre se degrada).
REFS = {
    # --- bosque ---
    "pad": 51,        # árbol      · rango de uso 43..60
    "sub": 36,        # hongo      · 31..42
    "pluck": 63,      # helecho    · 55..72
    "marimba": 63,    # arbusto    · 55..72
    "bell": 75,       # flor       · 67..84
    "shimmer": 87,    # luciérnaga · 79..96
    "perc": None,     # piedra     · sin altura
    # --- acuario / mar ---
    # La referencia va en el CENTRO del rango de uso: midi = root + 12*oct + grado,
    # con root en 55..62 y grado en 0..10, o sea ~63 + 12*oct. Salirse de ahí
    # obliga a remuestrear más de una octava y el timbre se deshace.
    "swell": 51,      # coral    (oct -1) · pad sumergido, ataque lentísimo
    "swell_hi": 63,   # alga     (oct  0)
    "waterdrop": 75,  # burbuja       · gota
    "glass": 75,      # anémona       · vaso, más limpio que la campana
    "blub": None,     # pez           · percusión acuática
    # --- ciudad ---
    "metal": 75,      # antena   (oct +1) · pluck inarmónico
    "click": None,    # farola        · percusión seca
    # --- selva ---
    "flute": 63,      # bambú    (oct  0) · soplo, aire audible
    "chirp": 87,      # ave      (oct +2) · trino corto
    "tom": None,      # rana          · tambor hueco
    # --- mar ---
    "horn": 51,       # faro     (oct -1) · sirena grave
    "wash": None,     # ola           · rompiente
    # --- interior ---
    "rhodes": 63,     # maceta   (oct  0) · eléctrico blando
    "musicbox": 75,   # libro    (oct +1) · caja de música
    "tick": None,     # reloj         · segundero
    # --- capas de fondo, por familia de ambiente ---
    # Que los siete biomas compartieran los mismos dos loops era la causa
    # principal de que todos sonaran igual: el fondo continuo es lo que más
    # tiempo está presente en el oído.
    "drone_loop": 33,    # bosque, selva  · madera y tierra
    "drone_deep": 33,    # acuario, mar   · masa de agua
    "drone_open": 33,    # cielo          · acorde abierto
    "drone_room": 33,    # ciudad, interior · zumbido de sala
    "drone_surge": 33,   # mar            · marejada, respira
    "air_loop": None,    # bosque, selva  · aire entre hojas
    "air_water": None,   # acuario, mar   · corriente
    "air_high": None,    # cielo          · brillo lejano
    "air_dust": None,    # ciudad, interior · suelo de ruido seco
    "air_surf": None,    # mar            · rompiente lejana
}


def freq(midi):
    return 440.0 * 2.0 ** ((midi - 69) / 12.0)


def t_axis(dur):
    return np.arange(int(SR * dur)) / SR


def exp_env(t, decay, attack=0.002):
    """Envolvente percusiva: ataque lineal corto + caída exponencial."""
    e = np.exp(-t / decay)
    a = np.clip(t / max(attack, 1e-6), 0.0, 1.0)
    return e * a


def onepole_lp(x, cutoff):
    """Pasa-bajos de un polo con frecuencia de corte variable en el tiempo."""
    cutoff = np.broadcast_to(np.asarray(cutoff, dtype=np.float64), x.shape)
    y = np.empty_like(x)
    prev = 0.0
    for i in range(x.size):
        a = 1.0 - math.exp(-2.0 * math.pi * cutoff[i] / SR)
        prev += a * (x[i] - prev)
        y[i] = prev
    return y


def lp_poles(x, cutoff, poles=3):
    """
    Pasa-bajos en cascada. Un solo polo cae a 6 dB/oct y no delimita banda: al
    restar dos, el resultado conserva casi toda la energía muy por encima de la
    frecuencia nominal. Tres polos (18 dB/oct) sí colocan la banda donde se pide.
    """
    y = x
    for _ in range(poles):
        y = onepole_lp(y, cutoff)
    return y


def band(x, lo, hi, poles=3):
    return lp_poles(x, hi, poles) - lp_poles(x, lo, poles)


def normalize(x, peak=0.89):
    m = np.max(np.abs(x))
    return x * (peak / m) if m > 1e-9 else x


def fade(x, ms_in=3.0, ms_out=30.0):
    """Evita clicks en los extremos."""
    n_in, n_out = int(SR * ms_in / 1000), int(SR * ms_out / 1000)
    x = x.copy()
    if n_in:
        x[:n_in] *= np.linspace(0, 1, n_in)
    if n_out:
        x[-n_out:] *= np.linspace(1, 0, n_out)
    return x


def seamless(x, xfade_s=1.0):
    """
    Convierte una señal en loop sin costura.

    Se recorta la cola y se mezcla sobre la cabeza. Así el último sample del
    resultado es x[L-n-1] y el primero es x[L-n]: contiguos en el original, que
    es justo lo que hace que el salto del loop sea inaudible.
    """
    n = int(SR * xfade_s)
    assert x.size > 2 * n
    out = x[: x.size - n].copy()
    w = np.linspace(0, 1, n)
    out[:n] = x[x.size - n:] * (1 - w) + x[:n] * w
    return out


# --------------------------------------------------------------------------
# instrumentos
# --------------------------------------------------------------------------

def make_pad(m):
    """Árbol: stack de sierras desafinadas bajo un filtro que se abre."""
    f, t = freq(m), t_axis(6.5)
    x = np.zeros_like(t)
    for det, gain in ((1.0, 1.0), (1.004, 0.7), (0.996, 0.7), (2.0, 0.35)):
        ph = 2 * np.pi * f * det * t
        x += gain * (2.0 * (ph / (2 * np.pi) % 1.0) - 1.0)   # sierra
    # Sub-octava discreta: el registro grave es territorio del `hongo`. Si el pad
    # pesa demasiado abajo, los dos se pelean y la mezcla se enloda.
    x += 0.20 * np.sin(2 * np.pi * f * 0.5 * t)
    cut = 280 + 900 * np.clip(t / 1.6, 0, 1) * np.exp(-t / 4.0)
    x = onepole_lp(x, cut)
    amp = np.clip(t / 1.1, 0, 1) * (0.45 + 0.55 * np.exp(-t / 2.2)) * np.exp(-t / 3.4)
    return fade(normalize(x * amp), ms_out=180)


def make_sub(m):
    """Hongo: seno con 2º armónico y un poco de saturación suave."""
    f, t = freq(m), t_axis(2.0)
    x = np.sin(2 * np.pi * f * t) + 0.22 * np.sin(2 * np.pi * 2 * f * t)
    x = np.tanh(x * 1.4)
    return fade(normalize(x * exp_env(t, 0.62, attack=0.02)), ms_out=120)


def make_pluck(m):
    """Helecho: Karplus-Strong. Ruido inicial + línea de retardo realimentada."""
    f = freq(m)
    n = int(SR * 2.2)
    d = max(2, int(SR / f))
    rng = np.random.default_rng(7)
    buf = rng.uniform(-1, 1, d)
    buf = onepole_lp(buf, 3600)          # ataque menos áspero
    y = np.empty(n)
    idx = 0
    for i in range(n):
        cur, nxt = buf[idx], buf[(idx + 1) % d]
        v = 0.5 * (cur + nxt) * 0.9965   # promedio = pasa-bajos del bucle
        y[i] = cur
        buf[idx] = v
        idx = (idx + 1) % d
    t = t_axis(2.2)[:n]
    y *= np.exp(-t / 1.4)
    # El ruido de excitación reparte energía por todos los armónicos y el filtro
    # del bucle deja al 2º dominando sobre el 1º: la nota se percibe una octava
    # arriba. Un seno al fundamental (resonancia de caja) lo devuelve a su sitio.
    body = np.sin(2 * np.pi * f * t) * exp_env(t, 0.9, attack=0.004) * 0.42
    return fade(normalize(y + body), ms_out=90)


def make_fm(m, ratio, index, decay, idx_decay, dur):
    f, t = freq(m), t_axis(dur)
    mod = np.sin(2 * np.pi * f * ratio * t) * index * np.exp(-t / idx_decay)
    x = np.sin(2 * np.pi * f * t + mod)
    return fade(normalize(x * exp_env(t, decay)), ms_out=90)


def make_marimba(m):
    """Arbusto: FM armónica (ratio 3) — madera, caída rápida."""
    x = make_fm(m, 3.0, 6.5, 0.42, 0.10, 1.6)
    t = t_axis(len(x) / SR)[: len(x)]
    click = np.sin(2 * np.pi * 620 * t) * exp_env(t, 0.012) * 0.25
    return fade(normalize(x + click), ms_out=90)


def make_bell(m):
    """
    Flor: FM inarmónica (ratio 5.02) — campana, cola larga.

    Con índice alto la energía se va a las bandas laterales y deja de leerse como
    campana afinada: suena a platillo. 4.2 mantiene el brillo sin perder altura.
    """
    return make_fm(m, 5.02, 4.2, 1.25, 0.22, 3.4)


def make_shimmer(m):
    """Luciérnaga: triángulos desafinados, ataque suave, mucho aire."""
    f, t = freq(m), t_axis(3.2)
    x = np.zeros_like(t)
    for det, g in ((1.0, 1.0), (2.01, 0.45), (3.0, 0.2)):
        ph = 2 * np.pi * f * det * t
        x += g * (2.0 / np.pi) * np.arcsin(np.sin(ph))       # triángulo
    amp = np.clip(t / 0.14, 0, 1) * np.exp(-t / 1.0)
    trem = 1.0 - 0.3 * (0.5 - 0.5 * np.cos(2 * np.pi * 3.2 * t))
    return fade(normalize(x * amp * trem), ms_out=110)


def make_perc(_m):
    """Piedra: ruido con banda estrecha + golpe de madera."""
    t = t_axis(0.30)
    rng = np.random.default_rng(11)
    noise = rng.uniform(-1, 1, t.size)
    band = onepole_lp(noise, 2600) - onepole_lp(noise, 700)   # pasa-banda burdo
    wood = np.sin(2 * np.pi * 380 * t) * exp_env(t, 0.020)
    x = band * exp_env(t, 0.055) + 0.5 * wood
    return fade(normalize(x), ms_out=40)


def _drone(m, partials, cutoff, seconds=8.0):
    """
    Base de drone. Los desafines son múltiplos de 1/seconds Hz para que el
    batido complete un periodo entero y el loop no tenga costura.
    """
    f, t = freq(m), t_axis(seconds)
    x = np.zeros_like(t)
    step = 1.0 / seconds
    for mult, det_steps, g in partials:
        x += g * np.sin(2 * np.pi * (f * mult + det_steps * step) * t)
    x = onepole_lp(x, cutoff)
    return normalize(fade(x, ms_in=1.0, ms_out=1.0), peak=0.72)


def make_drone(m):
    """Bosque y selva: tónica y quinta, madera. Cálido y cerrado."""
    return _drone(m, ((1.0, 0, 1.0), (1.0, 1, 0.9), (1.5, 0, 0.55),
                      (1.5, -1, 0.5), (2.0, 0, 0.22)), 520)


def make_drone_deep(m):
    """
    Acuario y mar: masa de agua. Solo octavas y quinta muy filtradas — bajo el
    agua los armónicos altos no llegan, y eso es justo lo que se reconoce.
    """
    return _drone(m, ((0.5, 0, 0.9), (1.0, 0, 1.0), (1.0, 1, 0.85),
                      (1.5, -1, 0.35)), 220)


def make_drone_open(m):
    """
    Cielo: acorde abierto, y sobre todo ALTO.

    Los cuatro drones vivían por debajo de 200 Hz, así que en los biomas de poca
    densidad —donde el drone es casi lo único que suena— resultaban
    indistinguibles. Este sube hasta el parcial 12 y con el filtro muy arriba: es
    lo que separa "cielo" de "fondo de mar".
    """
    return _drone(m, ((1.0, 0, 0.42), (1.5, 1, 0.55), (2.25, 0, 0.60),
                      (3.0, -1, 0.52), (4.5, 1, 0.44), (6.0, 0, 0.34),
                      (9.0, -1, 0.24), (12.0, 1, 0.16)), 3500)


def make_drone_room(m):
    """
    Ciudad e interior: zumbido de sala. Fundamental estrecho con armónicos
    impares débiles — la firma de un transformador, no de un instrumento.
    """
    # Armónicos impares hasta el 9 y filtro en la zona media: el zumbido de una
    # sala tiene cuerpo audible, no es solo un rumor grave.
    return _drone(m, ((1.0, 0, 1.0), (2.0, 0, 0.34), (3.0, 1, 0.42),
                      (5.0, 0, 0.30), (7.0, -1, 0.20), (9.0, 1, 0.12)), 900)


def make_drone_surge(m):
    """
    Mar: marejada. Igual de grave que el acuario pero RESPIRA — una amplitud que
    sube y baja es lo que separa mar abierto de tanque cerrado, y con tan pocas
    notas sonando es la capa la que define el ambiente.
    """
    x = _drone(m, ((0.5, 0, 0.7), (1.0, 0, 1.0), (1.0, 2, 0.8),
                   (1.5, -1, 0.45), (2.0, 1, 0.25)), 400)
    t = t_axis(len(x) / SR)[: len(x)]
    # Dos periodos enteros en el loop: sin costura.
    surge = 0.45 + 0.55 * (0.5 - 0.5 * np.cos(2 * np.pi * 2.0 * t / (len(x) / SR)))
    return normalize(x * surge, peak=0.72)


def _air(seed, lo, hi, lfo_hz, depth, seconds=9.0, peak=0.5):
    """Colchón de ruido con banda y movimiento propios. Loop con crossfade."""
    t = t_axis(seconds)
    rng = np.random.default_rng(seed)
    noise = rng.uniform(-1, 1, t.size)
    bed = band(noise, lo, hi)
    lfo = (1.0 - depth) + depth * np.sin(2 * np.pi * lfo_hz * t)
    return normalize(seamless(bed * lfo, 1.2), peak=peak)


def make_air(_m):
    """Bosque y selva: aire entre hojas."""
    return _air(23, 2400, 6000, 0.11, 0.45)


def make_air_water(_m):
    """Acuario y mar: corriente. Banda baja y movimiento lento y amplio."""
    return _air(29, 500, 1900, 0.055, 0.60, peak=0.55)


def make_air_high(_m):
    """Cielo: brillo lejano, casi inmóvil."""
    return _air(37, 5000, 12000, 0.028, 0.30, peak=0.40)


def make_air_surf(_m):
    """Mar: rompiente lejana. Banda ancha con oleaje lento y profundo."""
    return _air(47, 1200, 7000, 0.075, 0.72, peak=0.55)


def make_air_dust(_m):
    """
    Ciudad e interior: suelo de ruido seco, sin movimiento. Un colchón que
    respira delata "exterior"; una habitación está quieta.
    """
    return _air(43, 900, 4200, 0.012, 0.10, peak=0.32)


# --------------------------------------------------------------------------
# acuario / mar
# --------------------------------------------------------------------------

def make_swell(m):
    """
    Coral y medusa: pad de ataque lentísimo, como si el sonido emergiera del
    agua. Sin transitorio: es lo que separa "sumergido" de "en el aire".
    """
    f, t = freq(m), t_axis(7.0)
    x = np.zeros_like(t)
    for mult, g in ((1.0, 1.0), (2.0, 0.45), (3.0, 0.18), (4.01, 0.09)):
        # Desafinación lenta: el agua no deja nada perfectamente estable.
        drift = 1.0 + 0.0012 * np.sin(2 * np.pi * 0.13 * t + mult)
        x += g * np.sin(2 * np.pi * f * mult * drift * t)
    cut = 200 + 700 * np.clip(t / 2.6, 0, 1) * np.exp(-t / 5.0)
    x = onepole_lp(x, cut)
    amp = np.clip(t / 2.4, 0, 1) ** 1.6 * np.exp(-t / 3.2)
    return fade(normalize(x * amp), ms_in=8.0, ms_out=220)


def make_waterdrop(m):
    """Burbuja: gota. Un seno con la altura cayendo rápido y volviendo a subir."""
    f, t = freq(m), t_axis(0.85)
    # El glissando ASCENDENTE es lo que el oído lee como "gota", y tiene que
    # asentarse EN la nota: si acaba por debajo, cada burbuja suena desafinada
    # respecto a la escala (la primera versión caía 3 semitonos).
    bend = f * (1.0 - 0.45 * np.exp(-t / 0.055))
    phase = 2 * np.pi * np.cumsum(bend) / SR
    x = np.sin(phase) + 0.18 * np.sin(2 * phase)
    return fade(normalize(x * exp_env(t, 0.16, attack=0.001)), ms_out=60)


def make_glass(m):
    """Anémona: vaso. FM casi armónica — más limpia y fría que la campana."""
    x = make_fm(m, 4.0, 2.6, 1.6, 0.30, 3.0)
    t = t_axis(len(x) / SR)[: len(x)]
    air = np.sin(2 * np.pi * freq(m) * 2.0 * t) * exp_env(t, 2.0) * 0.16
    return fade(normalize(x + air), ms_out=120)


def make_blub(_m):
    """Pez: percusión acuática, sorda y redonda. Nada de ataque seco."""
    t = t_axis(0.35)
    rng = np.random.default_rng(31)
    noise = onepole_lp(rng.uniform(-1, 1, t.size), 420)
    thud = np.sin(2 * np.pi * 150 * t * (1.0 + 0.5 * np.exp(-t / 0.03)))
    x = thud * exp_env(t, 0.075, attack=0.006) + noise * exp_env(t, 0.05) * 0.7
    return fade(normalize(x), ms_out=60)


# --------------------------------------------------------------------------
# ciudad / interior
# --------------------------------------------------------------------------

def make_metal(m):
    """Antena: pluck inarmónico. Karplus-Strong con el bucle poco amortiguado."""
    f = freq(m)
    n = int(SR * 2.6)
    d = max(2, int(SR / f))
    rng = np.random.default_rng(13)
    buf = onepole_lp(rng.uniform(-1, 1, d), 7000)
    y = np.empty(n)
    idx = 0
    for i in range(n):
        # Menos promediado que en `pluck`: los agudos sobreviven y suena a metal.
        v = (0.82 * buf[idx] + 0.18 * buf[(idx + 1) % d]) * 0.9988
        y[i] = buf[idx]
        buf[idx] = v
        idx = (idx + 1) % d
    t = t_axis(2.6)[:n]
    y *= np.exp(-t / 1.8)
    body = np.sin(2 * np.pi * f * t) * exp_env(t, 1.1, attack=0.003) * 0.3
    return fade(normalize(y + body), ms_out=110)


def make_click(_m):
    """Farola: percusión seca, de aro. Corta y con cuerpo alto."""
    t = t_axis(0.16)
    rng = np.random.default_rng(17)
    noise = rng.uniform(-1, 1, t.size)
    band = onepole_lp(noise, 5200) - onepole_lp(noise, 1800)
    tone = np.sin(2 * np.pi * 1250 * t) * exp_env(t, 0.010)
    return fade(normalize(band * exp_env(t, 0.022) + 0.4 * tone), ms_out=25)


# --------------------------------------------------------------------------
# selva
# --------------------------------------------------------------------------

def make_flute(m):
    """
    Bambú: tubo soplado. La clave es el RUIDO DE SOPLO audible en el ataque —
    sin él un seno con vibrato suena a sintetizador, no a caña.
    """
    f, t = freq(m), t_axis(2.4)
    vib = 1.0 + 0.006 * np.sin(2 * np.pi * 4.8 * t) * np.clip(t / 0.5, 0, 1)
    phase = 2 * np.pi * np.cumsum(f * vib) / SR
    body = np.sin(phase) + 0.22 * np.sin(2 * phase) + 0.07 * np.sin(3 * phase)
    rng = np.random.default_rng(41)
    breath = onepole_lp(rng.uniform(-1, 1, t.size), 3000) * np.exp(-t / 0.10) * 0.5
    amp = np.clip(t / 0.09, 0, 1) * (0.5 + 0.5 * np.exp(-t / 1.4)) * np.exp(-t / 1.6)
    return fade(normalize(body * amp + breath * amp), ms_out=120)


def make_chirp(m):
    """Ave: dos glissandos ascendentes encadenados, muy cortos."""
    f, t = freq(m), t_axis(0.55)
    out = np.zeros_like(t)
    for start, dur, up in ((0.00, 0.10, 1.6), (0.14, 0.08, 1.35)):
        seg = (t >= start) & (t < start + dur)
        local = t[seg] - start
        bend = f * (1.0 + (up - 1.0) * (local / dur))
        out[seg] += np.sin(2 * np.pi * np.cumsum(bend) / SR) * np.exp(-local / (dur * 0.5))
    return fade(normalize(out), ms_out=40)


def make_tom(_m):
    """Rana: tambor hueco. Membrana con caída de altura, sin brillo."""
    t = t_axis(0.55)
    f = 118.0 * (1.0 + 0.9 * np.exp(-t / 0.045))
    x = np.sin(2 * np.pi * np.cumsum(f) / SR)
    x = np.tanh(x * 1.3)
    return fade(normalize(x * exp_env(t, 0.13, attack=0.003)), ms_out=70)


# --------------------------------------------------------------------------
# mar
# --------------------------------------------------------------------------

def make_horn(m):
    """Faro: sirena. Armónicos impares, ataque lento, cuerpo grande."""
    f, t = freq(m), t_axis(4.2)
    x = np.zeros_like(t)
    for mult, g in ((1.0, 1.0), (2.0, 0.30), (3.0, 0.42), (5.0, 0.16)):
        x += g * np.sin(2 * np.pi * f * mult * t)
    x = onepole_lp(x, 600)
    amp = np.clip(t / 0.35, 0, 1) * np.minimum(1.0, np.exp(-(t - 1.6) / 1.1))
    return fade(normalize(x * amp), ms_in=6.0, ms_out=200)


def make_wash(_m):
    """
    Ola: rompiente. Ruido que se abre y se cierra — la envolvente ASIMÉTRICA
    (sube lento, baja aún más lento) es lo que lo separa de un simple 'shhh'.
    """
    t = t_axis(3.2)
    rng = np.random.default_rng(53)
    noise = rng.uniform(-1, 1, t.size)
    # El corte del filtro sube con la ola y vuelve a caer: espuma y resaca.
    cut = 300 + 3200 * np.exp(-((t - 0.9) ** 2) / 0.45)
    x = onepole_lp(noise, cut)
    amp = np.clip(t / 0.7, 0, 1) * np.exp(-np.maximum(0.0, t - 0.9) / 1.1)
    return fade(normalize(x * amp), ms_in=20.0, ms_out=250)


# --------------------------------------------------------------------------
# interior
# --------------------------------------------------------------------------

def make_rhodes(m):
    """Maceta: eléctrico blando. FM de índice bajo y campana muy suave."""
    x = make_fm(m, 2.0, 1.9, 1.5, 0.55, 3.0)
    t = t_axis(len(x) / SR)[: len(x)]
    tine = np.sin(2 * np.pi * freq(m) * 6.0 * t) * exp_env(t, 0.05) * 0.11
    return fade(normalize(x + tine), ms_out=110)


def make_musicbox(m):
    """Libro: caja de música. Púa metálica, inarmónica y corta."""
    x = make_fm(m, 7.03, 3.1, 0.75, 0.14, 2.0)
    t = t_axis(len(x) / SR)[: len(x)]
    pin = np.sin(2 * np.pi * freq(m) * 11.0 * t) * exp_env(t, 0.008) * 0.2
    return fade(normalize(x + pin), ms_out=80)


def make_tick(_m):
    """Reloj: segundero. Dos clics muy secos, uno agudo y otro sordo."""
    t = t_axis(0.09)
    rng = np.random.default_rng(59)
    noise = rng.uniform(-1, 1, t.size)
    hi = (onepole_lp(noise, 9000) - onepole_lp(noise, 3500)) * exp_env(t, 0.004)
    lo = np.sin(2 * np.pi * 700 * t) * exp_env(t, 0.006)
    return fade(normalize(hi + 0.5 * lo), ms_out=15)


BUILDERS = {
    "pad": make_pad, "sub": make_sub, "pluck": make_pluck, "marimba": make_marimba,
    "bell": make_bell, "shimmer": make_shimmer, "perc": make_perc,
    "swell": make_swell, "swell_hi": make_swell, "waterdrop": make_waterdrop, "glass": make_glass,
    "blub": make_blub, "metal": make_metal, "click": make_click,
    "flute": make_flute, "chirp": make_chirp, "tom": make_tom,
    "horn": make_horn, "wash": make_wash,
    "rhodes": make_rhodes, "musicbox": make_musicbox, "tick": make_tick,
    "drone_loop": make_drone, "drone_deep": make_drone_deep,
    "drone_open": make_drone_open, "drone_room": make_drone_room,
    "drone_surge": make_drone_surge, "air_surf": make_air_surf,
    "air_loop": make_air, "air_water": make_air_water,
    "air_high": make_air_high, "air_dust": make_air_dust,
}


def write_wav(path, x):
    pcm = np.clip(x, -1.0, 1.0)
    pcm = (pcm * 32767.0).astype("<i2")
    with wave.open(path, "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(SR)
        w.writeframes(pcm.tobytes())
    return pcm.size


def main():
    os.makedirs(OUT, exist_ok=True)
    total = 0
    for name, build in BUILDERS.items():
        x = build(REFS[name])
        path = os.path.join(OUT, name + ".wav")
        frames = write_wav(path, x)
        total += frames * 2
        ref = REFS[name]
        print(f"  {name:11s} {frames / SR:5.2f} s   ref={'—' if ref is None else 'midi ' + str(ref)}")
    print(f"\n{len(BUILDERS)} archivos, {total / 1024:.0f} KiB en assets/audio/")


if __name__ == "__main__":
    main()
