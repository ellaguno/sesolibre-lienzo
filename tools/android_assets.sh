#!/usr/bin/env bash
# Prepara la carpeta de assets que va dentro del APK: solo lo que la app usa
# (sprites recortados, pack de audio, manifiesto), nunca el material crudo.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DST="$ROOT/lienzo/android/assets"
rm -rf "$DST" && mkdir -p "$DST/sprites" "$DST/audio"
cp -r "$ROOT/assets/sprites/cut" "$DST/sprites/"
for d in instruments ambience events; do cp -r "$ROOT/assets/audio/$d" "$DST/audio/"; done
cp "$ROOT/assets/audio/pack.txt" "$DST/audio/"
cp "$ROOT/assets/manifest.toml" "$DST/"
find "$DST" -name "*.wav" -delete
echo "assets para el APK en $DST: $(du -sh "$DST" | cut -f1)"
