#!/usr/bin/env python3
# Copyright 2026 steam-api-web contributors.
# SPDX-License-Identifier: Apache-2.0
# Licensed under the Apache License, Version 2.0.
# See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

"""Read-only crates.io audit of direct dependencies, including prereleases.

Uses cargo metadata and Python stdlib only. Prints JSON with --json; exits 1 if
an unaudited newer version exists. Does not change the manifest or lockfile.
"""
import argparse
import concurrent.futures
import json
import os
import pathlib
import re
import subprocess
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parent.parent
# reqwest's Any downcast requires the same rustls major/minor as its own dependency.
# A different preview would compile alongside it but fail TLS client construction.
EXCEPTIONS = {"rustls": "0.24.0-dev.2"}


def version_key(version):
    core, _, preview = version.partition("-")
    numbers = tuple(map(int, core.split("+", 1)[0].split(".")))
    parts = tuple((0, int(part)) if part.isdigit() else (1, part)
                  for part in re.split(r"[.-]", preview))
    return numbers + (not bool(preview), parts)


def audit(dependency):
    name, requirement = dependency
    request = urllib.request.Request(
        "https://crates.io/api/v1/crates/" + name,
        headers={"User-Agent": "steam-api-web-dependency-audit/0.1"})
    with urllib.request.urlopen(request, timeout=45) as response:
        versions = [v for v in json.load(response)["versions"] if not v["yanked"]]
    latest = max(versions, key=lambda v: version_key(v["num"]))
    selected = requirement.lstrip("^=")
    exception = (latest["num"] == EXCEPTIONS.get(name)
                 and selected == "0.23.46")
    return name, {"manifest_minimum": selected, "latest_including_prerelease": latest["num"],
                  "latest_rust_version": latest.get("rust_version"),
                  "status": "compatible_exception" if exception else
                  "current" if selected == latest["num"] else "update_required",
                  "source": "https://crates.io/api/v1/crates/" + name}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()
    env = dict(os.environ, RUSTUP_TOOLCHAIN="stable")
    metadata = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--no-deps", "--format-version", "1", "--offline"],
        cwd=ROOT, env=env))
    package = next(p for p in metadata["packages"] if p["name"] == "steam-api-web")
    dependencies = {d["name"]: d["req"] for d in package["dependencies"] if d["source"]}
    with concurrent.futures.ThreadPoolExecutor(max_workers=4) as executor:
        results = dict(executor.map(audit, sorted(dependencies.items())))
    if args.json:
        notice = {"copyright": "Copyright 2026 steam-api-web contributors.",
                  "license": "Apache-2.0", "license_url": "https://www.apache.org/licenses/LICENSE-2.0",
                  "SPDX-License-Identifier": "Apache-2.0"}
        print(json.dumps({"_metadata": notice, **results}, indent=2, sort_keys=True))
    else:
        for name, value in results.items():
            print("{}: manifest {}, latest {} [{}]".format(
                name, value["manifest_minimum"], value["latest_including_prerelease"], value["status"]))
    return int(any(v["status"] == "update_required" for v in results.values()))


if __name__ == "__main__":
    raise SystemExit(main())
