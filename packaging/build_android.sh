#!/usr/bin/env bash
# Builds the Android APK on Linux (used by the GitHub release workflow); same steps as the
# Android part of build_all.ps1.
#   build_android.sh <out.apk>
# Needs ANDROID_HOME, ANDROID_NDK_HOME, a JDK, and for signing ANDROID_KEYSTORE (path) and
# ANDROID_KEYSTORE_PASSWORD. Without them the APK stays unsigned and is not written.
set -euo pipefail
OUT=${1:?output apk}
NAME=UniversalSaveEditor
SDK=${ANDROID_HOME:?}
NDK=${ANDROID_NDK_HOME:?}
BIN=$NDK/toolchains/llvm/prebuilt/linux-x86_64/bin
BT=$(ls -d "$SDK"/build-tools/* | sort -V | tail -1)
PLATFORM=$(ls -d "$SDK"/platforms/android-* | sort -V | tail -1)
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
APK=target/apk
rm -rf "$APK"; mkdir -p "$APK"

libs=()
for pair in arm64-v8a:aarch64-linux-android x86_64:x86_64-linux-android; do
  abi=${pair%%:*}; t=${pair##*:}; u=${t//-/_}; up=${u^^}
  export "CC_$u=$BIN/${t}26-clang" "AR_$u=$BIN/llvm-ar" "CARGO_TARGET_${up}_LINKER=$BIN/${t}26-clang"
  # 16 KB pages: required for Android 15+ devices that use them.
  CARGO_ENCODED_RUSTFLAGS="${CARGO_ENCODED_RUSTFLAGS}"$'\x1f-C\x1flink-arg=-Wl,-z,max-page-size=16384' \
    cargo rustc --release --lib --crate-type cdylib --target "$t"
  mkdir -p "$APK/lib-$abi"
  "$BIN/llvm-strip" --strip-unneeded "target/$t/release/libuniversal_save_editor.so" -o "$APK/lib-$abi/libuniversal_save_editor.so"
  libs+=("lib/$abi/libuniversal_save_editor.so=$APK/lib-$abi/libuniversal_save_editor.so")
done

icons=$(dirname "$(ls target/aarch64-linux-android/release/build/universal_save_editor-*/out/icons/192.png | head -1)")
python3 -I packaging/package.py icons "$icons" "$APK/res"
IFS=. read -r MA MI PA <<<"$VERSION"
CODE=$((MA * 10000 + MI * 100 + PA))
sed "s/@VERSION@/$VERSION/; s/@VERSION_CODE@/$CODE/" packaging/android/AndroidManifest.xml > "$APK/AndroidManifest.xml"
mkdir -p "$APK/compiled"
"$BT/aapt2" compile --dir "$APK/res" -o "$APK/compiled"
"$BT/aapt2" link -o "$APK/unsigned.apk" --manifest "$APK/AndroidManifest.xml" -I "$PLATFORM/android.jar" "$APK"/compiled/*.flat
mkdir -p "$APK/java/classes"
javac -source 8 -target 8 -Xlint:-options -classpath "$PLATFORM/android.jar" -d "$APK/java/classes" $(find packaging/android/java -name '*.java')
"$BT/d8" --release --min-api 26 --lib "$PLATFORM/android.jar" --output "$APK/java" $(find "$APK/java/classes" -name '*.class')
python3 -I packaging/package.py add "$APK/unsigned.apk" "classes.dex=$APK/java/classes.dex" "${libs[@]}"
"$BT/zipalign" -f -p 4 "$APK/unsigned.apk" "$APK/aligned.apk"

: "${ANDROID_KEYSTORE:?no signing key}"
"$BT/apksigner" sign --ks "$ANDROID_KEYSTORE" --ks-pass env:ANDROID_KEYSTORE_PASSWORD --out "$OUT" "$APK/aligned.apk"
"$BT/apksigner" verify "$OUT"
rm -f "$OUT.idsig"
