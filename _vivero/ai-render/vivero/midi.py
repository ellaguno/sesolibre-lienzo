"""
Lector mínimo de Standard MIDI File (formatos 0 y 1).

Existe para *verificar* lo que el juego exporta antes de mandárselo a un modelo
—un .mid mal formado se acepta silenciosamente y devuelve basura— y para poder
inspeccionarlo desde la CLI. Los proveedores suben el archivo tal cual.
"""

from __future__ import annotations

import struct
from dataclasses import dataclass


class MidiError(ValueError):
    pass


@dataclass
class Note:
    start_ticks: int
    end_ticks: int
    pitch: int
    velocity: int

    def start_s(self, ppq: int, bpm: float) -> float:
        return self.start_ticks / ppq * 60.0 / bpm

    def dur_s(self, ppq: int, bpm: float) -> float:
        return (self.end_ticks - self.start_ticks) / ppq * 60.0 / bpm


@dataclass
class Midi:
    ppq: int
    bpm: float
    notes: list
    total_ticks: int

    @property
    def duration_s(self) -> float:
        return self.total_ticks / self.ppq * 60.0 / self.bpm


def _read_vlq(data: bytes, i: int) -> tuple[int, int]:
    value = 0
    for _ in range(4):
        if i >= len(data):
            raise MidiError("VLQ truncada")
        b = data[i]
        i += 1
        value = (value << 7) | (b & 0x7F)
        if not b & 0x80:
            return value, i
    raise MidiError("VLQ de más de 4 bytes")


def parse(data: bytes) -> Midi:
    if len(data) < 14 or data[:4] != b"MThd":
        raise MidiError("no es un MIDI: falta la cabecera MThd")
    hlen, fmt, ntracks, division = struct.unpack(">IHHH", data[4:14])
    if hlen < 6:
        raise MidiError(f"cabecera MThd de longitud inválida: {hlen}")
    if fmt not in (0, 1):
        raise MidiError(f"solo se soportan los formatos 0 y 1 (es {fmt})")
    if division & 0x8000:
        raise MidiError("división SMPTE no soportada; se espera ticks por negra")
    ppq = division

    i = 8 + hlen
    bpm = 120.0
    notes: list[Note] = []
    total = 0

    for _ in range(ntracks):
        if i + 8 > len(data):
            raise MidiError("pista anunciada que no existe")
        if data[i:i + 4] != b"MTrk":
            raise MidiError(f"se esperaba MTrk en el offset {i}")
        tlen = struct.unpack(">I", data[i + 4:i + 8])[0]
        i += 8
        end = i + tlen
        if end > len(data):
            raise MidiError("longitud de pista mayor que el archivo")

        t = 0
        running = None
        # pitch -> (tick de inicio, velocidad)
        sounding: dict[int, tuple[int, int]] = {}

        while i < end:
            delta, i = _read_vlq(data, i)
            t += delta
            if i >= end:
                raise MidiError("pista truncada")
            status = data[i]
            if status & 0x80:
                i += 1
                running = status if status < 0xF0 else None
            elif running is not None:
                status = running          # running status
            else:
                raise MidiError(f"byte de estado esperado en el offset {i}")

            kind = status & 0xF0
            if status == 0xFF:
                meta = data[i]
                length, i = _read_vlq(data, i + 1)
                if meta == 0x51 and length == 3:
                    mpq = int.from_bytes(data[i:i + 3], "big")
                    if mpq > 0:
                        bpm = 60_000_000.0 / mpq
                i += length
            elif status in (0xF0, 0xF7):
                length, i = _read_vlq(data, i)
                i += length
            elif kind in (0x80, 0x90):
                pitch, vel = data[i], data[i + 1]
                i += 2
                if kind == 0x90 and vel > 0:
                    sounding[pitch] = (t, vel)
                else:
                    started = sounding.pop(pitch, None)
                    if started is not None:
                        notes.append(Note(started[0], t, pitch, started[1]))
            elif kind in (0xA0, 0xB0, 0xE0):
                i += 2
            elif kind in (0xC0, 0xD0):
                i += 1
            else:
                raise MidiError(f"evento no soportado {hex(status)} en el offset {i}")

        # Las notas sin note-off se cierran al final de la pista en vez de
        # descartarse: un exportador con un bug no debería perder notas.
        for pitch, (start, vel) in sounding.items():
            notes.append(Note(start, t, pitch, vel))
        total = max(total, t)
        i = end

    notes.sort(key=lambda n: (n.start_ticks, n.pitch))
    return Midi(ppq=ppq, bpm=bpm, notes=notes, total_ticks=total)


def parse_file(path: str) -> Midi:
    with open(path, "rb") as f:
        return parse(f.read())
