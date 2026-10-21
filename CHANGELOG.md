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

## [0.1.0] - Unreleased

### Added

- Independent async bindings for all 18 documented ISteamUserAuth, ISteamUserStats, and ISteamUser methods.
- Explicit AWS-LC TLS provider, connection reuse, bounded responses and concurrency, request pacing, timeouts, retry budgets, and shared Retry-After cooldowns.
- Conservative authentication/write replay policy, redacted structured errors, zeroized owned credential strings, and lossless SteamID/stat decoding.
- steamworks-rs ticket buffer-length handling and raw SteamID interoperability without a native SDK dependency.
- English/Chinese READMEs, design and dependency records, complete interface/module roadmap, release instructions, and Apache-2.0 file headers.
- Cross-platform reusable CI, default manual release dry runs, tag/version/changelog gates, package checksum comparison, registry verification, and GitHub release notes.
- Local protocol/resilience tests and offline release guard tests.

### Validation boundaries

- Anonymous Steam HTTPS reads have been exercised; Publisher authentication, writes, v5 ownership, and production load remain unverified.
- Three methods retain opaque results where the official response schema is incomplete. Legacy login requires prepared ciphertext.
- Rustls 0.23.46 is retained for reqwest compatibility rather than the incompatible 0.24.0-dev.2 preview.
