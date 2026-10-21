# Copyright 2026 steam-api-web contributors.
# SPDX-License-Identifier: Apache-2.0
# Licensed under the Apache License, Version 2.0.
# See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

"""Release guard regression tests; no network, Git tags or publication required."""
import hashlib
import io
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch
import urllib.error

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from release import changelog_section, validate_version, verify_archive, verify_release_version
from verify_registry import verify
import release


class ReleaseGuards(unittest.TestCase):
    def test_semver_accepts_stable_rc_and_beta(self):
        for version in ("0.1.0", "1.2.3-rc.1", "2.0.0-beta.2"):
            self.assertEqual(validate_version(version), version)
        for version in ("v1.2.3", "1.2", "01.2.3", "1.2.3-01", "1.2.3\ninjected=true", "$(id)"):
            with self.assertRaises(ValueError):
                validate_version(version)

    def test_exact_changelog_section_is_required(self):
        text = "## [Unreleased]\n- Future work\n## [0.1.0] - Unreleased\n### Added\n- Current change\n## [0.0.1]\n- Older change\n"
        self.assertEqual(changelog_section(text, "0.1.0"), "### Added\n- Current change")
        for version in ("0.2.0", "0.1"):
            with self.assertRaises(ValueError):
                changelog_section(text, version)
        with self.assertRaises(ValueError):
            changelog_section("## [0.1.0]\n", "0.1.0")

    def test_manifest_and_both_languages_must_match(self):
        package = {"name": "steam-api-web", "version": "0.1.0", "edition": "2024", "license": "Apache-2.0"}
        notes = "## [0.1.0]\n- Added initial bindings\n"
        verify_release_version(package, "0.1.0", notes, notes)
        with self.assertRaises(ValueError):
            verify_release_version(package, "0.2.0", notes, notes)
        with self.assertRaises(ValueError):
            verify_release_version(package, "0.1.0", notes, "## [Unreleased]\n- Work\n")

    def test_archive_checksum_and_name_must_match(self):
        with tempfile.TemporaryDirectory() as directory:
            archive = Path(directory) / "steam-api-web-0.1.0.crate"
            archive.write_bytes(b"reviewed archive")
            checksum = Path(directory) / "SHA256SUMS"
            checksum.write_text(hashlib.sha256(archive.read_bytes()).hexdigest() + "  " + archive.name + "\n")
            verify_archive(archive, checksum)
            archive.write_bytes(b"different archive")
            with self.assertRaises(ValueError):
                verify_archive(archive, checksum)

    def test_prepare_dry_run_records_revision_without_requiring_a_tag(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            notes = "## [0.1.0]\n- Prepared change\n"
            for name in ("CHANGELOG.md", "CHANGELOG_ZH.md"):
                (root / name).write_text(notes, encoding="utf-8")
            package = {"name": "steam-api-web", "version": "0.1.0", "edition": "2024", "license": "Apache-2.0"}
            output = root / "outputs"
            argv = ["release.py", "prepare", "--version", "0.1.0", "--event", "workflow_dispatch", "--ref", "main", "--dry-run", "true", "--output", str(output)]
            with patch.object(release, "ROOT", root), patch.object(release, "package_metadata", return_value=package), patch.object(release, "git", return_value="a" * 40) as git, patch.object(sys, "argv", argv):
                release.main()
            self.assertEqual(git.call_count, 1)
            self.assertIn("is_dry_run=true\n", output.read_text())
            self.assertIn("revision=" + "a" * 40, output.read_text())

    def test_prepare_publication_rejects_a_different_tagged_commit(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            notes = "## [0.1.0]\n- Prepared change\n"
            for name in ("CHANGELOG.md", "CHANGELOG_ZH.md"):
                (root / name).write_text(notes, encoding="utf-8")
            package = {"name": "steam-api-web", "version": "0.1.0", "edition": "2024", "license": "Apache-2.0"}
            output = root / "outputs"
            argv = ["release.py", "prepare", "--version", "0.1.0", "--event", "workflow_dispatch", "--ref", "main", "--dry-run", "false", "--output", str(output)]
            with patch.object(release, "ROOT", root), patch.object(release, "package_metadata", return_value=package), patch.object(release, "git", side_effect=["a" * 40, "b" * 40]), patch.object(sys, "argv", argv):
                with self.assertRaises(ValueError):
                    release.main()
            self.assertFalse(output.exists())

    def test_tag_push_uses_tag_version_even_if_a_different_version_is_supplied(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            notes = "## [0.1.0]\n- Prepared change\n"
            for name in ("CHANGELOG.md", "CHANGELOG_ZH.md"):
                (root / name).write_text(notes, encoding="utf-8")
            package = {"name": "steam-api-web", "version": "0.1.0", "edition": "2024", "license": "Apache-2.0"}
            argv = ["release.py", "prepare", "--version", "0.1.0", "--event", "push", "--ref", "v0.2.0", "--dry-run", "false", "--output", str(root / "outputs")]
            with patch.object(release, "ROOT", root), patch.object(release, "package_metadata", return_value=package), patch.object(sys, "argv", argv):
                with self.assertRaises(ValueError):
                    release.main()

    def test_registry_requires_exact_checksum_not_just_version_existence(self):
        with tempfile.TemporaryDirectory() as directory:
            archive = Path(directory) / "steam-api-web-0.1.0.crate"
            archive.write_bytes(b"reviewed archive")
            metadata = {"version": {"crate": "steam-api-web", "num": "0.1.0", "yanked": False,
                        "checksum": hashlib.sha256(archive.read_bytes()).hexdigest()}}
            response = lambda: io.BytesIO(json.dumps(metadata).encode())
            with patch("urllib.request.urlopen", side_effect=lambda *a, **k: response()):
                verify("steam-api-web", "0.1.0", archive, attempts=1)
            metadata["version"]["checksum"] = "0" * 64
            with patch("urllib.request.urlopen", side_effect=lambda *a, **k: response()):
                with self.assertRaises(ValueError):
                    verify("steam-api-web", "0.1.0", archive, attempts=1)

    def test_registry_not_found_is_bounded_and_permanent_errors_fail(self):
        with tempfile.TemporaryDirectory() as directory:
            archive = Path(directory) / "steam-api-web-0.1.0.crate"
            archive.write_bytes(b"archive")
            with patch("urllib.request.urlopen", side_effect=urllib.error.HTTPError("url", 404, "missing", {}, None)) as call:
                with self.assertRaises(ValueError):
                    verify("steam-api-web", "0.1.0", archive, attempts=2, interval=0)
                self.assertEqual(call.call_count, 2)
            with patch("urllib.request.urlopen", side_effect=urllib.error.HTTPError("url", 403, "forbidden", {}, None)) as call:
                with self.assertRaises(urllib.error.HTTPError):
                    verify("steam-api-web", "0.1.0", archive, attempts=2, interval=0)
                self.assertEqual(call.call_count, 1)


if __name__ == "__main__":
    unittest.main()
