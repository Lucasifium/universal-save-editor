"""Writes the Windows package-manager manifests for a published release.

  python3 -I packaging/manifests.py [version] [SHA256SUMS.txt]

With no arguments it uses the version in Cargo.toml and downloads that release's SHA256SUMS.txt.
Output:
  packaging/scoop/universal-save-editor.json      Scoop (install straight from its raw URL)
  packaging/winget/<version>/*.yaml               winget (copy into a pull request to
                                                  microsoft/winget-pkgs under
                                                  manifests/l/Lucasifium/UniversalSaveEditor/<version>/)
"""
import datetime, json, os, re, sys, urllib.request

REPO = "https://github.com/Lucasifium/universal-save-editor"
SITE = "https://lucasifium.github.io/universal-save-editor/"
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


def exe(arch):
    return f"UniversalSaveEditor-windows-{arch}.exe"


def scoop(version, hashes):
    base = f"{REPO}/releases/download/v{version}"
    auto = f"{REPO}/releases/download/v$version"
    m = {
        "version": version,
        "description": SHORT,
        "homepage": SITE,
        "license": "GPL-3.0-or-later",
        "architecture": {
            key: {"url": f"{base}/{exe(a)}#/UniversalSaveEditor.exe", "hash": hashes[exe(a)]}
            for a, key in ARCHES
        },
        "bin": "UniversalSaveEditor.exe",
        "shortcuts": [["UniversalSaveEditor.exe", "Universal Save Editor"]],
        "checkver": "github",
        "autoupdate": {
            "architecture": {
                key: {"url": f"{auto}/{exe(a)}#/UniversalSaveEditor.exe"} for a, key in ARCHES
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
            f"- Architecture: {a}\n  InstallerUrl: {REPO}/releases/download/v{version}/{exe(a)}\n"
            f"  InstallerSha256: {hashes[exe(a)].upper()}\n"
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


def main():
    version = sys.argv[1] if len(sys.argv) > 1 else cargo_version()
    hashes = sums(version, sys.argv[2] if len(sys.argv) > 2 else None)
    missing = [exe(a) for a, _ in ARCHES if exe(a) not in hashes]
    if missing:
        sys.exit(f"SHA256SUMS.txt has no line for {', '.join(missing)}")
    scoop(version, hashes)
    winget(version, hashes)
    print(f"manifests written for {version}")


if __name__ == "__main__":
    main()
