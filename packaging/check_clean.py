"""Release gate: fails if a built file holds a build machine's folders, user names or e-mail addresses.

  check_clean.py <file or folder> ...

Looks inside tar.gz / zip / apk files too, as plain bytes and as UTF-16 (Windows strings).
"""
import io, os, re, sys, tarfile, zipfile

PATTERNS = [
    rb"[A-Za-z]:[\\/]Users[\\/]",
    rb"[A-Za-z]:[\\/]a[\\/]",
    rb"/home/runner",
    rb"/Users/runner",
    rb"runneradmin",
    rb"/runner/work",
    rb"[\\/]\.cargo[\\/]registry",
    rb"[\\/]\.rustup[\\/]",
    # Personal-looking addresses, except the font authors' copyright lines in the bundled Cantarell font.
    rb"(?<![A-Za-z0-9._%+-])(?!jimmac@gmail\.com)[A-Za-z0-9._%+-]+@(?:gmail|googlemail|outlook|hotmail|live|yahoo|icloud|proton|protonmail)\.(?:com|me)",
]
# Anything from the environment of the build itself (folders, account name).
literals = []
for var in ["HOME", "USERPROFILE", "GITHUB_WORKSPACE", "CARGO_HOME", "RUSTUP_HOME", "RUNNER_TEMP", "PWD"]:
    v = os.environ.get(var, "")
    if len(v) > 5 and v not in ("/", "/root"):
        literals += [v, v.replace("\\", "/")]
for var in ["USER", "USERNAME", "GITHUB_ACTOR", "GITHUB_REPOSITORY_OWNER"]:
    v = os.environ.get(var, "")
    if len(v) >= 4:
        literals += [f"/{v}/", f"\\{v}\\"]
for lit in literals:
    pat = re.escape(lit.encode())
    # The project's own public web addresses (github.com/<owner>/...) are not a build folder.
    if lit.startswith("/") and lit.endswith("/") and not lit.startswith("//"):
        pat = rb"(?<!github\.com)(?<!githubusercontent\.com)" + pat
    PATTERNS.append(pat)
REGEX = [re.compile(p, re.I) for p in PATTERNS]


def scan_bytes(label, data, problems):
    for text in (data, data.decode("utf-16-le", "ignore").encode("latin-1", "ignore")):
        for rx in REGEX:
            m = rx.search(text)
            if m:
                problems.append(f"{label}: matches {m.group(0)[:60]!r}")
    # archives inside archives, and compressed members
    if data[:4] == b"PK\x03\x04":
        try:
            with zipfile.ZipFile(io.BytesIO(data)) as z:
                for n in z.namelist():
                    scan_bytes(f"{label}!{n}", z.read(n), problems)
        except zipfile.BadZipFile:
            pass
    elif data[:2] == b"\x1f\x8b":
        try:
            with tarfile.open(fileobj=io.BytesIO(data)) as t:
                for m in t.getmembers():
                    if m.isfile():
                        scan_bytes(f"{label}!{m.name}", t.extractfile(m).read(), problems)
        except tarfile.TarError:
            pass


def main(paths):
    files = []
    for p in paths:
        if os.path.isdir(p):
            for root, _, names in os.walk(p):
                files += [os.path.join(root, n) for n in names]
        else:
            files.append(p)
    problems = []
    for f in files:
        with open(f, "rb") as fh:
            scan_bytes(f, fh.read(), problems)
    for p in problems:
        print("TRACE:", p)
    if problems:
        sys.exit(f"{len(problems)} personal or build-machine trace(s) found; not releasing")
    print(f"clean: {len(files)} file(s) checked")


main(sys.argv[1:])
