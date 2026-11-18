#!/usr/bin/env bash
# Construye el APK firmado de Lienzo. La llave vive fuera del repo en
# ~/.local/share/lienzo/ (keystore + contraseña); si no existe, se crea.
#
#   tools/android_apk.sh            # construye
#   tools/android_apk.sh install    # construye e instala por adb
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
D="$HOME/.local/share/lienzo"
export ANDROID_HOME="${ANDROID_HOME:-$HOME/Android/Sdk}"
export ANDROID_NDK_ROOT="${ANDROID_NDK_ROOT:-$(ls -d "$ANDROID_HOME"/ndk/* | sort -V | tail -1)}"
export ANDROID_NDK_HOME="$ANDROID_NDK_ROOT"

if [ ! -f "$D/lienzo.keystore" ]; then
    mkdir -p "$D"
    PASS=$(head -c 24 /dev/urandom | base64 | tr -d '/+=' | head -c 24)
    (umask 077; printf '%s' "$PASS" > "$D/keystore.pass")
    keytool -genkeypair -v -keystore "$D/lienzo.keystore" -alias lienzo -keyalg RSA \
        -keysize 2048 -validity 10000 -storepass "$PASS" -keypass "$PASS" \
        -dname "CN=Lienzo, O=SesoLibre" >/dev/null
    echo "llave nueva en $D/lienzo.keystore (guárdala: sin ella no se pueden actualizar instalaciones)"
fi
export CARGO_APK_RELEASE_KEYSTORE="$D/lienzo.keystore"
export CARGO_APK_RELEASE_KEYSTORE_PASSWORD="$(cat "$D/keystore.pass")"

"$ROOT/tools/android_assets.sh"
cd "$ROOT/lienzo"
cargo apk build -p lienzo-app --release --lib
APK="$ROOT/lienzo/target/release/apk/lienzo.apk"
echo "APK: $APK ($(du -h "$APK" | cut -f1))"

if [ "${1:-}" = "install" ]; then
    adb install -r "$APK"
    echo "instalado; para ver el registro: adb logcat -s RustStdoutStderr lienzo_app bevy_app"
fi
