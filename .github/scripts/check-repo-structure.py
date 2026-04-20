#!/usr/bin/env python3
"""Repo structure enforcer.

Walks every tracked file in the repo and rejects any path that is not
explicitly allowed by `.repo-structure.yml` at the repo root. This is the
allowlist half of the public-repo hygiene stack; anything not declared
fails CI.

To add a new allowed path, update `.repo-structure.yml` in the same PR.
If a file does not belong in a public repo, move it to murmurlinux/internal.

Why this exists: .gitignore is defence-in-depth, not the first line of
defence. An allowlist makes every addition to the repo a conscious decision
that shows up in code review.
"""
from __future__ import annotations

import subprocess
import sys
from pathlib import Path

try:
    import pathspec
    import yaml
except ImportError as exc:
    sys.stderr.write(
        f"Missing dependency: {exc.name}. Install with: pip install pathspec pyyaml\n"
    )
    sys.exit(2)


MANIFEST = Path(".repo-structure.yml")


def load_patterns() -> list[str]:
    if not MANIFEST.exists():
        sys.stderr.write(f"::error::{MANIFEST} not found at repo root\n")
        sys.exit(2)
    data = yaml.safe_load(MANIFEST.read_text()) or {}
    patterns = data.get("allowed") or []
    if not patterns:
        sys.stderr.write(
            f"::error::{MANIFEST} has no 'allowed' patterns\n"
        )
        sys.exit(2)
    return patterns


def tracked_files() -> list[str]:
    result = subprocess.run(
        ["git", "ls-tree", "-r", "HEAD", "--name-only"],
        check=True,
        capture_output=True,
        text=True,
    )
    return [line for line in result.stdout.splitlines() if line]


def main() -> int:
    patterns = load_patterns()
    spec = pathspec.PathSpec.from_lines("gitwildmatch", patterns)

    files = tracked_files()
    disallowed = [p for p in files if not spec.match_file(p)]

    if disallowed:
        print(
            "::error::Repo structure check failed. Tracked paths not declared in "
            ".repo-structure.yml:"
        )
        for path in disallowed:
            print(
                f"::error file={path}::{path} is not declared in .repo-structure.yml"
            )
        print()
        print(
            "Every tracked path in this public repo must be declared in "
            ".repo-structure.yml."
        )
        print(
            "If a new file belongs here, add a matching pattern to the manifest "
            "in the same PR."
        )
        print(
            "If it does not belong in a public repo, move it to "
            "murmurlinux/internal."
        )
        return 1

    print(
        f"OK: all {len(files)} tracked paths match the allowlist in "
        f"{MANIFEST}."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
