#!/usr/bin/env bash
# Construye el Android App Bundle (AAB) firmado para Google Play, sin Gradle:
# compila la biblioteca con cargo-apk, enlaza recursos y manifiesto con aapt2
# en formato protobuf, arma el módulo base y lo empaqueta con bundletool.
#
#   tools/android_aab.sh            # construye lienzo/target/play/lienzo.aab
#   tools/android_aab.sh apk        # además, un APK universal para probar
#
# Necesita bundletool (java -jar). Descárgalo de
#   https://github.com/google/bundletool/releases
# y déjalo en $BUNDLETOOL (por defecto ~/.local/share/lienzo/tools/bundletool.jar).
#
# La llave de subida es la misma de android_apk.sh (~/.local/share/lienzo/).
# versionCode: AAAMMDDN (fecha más un dígito), siempre creciente.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
D="$HOME/.local/share/lienzo"
BUNDLETOOL="${BUNDLETOOL:-$D/tools/bundletool.jar}"
export ANDROID_HOME="${ANDROID_HOME:-$HOME/Android/Sdk}"
BT="$(ls -d "$ANDROID_HOME"/build-tools/* | sort -V | tail -1)"
PLATFORM="$(ls -d "$ANDROID_HOME"/platforms/android-* | sort -V | tail -1)"
AAPT2="$BT/aapt2"
ANDROID_JAR="$PLATFORM/android.jar"

if [ ! -f "$BUNDLETOOL" ]; then
    echo "falta bundletool en $BUNDLETOOL (ver cabecera de este guion)" >&2
    exit 1
fi

VERSION_NAME="$(grep -m1 '^version' "$ROOT/lienzo/Cargo.toml" | sed 's/.*"\(.*\)".*/\1/')"
VERSION_CODE="${VERSION_CODE:-$(date +%Y%m%d)0}"

# 1. biblioteca nativa y assets (lo mismo que el APK de desarrollo)
"$ROOT/tools/android_apk.sh" >/dev/null
SO="$ROOT/lienzo/target/aarch64-linux-android/release/liblienzo_app.so"
[ -f "$SO" ] || { echo "no está $SO" >&2; exit 1; }

OUT="$ROOT/lienzo/target/play"
rm -rf "$OUT" && mkdir -p "$OUT/base" "$OUT/tmp"

# 2. manifiesto con versión
sed -e "s/VERSION_CODE/$VERSION_CODE/" -e "s/VERSION_NAME/$VERSION_NAME/" \
    "$ROOT/lienzo/android/AndroidManifest.xml" > "$OUT/tmp/AndroidManifest.xml"

# 3. recursos en protobuf (lo que exige el bundle)
"$AAPT2" compile --dir "$ROOT/lienzo/android/res" -o "$OUT/tmp/res.zip"
"$AAPT2" link --proto-format -o "$OUT/tmp/base.apk" \
    --manifest "$OUT/tmp/AndroidManifest.xml" -I "$ANDROID_JAR" \
    --min-sdk-version 28 --target-sdk-version 35 \
    --version-code "$VERSION_CODE" --version-name "$VERSION_NAME" \
    -A "$ROOT/lienzo/android/assets" \
    --auto-add-overlay "$OUT/tmp/res.zip"

# 4. módulo base: manifest/, res/, resources.pb, assets/, lib/
(cd "$OUT/base" && unzip -q "$OUT/tmp/base.apk")
mkdir -p "$OUT/base/manifest" "$OUT/base/lib/arm64-v8a"
mv "$OUT/base/AndroidManifest.xml" "$OUT/base/manifest/"
cp "$SO" "$OUT/base/lib/arm64-v8a/liblienzo_app.so"
(cd "$OUT/base" && zip -q -r "$OUT/tmp/base.zip" .)

# 5. bundle y firma con la llave de subida
java -jar "$BUNDLETOOL" build-bundle --modules="$OUT/tmp/base.zip" --output="$OUT/lienzo-unsigned.aab"
PASS="$(cat "$D/keystore.pass")"
jarsigner -sigalg SHA256withRSA -digestalg SHA-256 \
    -keystore "$D/lienzo.keystore" -storepass "$PASS" -keypass "$PASS" \
    -signedjar "$OUT/lienzo.aab" "$OUT/lienzo-unsigned.aab" lienzo >/dev/null
rm -f "$OUT/lienzo-unsigned.aab"
echo "AAB: $OUT/lienzo.aab ($(du -h "$OUT/lienzo.aab" | cut -f1)) versionCode=$VERSION_CODE versionName=$VERSION_NAME"

# 6. opcional: APK universal desde el bundle, para instalar y comprobar
if [ "${1:-}" = "apk" ]; then
    rm -f "$OUT/lienzo.apks"
    java -jar "$BUNDLETOOL" build-apks --bundle="$OUT/lienzo.aab" --output="$OUT/lienzo.apks" \
        --mode=universal --ks="$D/lienzo.keystore" --ks-pass="pass:$PASS" \
        --ks-key-alias=lienzo --key-pass="pass:$PASS"
    (cd "$OUT" && unzip -q -o lienzo.apks universal.apk)
    echo "APK universal: $OUT/universal.apk (adb install -r)"
fi
