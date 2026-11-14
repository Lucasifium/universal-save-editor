"""CI helper: exports CARGO_ENCODED_RUSTFLAGS so no runner folder ends up in a build.

Rust writes source paths into the program (panic messages, debug paths). Replace the runner's
folders with neutral names, the same way build_all.ps1 does on a PC. When several prefixes match,
the last one given wins, so the most specific folder goes last.
"""
import os

SEP = "\x1f"
prefixes = []
home = os.environ.get("HOME") or os.environ.get("USERPROFILE") or ""
os.environ.setdefault("CARGO_HOME", os.path.join(home, ".cargo"))
os.environ.setdefault("RUSTUP_HOME", os.path.join(home, ".rustup"))
for var, name in [("USERPROFILE", "~"), ("HOME", "~"), ("RUSTUP_HOME", "/rustup"),
                  ("CARGO_HOME", "/cargo"), ("GITHUB_WORKSPACE", "/build")]:
    value = os.environ.get(var)
    if value:
        prefixes.append(f"--remap-path-prefix={value}={name}")
# Windows: link the C runtime statically so the exe runs without the VC++ redistributable.
crt = ["-C", "target-feature=+crt-static"] if os.name == "nt" else []
flags = SEP.join(["-A", "linker_messages"] + crt + prefixes)
with open(os.environ["GITHUB_ENV"], "a", encoding="utf-8") as f:
    f.write(f"CARGO_ENCODED_RUSTFLAGS<<EOF_FLAGS\n{flags}\nEOF_FLAGS\n")
