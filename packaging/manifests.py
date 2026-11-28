"""Writes the Windows package-manager manifests for a published release.

  python3 -I packaging/manifests.py [version] [SHA256SUMS.txt]

With no arguments it uses the version in Cargo.toml and downloads that release's SHA256SUMS.txt.
Output:
  packaging/scoop/universal-save-editor.json      Scoop (install straight from its raw URL)
  packaging/flatpak/<app id>.yml                  Flatpak (build with flatpak-builder from this folder, see BUILDING.md)
  packaging/winget/<version>/*.yaml               winget (copy into a pull request to
                                                  microsoft/winget-pkgs under
                                                  manifests/l/Lucasifium/UniversalSaveEditor/<version>/)
"""
import datetime, json, os, re, sys, urllib.request

REPO = "https://github.com/Lucasifium/universal-save-editor"
SITE = "https://universalsaveeditor.com/"
ID = "Lucasifium.UniversalSaveEditor"
SHORT = "Free, offline save editor and save manager for PC games, with backups and restore."
LONG = ("Universal Save Editor opens and edits save files from hundreds of PC games, with a dedicated "
        "screen for the most popular ones and a values/hex editor for the rest. It keeps a backup "
        "before every save and works fully offline.")
TAGS = ["save-editor", "savegame", "games", "hex-editor", "backup", "offline"]
ARCHES = [("x64", "64bit"), ("arm64", "arm64")]
HERE = os.path.dirname(os.path.abspath(__file__))


def cargo_version():
    with open(os.path.join(HERE, "..", "Cargo.toml"), encoding="utf-8") as f:
        return re.search(r'^version = "([^"]+)"', f.read(), re.M).group(1)


def sums(version, path=None):
    if path:
        with open(path, encoding="utf-8") as f:
            text = f.read()
    else:
        url = f"{REPO}/releases/download/v{version}/SHA256SUMS.txt"
        with urllib.request.urlopen(url) as r:
            text = r.read().decode()
    out = {}
    for line in text.splitlines():
        parts = line.split()
        if len(parts) == 2 and re.fullmatch(r"[0-9a-fA-F]{64}", parts[0]):
            out[parts[1].lstrip("*")] = parts[0].lower()
    return out


def exe(arch, version):
    return f"UniversalSaveEditor-{version}-windows-{arch}.exe"


def scoop(version, hashes):
    base = f"{REPO}/releases/download/v{version}"
    auto = f"{REPO}/releases/download/v$version"
    m = {
        "version": version,
        "description": SHORT,
        "homepage": SITE,
        "license": "GPL-3.0-or-later",
        "architecture": {
            key: {"url": f"{base}/{exe(a, version)}#/UniversalSaveEditor.exe", "hash": hashes[exe(a, version)]}
            for a, key in ARCHES
        },
        "bin": "UniversalSaveEditor.exe",
        "shortcuts": [["UniversalSaveEditor.exe", "Universal Save Editor"]],
        "checkver": "github",
        "autoupdate": {
            "architecture": {
                key: {"url": f"{auto}/{exe(a, '$version')}#/UniversalSaveEditor.exe"} for a, key in ARCHES
            },
            "hash": {"url": "$baseurl/SHA256SUMS.txt"},
        },
    }
    d = os.path.join(HERE, "scoop")
    os.makedirs(d, exist_ok=True)
    with open(os.path.join(d, "universal-save-editor.json"), "w", encoding="utf-8", newline="\n") as f:
        json.dump(m, f, indent=4)
        f.write("\n")


def winget(version, hashes):
    head = f"PackageIdentifier: {ID}\nPackageVersion: {version}\n"
    tail = "ManifestVersion: 1.9.0\n"
    files = {
        f"{ID}.yaml": head + "DefaultLocale: en-US\nManifestType: version\n" + tail,
        f"{ID}.installer.yaml": head
        + "InstallerType: portable\nCommands:\n- UniversalSaveEditor\n"
        + f"ReleaseDate: {datetime.date.today().isoformat()}\nInstallers:\n"
        + "".join(
            f"- Architecture: {a}\n  InstallerUrl: {REPO}/releases/download/v{version}/{exe(a, version)}\n"
            f"  InstallerSha256: {hashes[exe(a, version)].upper()}\n"
            for a, _ in ARCHES
        )
        + "ManifestType: installer\n" + tail,
        f"{ID}.locale.en-US.yaml": head
        + "PackageLocale: en-US\nPublisher: Lucasifium\n"
        + f"PublisherUrl: https://github.com/Lucasifium\nPackageName: Universal Save Editor\n"
        + f"PackageUrl: {SITE}\nLicense: GPL-3.0-or-later\nLicenseUrl: {REPO}/blob/main/LICENSE\n"
        + f"ShortDescription: {SHORT}\nDescription: {LONG}\nTags:\n"
        + "".join(f"- {t}\n" for t in TAGS)
        + f"ReleaseNotesUrl: {REPO}/releases/tag/v{version}\n"
        + "ManifestType: defaultLocale\n" + tail,
    }
    root = os.path.join(HERE, "winget")
    if os.path.isdir(root):  # keep only the current version
        for old in os.listdir(root):
            p = os.path.join(root, old)
            if os.path.isdir(p) and old != version:
                for n in os.listdir(p):
                    os.remove(os.path.join(p, n))
                os.rmdir(p)
    d = os.path.join(root, version)
    os.makedirs(d, exist_ok=True)
    kinds = {f"{ID}.yaml": "version", f"{ID}.installer.yaml": "installer", f"{ID}.locale.en-US.yaml": "defaultLocale"}
    for name, text in files.items():
        # winget refuses manifests without this first line.
        text = f"# yaml-language-server: $schema=https://aka.ms/winget-manifest.{kinds[name]}.1.9.0.schema.json\n" + text
        with open(os.path.join(d, name), "w", encoding="utf-8", newline="\n") as f:
            f.write(text)


