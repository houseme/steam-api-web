#!/usr/bin/env python3
# Copyright 2026 steam-api-web contributors.
# SPDX-License-Identifier: Apache-2.0
# Licensed under the Apache License, Version 2.0.
# See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

"""Validate release metadata and prepare immutable-revision workflow outputs."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parent.parent
SEMVER = re.compile(r"(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?")


def validate_version(value):
    """Accept release SemVer, including rc/beta, but reject ambiguous identifiers."""
    match = SEMVER.fullmatch(value)
    if not match:
        raise ValueError("expected MAJOR.MINOR.PATCH with an optional prerelease")
    if match[4] and any(p.isdigit() and len(p) > 1 and p.startswith("0") for p in match[4].split(".")):
        raise ValueError("numeric prerelease identifiers must not have leading zeros")
    return value


def changelog_section(text, version):
    """Extract one exact version section without borrowing another release's notes."""
    match = re.search(r"^## \[" + re.escape(version) + r"\][^\n]*\n(.*?)(?=^## \[|\Z)", text, re.M | re.S)
    if not match or not match[1].strip():
        raise ValueError("missing nonempty changelog section for " + version)
    body = match[1].strip()
    if not re.search(r"^[-*] \S", body, re.M):
        raise ValueError("release changelog must contain concrete bullet entries")
    return body


def verify_release_version(package, expected, english, chinese):
    """Check the package and both changelogs against the requested release."""
    validate_version(expected)
    if package["name"] != "steam-api-web" or package["version"] != expected:
        raise ValueError("release version does not match the steam-api-web manifest")
    if package["license"] != "Apache-2.0" or package["edition"] != "2024":
        raise ValueError("expected Apache-2.0 and Rust edition 2024")
    notes = changelog_section(english, expected)
    changelog_section(chinese, expected)
    return notes


def verify_archive(archive, checksums):
    """Verify the sole crate archive against the checksum from the CI artifact."""
    path = Path(archive)
    if not re.fullmatch(r"steam-api-web-[0-9A-Za-z.-]+\.crate", path.name):
        raise ValueError("unexpected crate archive name")
    lines = Path(checksums).read_text(encoding="utf-8").splitlines()
    if len(lines) != 1:
        raise ValueError("expected exactly one archive checksum")
    match = re.fullmatch(r"([0-9a-f]{64})  (steam-api-web-[0-9A-Za-z.-]+\.crate)", lines[0])
    if not match or match[2] != path.name:
        raise ValueError("checksum manifest does not name this archive")
    if hashlib.sha256(path.read_bytes()).hexdigest() != match[1]:
        raise ValueError("crate archive checksum mismatch")


def package_metadata():
    result = subprocess.check_output(["cargo", "metadata", "--locked", "--no-deps", "--format-version", "1"], cwd=ROOT)
    metadata = json.loads(result)
    return next(p for p in metadata["packages"] if p["name"] == "steam-api-web")


def git(*args):
    return subprocess.check_output(["git", *args], cwd=ROOT, text=True).strip()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    for name in ("check", "prepare"):
        command = commands.add_parser(name)
        command.add_argument("--version", default="")
        command.add_argument("--notes", type=Path)
        if name == "prepare":
            command.add_argument("--event", choices=["push", "workflow_dispatch"], required=True)
            command.add_argument("--ref", required=True)
            command.add_argument("--dry-run", choices=["true", "false"], required=True)
            command.add_argument("--output", type=Path, required=True)
    command = commands.add_parser("verify-archive")
    command.add_argument("--archive", type=Path, required=True)
    command.add_argument("--checksums", type=Path, required=True)
    args = parser.parse_args()
    if args.command == "verify-archive":
        verify_archive(args.archive, args.checksums)
        print("Verified CI package checksum")
        return
    version = args.version
    if args.command == "prepare" and args.event == "push":
        if not args.ref.startswith("v"):
            raise ValueError("release pushes must use v<VERSION> tags")
        version = args.ref[1:]
        if args.dry_run != "false":
            raise ValueError("tag releases cannot be silently changed into dry runs")
    package = package_metadata()
    version = validate_version(version or package["version"])
    notes = verify_release_version(package, version,
        (ROOT / "CHANGELOG.md").read_text(encoding="utf-8"),
        (ROOT / "CHANGELOG_ZH.md").read_text(encoding="utf-8"))
    if args.notes:
        args.notes.write_text("# steam-api-web v" + version + "\n\n" + notes + "\n", encoding="utf-8")
    if args.command == "prepare":
        revision = git("rev-parse", "HEAD")
        if args.dry_run == "false":
            tagged = git("rev-parse", "--verify", "refs/tags/v" + version + "^{commit}")
            if tagged != revision:
                raise ValueError("publishing requires checking out the exact version tag")
        values = {"version": version, "tag": "v" + version, "revision": revision,
                  "is_dry_run": args.dry_run, "prerelease": str("-" in version).lower()}
        with args.output.open("a", encoding="utf-8") as output:
            for key, value in values.items():
                output.write(key + "=" + value + "\n")
    print("Release metadata and bilingual changelogs verified: " + version)


if __name__ == "__main__":
    try:
        main()
    except (ValueError, subprocess.CalledProcessError) as error:
        raise SystemExit(str(error)) from None
