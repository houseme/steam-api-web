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

## [0.3.0] - Unreleased

### Added

- All 66 P2 methods for Cloud, inventory, asset economy, trades, market, remote UGC, Workshop search/voting/moderation and payment configuration; 123 methods across 24 interfaces in total.
- Cloud OAuth, strict x-eresult validation, redacted signed transfer instructions and explicit batch/file-commit/completion stages; callers retain file-byte transport responsibility.
- Inventory request IDs, typed dynamic properties and concurrency timestamps, trade pagination, Workshop cursors/revisions and the verbatim v0001 item-definition path.
- A read-only Cloud example, complete protocol/error/replay tests and bilingual P2 contract documentation.

### Changed

- Shared response validation accepts checked object-valued result envelopes while retaining strict scalar EResult handling; mutations still never replay, including side-effecting GETs.
- Consolidate response contracts in ResponsePolicy and eliminate temporary scalar form strings using borrowed text and stack-formatted integers; add an offline benchmark against the reviewed 0.3 encoder.
- Repository guards compare coverage/version, public modules and P2 method names. Maintenance remains Rust; tests, benchmarks, tools, workflows and reference material stay outside the published package.

### Validation boundaries

- No live OAuth/Publisher credentials or Cloud, inventory, trade, listing or Workshop writes were used. Actual authorization, deduplication and persistence outcomes remain unverified.
- Version 0.3.0 is unreleased; existing 0.1.0 and 0.2.0 tags are unchanged.

## [0.2.0] - 2026-10-21

### Added

- All 39 P1 methods for API discovery, app administration, players, store, reviews, news, game servers and leaderboards; 57 methods across 12 interfaces in total.
- Service input_json, review filtering/pagination, raw leaderboard bytes, redacted server tokens, and per-method credential/retry policies.
- Distinct HTTP 200/201 build-switch outcomes, empty-body acceptance only for pending-confirmation 201, and shared EResult validation.
- Rust xtask replaces every Python maintenance/release script and inline workflow Python while retaining version/tag/changelog/archive/registry guards.
- Explicit package allowlist keeps tooling, workflows, integration tests, planning documents and reference JSON on GitHub; archive contents and independent builds are verified.

### Changed

- Centralize endpoint policies and direct form/Service JSON encoding; move typed response data and forward opaque JSON without rebuilding it, preserving public APIs and resilience behavior.
- Add an offline optimized codec benchmark against the pre-refactor algorithms, with bilingual review findings, reproducible measurements and ownership regression tests.

### Validation boundaries

- New methods have local protocol tests; no live Steam Publisher administrative writes, credentialed integration or production load evidence was obtained.
- Tag creation and registry publication are separate steps; the registry and release workflow determine publication status. The historical 0.1.0 tag and published archive are unchanged.

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
