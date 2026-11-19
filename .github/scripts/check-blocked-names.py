#!/usr/bin/env python3
"""Fails if a tracked text file contains one of a few blocked names.

The names are stored only as SHA-256 hashes of their lowercase letters, so this
file does not spell them out. Text is compared after lowercasing and dropping
everything but the letters a-z, so spacing, punctuation and case don't matter.
"""
import hashlib
import re
import subprocess
import sys

# (window length, first letter, sha256 of the blocked letters)
BLOCKED = [
    (7, "l", "3747e0f8e76c3260954b5b5c6873f7e1038be9d03ed0a02585b6ac401113697b"),
    (11, "l", "ef065d1b88ddb0295bb20ceb76dc7712a97056b116868791786c7f85edf30729"),
    (11, "c", "537d4725e817b4345fb02e1e92dfe1a543908cef1c709f49cc6b464fe039aa22"),
    (6, "h", "e54cf02e2dcb10ccd5ceef7c26da528c9a61eb4b43be9f3ae14ba3720d8cb983"),
]
# The project's own account name (in links and package ids) is allowed.
ALLOWED = {(10, "df9bab9649cc97f7fba988cbade60d952ad4a4eb1a61ff99030e7ce9b866d57d")}
FIRST = {b[1] for b in BLOCKED}
NON_LETTERS = re.compile(r"[^a-z]+")


def h(s):
    return hashlib.sha256(s.encode()).hexdigest()


def hits(text):
    letters = NON_LETTERS.sub("", text.lower())
    for i, ch in enumerate(letters):
        if ch not in FIRST:
            continue
        for n, first, digest in BLOCKED:
            if ch == first and h(letters[i:i + n]) == digest:
                if any(h(letters[i:i + m]) == a for m, a in ALLOWED):
                    continue
                yield i


def main():
    files = subprocess.run(["git", "ls-files", "-z"], capture_output=True, check=True).stdout.split(b"\0")
    bad = []
    for name in filter(None, files):
        path = name.decode()
        try:
            data = open(path, "rb").read()
        except OSError:
            continue
        if b"\0" in data[:8000]:
            continue  # binary
        text = data.decode("utf-8", "replace")
        for lineno, line in enumerate(text.splitlines(), 1):
            if any(True for _ in hits(line)):
                bad.append(f"{path}:{lineno}")
    if bad:
        print("::error::A blocked name appears in:\n" + "\n".join(bad))
        return 1
    print("No blocked names found.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
