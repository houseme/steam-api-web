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

## [0.5.0] - Unreleased

### Added

- Complete GetPlayersBanned time filtering and cursor pagination while preserving its existing convenience method; add six protocol regressions.
- Add 33 standalone examples for 38 total programs covering all 33 interface families, compile-checked English call examples for all 160 public async methods, and enforced public/error documentation.
- Rust API completeness auditing for 735 parameter rows and 167 binding/fixture mappings, with 53 discovery-only candidates and separate response/acceptance states; add bilingual feature matrices and P5 native interoperability design.
- Public User-key ticket and conditional build-confirmation protocol fixtures; CI retains offline audit reports. HTML5 parsing is confined to the unpublished xtask.
- All six P4 methods for financial reports, PC Cafe site activity/playtime and broadcast frames; the documented snapshot now binds all 33 interfaces and 167 routes (157 explicit, ten inherited Sandbox routes).
- A dedicated Financial API Group key kind and isolated access policy, lossless report money/cursors, conditional sales/wishlist models and a read-only change-batch example.
- Site models with private-field Debug redaction, response-local game IDs, future license types and exact UTC interval comparison; explicit single-send broadcast frames.
- Rust coverage reports with complete module/method/path mappings and inherited-signature checks, saved as a separate GitHub CI artifact.

### Changed

- Reuse single-chunk HTTP bodies and buffer only multiple chunks; keep logical length limits, response validation and cancellation behavior.
- Stream CSV scalars and legacy asset keys into the shared form encoder, removing remaining collect/join and formatted-key allocations; add bounded ownership tests and drift-gated CPU benchmarks.
- Centralize all direct dependency versions and shared options in the root workspace manifest, preserving member-specific features and standalone package builds.
- Update Tokio to 1.54.1 after the registry withdrew 1.54.0; refresh the lockfile and dependency audit record.
- Fix valid 128-bit statistics being rejected during Value-to-typed decoding when downstream arbitrary_precision is enabled; delegate range and precision behavior to Number.
- Decode statistic numbers and borrowed text directly while preserving floats, negative zero and downstream arbitrary_precision compatibility; add complete-response CPU benchmarks and Linux feature-unification checks.
- Consolidate exact signed/unsigned integer decoding across reports and IDs in one checked, borrowed-text Visitor, removing untagged branch retries and duplicate optional wrappers; see the repository's bilingual 0.5 review for reproducible scalar measurements.
- Share Gregorian/UTC validation across MicroTxn, financial dates and site reporting, retaining original date spelling and arbitrary fractional precision.
- Keep repository maintenance in Rust and preserve the package allowlist; tests, benchmarks, tooling, workflows, coverage data and prose docs remain on GitHub.

### Validation boundaries

- No financial account data, Site credentials or live broadcast operations were used. Financial permissions, Site response wrapping/array omission and production availability still require credentialed integration.
- Version 0.5.0 is unreleased. Existing tags and registry versions remain separate from this development work.

## [0.4.0] - 2026-10-22

### Added

- All 38 P3 routes: ten production billing and ten inherited Sandbox routes, eight anti-cheat/community methods and ten notification/lobby methods; 161 routes across 30 interfaces in total.
- Explicit billing environments, exact integer amounts/IDs, recurring agreements and reconciliation cursors. Sandbox failures never fall back to production, and every mutation remains single-send.
- Explicit result=OK billing acknowledgements and endpoint-scoped VAC readiness responses that preserve pending/unverified states without hiding errors.
- Notification sessions, explicit ban/report/lobby operations, a read-only Sandbox query example, local protocol tests and bilingual P3 contract documentation.

### Changed

- Coverage guards distinguish explicit signatures from inherited Sandbox counterparts.
- Preserve explicit VAC API errors before requiring readiness fields, using one scoped error traversal.
- Move single-send URL/body ownership and the final read-attempt URL into reqwest; centralize concurrency, pacing and metrics at the send boundary.
- Indexed form keys write directly into the final buffer, and each client caches fixed endpoint URLs without retaining queries or credentials.
- The README keeps a quick start, coverage summary and documentation index; detailed configuration, reliability, data handling and steamworks-rs interop notes moved to the bilingual usage guide in docs.
- Maintenance remains Rust. Tests, benchmarks, tools, workflows, documents and reference JSON stay on GitHub outside the distributable crate.

### Validation boundaries

- No live billing/anti-cheat authorization or real payments, refunds, bans, reports, notifications or lobby writes were used. Permissions, callbacks and persistence require independent validation.
- Existing version tags remain unchanged; registry publication completes only after the release workflow's registry checksum verification succeeds.

## [0.3.0] - 2026-10-21

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
- Tag creation and registry publication are separate steps; publication status is determined by the registry and release workflow. Existing 0.1.0 and 0.2.0 tags are unchanged.

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
