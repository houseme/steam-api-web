<!--
Copyright 2026 steam-api-web contributors.
SPDX-License-Identifier: Apache-2.0
Licensed under the Apache License, Version 2.0.
See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.
-->

# CI and release operations

English | [简体中文](releases_ZH.md)

The workflows adapt the structure of [rustfs-mimalloc CI](https://github.com/houseme/rustfs-mimalloc/blob/main/.github/workflows/ci.yml) and its [release workflow](https://github.com/houseme/rustfs-mimalloc/blob/main/.github/workflows/release.yml) to a single HTTP client crate. Allocator-specific feature matrices, native profiling and multi-crate publish ordering are intentionally replaced with this crate's actual contracts.

## CI

CI runs on main pushes, pull requests, manual dispatch, and reusable workflow calls. A release passes an exact source revision to the reusable workflow. Every checkout uses that revision with credentials disabled.

- Stable Linux, macOS and Windows build all targets and run all Rust tests, including doctests and the isolated provider process.
- The lint job runs fmt, strict Clippy, bilingual/link/license/inventory checks, AWS-LC graph checks, offline Python release tests, and actionlint.
- The documentation job rejects rustdoc warnings.
- Packaging waits for every preceding gate, runs cargo package with its independent build verification, and uploads the crate plus SHA256SUMS as the steam-api-web-package artifact.

Existing security audit and SARIF workflows remain separate. CI concurrency cancels superseded development runs; tag and manual release validation avoid automatic cancellation. Crate availability on crates.io or successful remote CI is not implied by committing these definitions.

## Prepare a release

1. Set Cargo.toml to the intended SemVer and refresh Cargo.lock when dependency changes require it.
2. Add nonempty matching version sections to both CHANGELOG.md and CHANGELOG_ZH.md. Replace the prepared entry's Unreleased date before publication when the release date is known.
3. Update both READMEs and paired docs; keep code comments in English. Retain copyright/SPDX headers when generated source metadata is refreshed.
4. Run local checks and inspect the package contents. Before publishing, ensure CI passes for the exact revision.

```sh
python3 scripts/release.py check --version 0.1.0
python3 scripts/check_repository.py
python3 -m unittest discover -s scripts/tests -v
cargo fmt --all --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --all-features --locked
cargo package --locked
```

Local work-in-progress packaging can use --allow-dirty for inspection. CI publication requires a clean checkout and does not use that flag.

## Manual dry run

Open the Release workflow, select the source branch, enter its manifest version, and leave dry_run enabled. The default is true. A dry run validates both changelogs, all reusable CI gates and the archive. It does not need a registry token and neither publishes nor creates a GitHub release. Download the artifact to inspect the prepared package and checksum.

## Actual publication

Create a repository secret named CARGO_REGISTRY_TOKEN with permission to publish steam-api-web. GitHub release creation uses the job's GITHUB_TOKEN with contents: write only in the release job; other jobs default to contents: read. This configuration does not create credentials or trigger publication.

Publish by pushing an immutable v<VERSION> tag after reviewing the exact commit, or manually dispatch with dry_run disabled and an existing matching tag. Example commands, to be run only when publication is intended:

```sh
git tag v0.1.0
git push origin v0.1.0
```

Tag pushes resolve the version from the tag; manual publication checks out the existing requested tag. Preparation validates strict SemVer, Cargo package version/license/edition, both changelogs, and that HEAD is the exact tagged commit. Leading-zero versions and command-like inputs are rejected. rc/beta versions are supported and create prereleases.

After reusable CI passes, publication downloads its archive and verifies the stored checksum. A fresh cargo package at the same revision must byte-match that reviewed artifact. cargo publish then uploads from the clean revision. A failed upload is not treated as success merely because a version exists: the verifier polls crates.io for the exact version and archive SHA-256, rejecting different or yanked content. The GitHub release runs only after registry verification, uses the exact English changelog section, and attaches the validated crate and checksum.

Release runs are serialized and are not automatically canceled. Retrying an interrupted run is safe only when the existing published archive matches the newly validated artifact. crates.io publication is irreversible; workflow implementation and offline tests do not exercise a real upload.

## File headers and machine data

Repository Rust, Python, TOML, YAML, Markdown, ignore/lock files and LICENSE include English copyright and Apache-2.0 notices. Markdown uses HTML comments. Python preserves its interpreter shebang; Cargo.lock may retain Cargo's generated banner before its legal notice. JSON has a first _metadata object containing copyright and license fields because JSON does not support comments. api-inventory.json and dependency-versions.json are shared language-neutral evidence, while every prose document has an English/Chinese pair.
