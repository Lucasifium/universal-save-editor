#!/usr/bin/env bash
# Prints the release notes for one version from CHANGELOG.md: the headline (a "> " line under the
# version's heading), the changes, and the downloads of that version.
set -euo pipefail
v="$1"
repo="https://github.com/Lucasifium/universal-save-editor"
section=$(awk -v v="$v" '/^## /{f=($2==v);next} f' CHANGELOG.md)
headline=$(printf '%s\n' "$section" | sed -n 's/^> //p' | head -1)
changes=$(printf '%s\n' "$section" | grep -v '^> ' | sed '/./,$!d')
dl="$repo/releases/download/v$v/UniversalSaveEditor-$v"
if [ -n "$headline" ]; then printf '%s\n\n' "$headline"; fi
printf "## What's new\n\n%s\n\n" "$changes"
cat <<EOF
## Download

| System | File |
|---|---|
| Windows (64-bit) | [UniversalSaveEditor-$v-windows-x64.exe]($dl-windows-x64.exe) |
| Windows on ARM | [UniversalSaveEditor-$v-windows-arm64.exe]($dl-windows-arm64.exe) |
| Linux (64-bit) | [UniversalSaveEditor-$v-linux-x64.tar.gz]($dl-linux-x64.tar.gz) |
| Linux on ARM | [UniversalSaveEditor-$v-linux-arm64.tar.gz]($dl-linux-arm64.tar.gz) |
| macOS 11+ (Apple Silicon and Intel) | [UniversalSaveEditor-$v-macos.zip]($dl-macos.zip) |

No install needed: the [browser version](https://universalsaveeditor.com/app/) runs the same editor in your browser, and your save is never uploaded.

Windows may say "Windows protected your PC" because the builds aren't code-signed yet: choose **More info**, then **Run anyway**. On a Mac, the app isn't signed by Apple: right-click it, choose **Open**, then **Open** again. To check a download, compare it with \`SHA256SUMS.txt\` below.

Edited saves are for single-player play. The editor backs up every save before changing it.
EOF