APP_ID = "io.github.Lucasifium.UniversalSaveEditor"
FLATPAK_ARCHES = [("x86_64", "x64"), ("aarch64", "arm64")]


def flatpak(version, hashes):
    """The Flatpak manifest: installs the release's Linux binary plus the desktop files in packaging/linux."""
    def tar(a):
        return f"UniversalSaveEditor-{version}-linux-{a}.tar.gz"

    sources = "".join(
        f"      - type: archive\n        url: {REPO}/releases/download/v{version}/{tar(a)}\n"
        f"        sha256: {hashes[tar(a)]}\n        only-arches: [{fa}]\n        strip-components: 0\n"
        for fa, a in FLATPAK_ARCHES
    )
    text = f"""app-id: {APP_ID}
runtime: org.freedesktop.Platform
runtime-version: '24.08'
sdk: org.freedesktop.Sdk
command: UniversalSaveEditor
finish-args:
  - --share=ipc
  - --socket=wayland
  - --socket=fallback-x11
  - --device=dri
  # Saves live in the Steam folders (native and Proton prefixes), the Flatpak Steam's own
  # folder, and the usual document and data folders. Files elsewhere open through the
  # file chooser, which asks the system for access to the one file picked.
  - --filesystem=~/.local/share/Steam
  - --filesystem=~/.steam
  - --filesystem=~/.var/app/com.valvesoftware.Steam
  - --filesystem=xdg-documents
  - --filesystem=xdg-data
  - --filesystem=xdg-config
modules:
  - name: universal-save-editor
    buildsystem: simple
    build-commands:
      - install -Dm755 UniversalSaveEditor /app/bin/UniversalSaveEditor
      - install -Dm644 {APP_ID}.desktop /app/share/applications/{APP_ID}.desktop
      - install -Dm644 {APP_ID}.metainfo.xml /app/share/metainfo/{APP_ID}.metainfo.xml
      - install -Dm644 {APP_ID}.mime.xml /app/share/mime/packages/{APP_ID}.xml
      - install -Dm644 {APP_ID}.png /app/share/icons/hicolor/256x256/apps/{APP_ID}.png
    sources:
{sources}      - type: dir
        path: ../linux
"""
    d = os.path.join(HERE, "flatpak")
    os.makedirs(d, exist_ok=True)
    with open(os.path.join(d, f"{APP_ID}.yml"), "w", encoding="utf-8", newline="\n") as f:
        f.write(text)
    # The release the store listing shows.
    info = os.path.join(HERE, "linux", f"{APP_ID}.metainfo.xml")
    with open(info, encoding="utf-8") as f:
        xml = f.read()
    rel = f'<releases>\n    <release version="{version}" date="{datetime.date.today().isoformat()}"/>\n  </releases>'
    xml = re.sub(r"<releases>.*?</releases>|<!-- RELEASES -->", lambda _: rel, xml, flags=re.S)
    with open(info, "w", encoding="utf-8", newline="\n") as f:
        f.write(xml)


def main():
    version = sys.argv[1] if len(sys.argv) > 1 else cargo_version()
    hashes = sums(version, sys.argv[2] if len(sys.argv) > 2 else None)
    missing = [exe(a, version) for a, _ in ARCHES if exe(a, version) not in hashes]
    if missing:
        sys.exit(f"SHA256SUMS.txt has no line for {', '.join(missing)}")
    scoop(version, hashes)
    winget(version, hashes)
    # Flatpak needs the Linux archives; older releases without them just skip it.
    if all(f"UniversalSaveEditor-{version}-linux-{a}.tar.gz" in hashes for _, a in FLATPAK_ARCHES):
        flatpak(version, hashes)
    print(f"manifests written for {version}")


if __name__ == "__main__":
    main()
