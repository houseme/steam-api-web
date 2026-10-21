<!--
Copyright 2026 steam-api-web contributors.
SPDX-License-Identifier: Apache-2.0
Licensed under the Apache License, Version 2.0.
See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.
-->

# Changelog

English | [简体中文](CHANGELOG_ZH.md)

All notable user-facing changes are recorded here. Versions follow Semantic Versioning. A prepared entry marked Unreleased is not evidence of publication.

## [Unreleased]

### Planned

- Extend coverage according to the complete Web API and native SDK interoperability roadmap.
- Validate Publisher-only contracts, deletion cursors, write results, and ownership v5 with authorized credentials.

## [0.2.0] - Unreleased

### Added

- All 39 P1 methods for API discovery, app administration, players, store, reviews, news, game servers and leaderboards; 57 methods across 12 interfaces in total.
- Service input_json, review filtering/pagination, raw leaderboard bytes, redacted server tokens, and per-method credential/retry policies.
- Distinct HTTP 200/201 build-switch outcomes, empty-body acceptance only for pending-confirmation 201, and shared EResult validation.
- Rust xtask replaces every Python maintenance/release script and inline workflow Python while retaining version/tag/changelog/archive/registry guards.
- Explicit package allowlist keeps tooling, workflows, integration tests, planning documents and reference JSON on GitHub; archive contents and independent builds are verified.

### Validation boundaries

- New methods have local protocol tests; no live Steam Publisher administrative writes, credentialed integration or production load evidence was obtained.
- Version 0.2.0 is unreleased. The historical 0.1.0 tag and published archive are unchanged.

## [0.1.0] - 2026-10-21

### Added

- Independent async bindings for all 18 documented ISteamUserAuth, ISteamUserStats, and ISteamUser methods.
- Explicit AWS-LC TLS provider, connection reuse, bounded responses and concurrency, request pacing, timeouts, retry budgets, and shared Retry-After cooldowns.
- Conservative authentication/write replay policy, redacted structured errors, zeroized owned credential strings, and lossless SteamID/stat decoding.
- steamworks-rs ticket buffer-length handling and raw SteamID interoperability without a native SDK dependency.
- English/Chinese READMEs, design and dependency records, complete interface/module roadmap, release instructions, and Apache-2.0 file headers.
- Cross-platform reusable CI, default manual release dry runs, tag/version/changelog gates, package checksum comparison, registry verification, and GitHub release notes.
- Local protocol/resilience tests and offline release guard tests.

### Validation boundaries

- Stable Linux, macOS, and Windows CI, documentation, workflow checks, and distributable crate verification passed before tagging.
- Anonymous Steam HTTPS reads have been exercised; Publisher authentication, writes, v5 ownership, and production load remain unverified.
- Three methods retain opaque results where the official response schema is incomplete. Legacy login requires prepared ciphertext.
- Rustls 0.23.46 is retained for reqwest compatibility rather than the incompatible 0.24.0-dev.2 preview.
