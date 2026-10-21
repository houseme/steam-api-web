#!/usr/bin/env python3
# Copyright 2026 steam-api-web contributors.
# SPDX-License-Identifier: Apache-2.0
# Licensed under the Apache License, Version 2.0.
# See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

"""Confirm that crates.io contains the exact archive validated by release CI."""
import argparse
import hashlib
import json
from pathlib import Path
import time
import urllib.error
import urllib.request
from release import validate_version


def verify(package, version, archive, attempts=20, interval=3):
    if package != "steam-api-web":
        raise ValueError("this release verifier only accepts steam-api-web")
    validate_version(version)
    expected = hashlib.sha256(Path(archive).read_bytes()).hexdigest()
    request = urllib.request.Request("https://crates.io/api/v1/crates/" + package + "/" + version,
                                    headers={"User-Agent": "steam-api-web-release-verifier"})
    for attempt in range(attempts):
        try:
            with urllib.request.urlopen(request, timeout=15) as response:
                metadata = json.load(response)["version"]
            if metadata.get("yanked") or metadata.get("checksum") != expected:
                raise ValueError("published version is yanked or its checksum differs from the validated archive")
            if metadata.get("num") != version or metadata.get("crate") != package:
                raise ValueError("registry returned a different package or version")
            return
        except urllib.error.HTTPError as error:
            if error.code not in (404, 429, 500, 502, 503, 504):
                raise
        except (urllib.error.URLError, TimeoutError):
            pass
        if attempt + 1 < attempts:
            time.sleep(interval)
    raise ValueError("crates.io did not expose the validated archive before the retry budget expired")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("package")
    parser.add_argument("version")
    parser.add_argument("--archive", type=Path, required=True)
    args = parser.parse_args()
    verify(args.package, args.version, args.archive)
    print("Verified crates.io version and archive checksum")


if __name__ == "__main__":
    try:
        main()
    except (ValueError, urllib.error.URLError) as error:
        raise SystemExit(str(error)) from None
