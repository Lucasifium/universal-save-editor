"""Packaging steps that are awkward in PowerShell; called by build_all.ps1.

  package.py tar   <binary> <out.tar.gz>         Linux: tarball keeping the executable bit
  package.py icons <png dir> <res dir>          Android launcher icons (PNGs made by build.rs)
  package.py add <apk> <path in apk>=<file> ...  put files (native libraries, classes.dex) into an APK
"""
import os, shutil, sys, tarfile, zipfile


def tar(binary, out):
    with tarfile.open(out, "w:gz") as t:
        info = t.gettarinfo(binary, arcname="SaveEditorOffline")
        info.mode = 0o755
        info.uid = info.gid = 0
        info.uname = info.gname = ""
        with open(binary, "rb") as f:
            t.addfile(info, f)


def icons(pngs, res):
    for dpi, px in [("mdpi", 48), ("hdpi", 72), ("xhdpi", 96), ("xxhdpi", 144), ("xxxhdpi", 192)]:
        d = os.path.join(res, f"mipmap-{dpi}")
        os.makedirs(d, exist_ok=True)
        shutil.copyfile(os.path.join(pngs, f"{px}.png"), os.path.join(d, "ic_launcher.png"))


def add(apk, pairs):
    with zipfile.ZipFile(apk, "a", zipfile.ZIP_DEFLATED) as z:
        for p in pairs:
            name, path = p.split("=", 1)
            z.write(path, name)


if __name__ == "__main__":
    cmd, *args = sys.argv[1:]
    {"tar": lambda: tar(*args), "icons": lambda: icons(*args), "add": lambda: add(args[0], args[1:])}[cmd]()
