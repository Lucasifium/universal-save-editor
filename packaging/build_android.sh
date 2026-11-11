#!/usr/bin/env bash
# Builds the Android APK on Linux (used by the GitHub release workflow); same steps as the
# Android part of build_all.ps1.
#   build_android.sh <out.apk>
# Needs ANDROID_HOME, ANDROID_NDK_HOME and a JDK. Signs with ANDROID_KEYSTORE / ANDROID_KEYSTORE_PASSWORD
# if set, else with a key it makes in packaging/android/ (git-ignored).
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

# Signing key: your own, made once on first use. Android only installs updates signed with the same key.
if [ -z "${ANDROID_KEYSTORE:-}" ]; then
  ANDROID_KEYSTORE=packaging/android/release.keystore
  if [ ! -f "$ANDROID_KEYSTORE" ]; then
    ANDROID_KEYSTORE_PASSWORD=$(head -c 18 /dev/urandom | base64 | tr -dc 'A-Za-z0-9' | head -c 24)
    printf '%s' "$ANDROID_KEYSTORE_PASSWORD" > packaging/android/keystore-password.txt
    keytool -genkeypair -keystore "$ANDROID_KEYSTORE" -alias universalsaveeditor -keyalg RSA -keysize 4096 -validity 10000 \
      -storepass "$ANDROID_KEYSTORE_PASSWORD" -keypass "$ANDROID_KEYSTORE_PASSWORD" -dname 'CN=UniversalSaveEditor'
  fi
  ANDROID_KEYSTORE_PASSWORD=${ANDROID_KEYSTORE_PASSWORD:-$(cat packaging/android/keystore-password.txt)}
fi
"$BT/apksigner" sign --ks "$ANDROID_KEYSTORE" --ks-pass env:ANDROID_KEYSTORE_PASSWORD --out "$OUT" "$APK/aligned.apk"
"$BT/apksigner" verify "$OUT"
rm -f "$OUT.idsig"
