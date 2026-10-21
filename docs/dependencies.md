<!--
Copyright 2026 steam-api-web contributors.
SPDX-License-Identifier: Apache-2.0
Licensed under the Apache License, Version 2.0.
See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.
-->

# Dependency version record

English | [简体中文](dependencies_ZH.md)

Checked on 2026-10-21 using each crate's crates.io API, excluding yanked versions and sorting semantic versions including rc, beta, alpha, and dev previews. The shared, language-neutral [dependency-versions.json](dependency-versions.json) records the evidence.

| Direct dependency | Selected version | Purpose |
| --- | --- | --- |
| aws-lc-rs | 1.18.2 | Crypto backend and randomized jitter |
| reqwest | 0.13.5 | HTTP/1.1, pooling, and timeouts |
| rustls | 0.23.46 | Explicit per-client AWS-LC provider |
| rustls-platform-verifier | 0.7.1 | Platform certificate verification |
| serde | 1.0.229 | Typed response decoding |
| serde_json | 1.0.154 | JSON and precise integers |
| tokio | 1.54.0 | Async execution, concurrency, pacing, and timeouts |
| thiserror | 2.0.21 | Matchable error variants |
| url | 2.5.8 | URL and base URL validation |
| httpdate | 1.0.3 | Retry-After dates |
| itoa | 1.0.18 | Stack-based integer formatting for form fields |
| zeroize | 1.9.1 | Clearing owned credential strings |
| wiremock (tests) | 0.6.5 | HTTP contract and failure tests |

The highest Rustls version is **0.24.0-dev.2**, while reqwest 0.13.5 uses Rustls 0.23 and identifies ClientConfig by its concrete type in `tls_backend_preconfigured`. Compiling both versions does not make them interoperable: client construction would fail. The latest 0.23 release, **0.23.46**, is therefore the compatibility exception to selecting the highest version. Upgrade it when reqwest supports the new line. Other direct dependencies use the highest observed semantic version, with no newer rc/beta.

The normal dependency graph uses AWS-LC without ring or native-tls. The provider is explicitly configured per client rather than inherited globally. macOS Security.framework is used for certificate verification, not as a native-tls transport backend.

Cargo.lock was refreshed with online cargo update to the newest versions compatible with existing constraints and the compiler. Cargo does not automatically cross incompatible major versions or opt into unrequested transitive prereleases; forcing every transitive dependency to a globally highest version would break upstream contracts. ICU 2.3 requires Rust 1.88. Local checks used stable 1.99.1.

```sh
cargo xtask check-dependencies
cargo update
cargo tree --package steam-api-web --edges normal -i aws-lc-rs
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
```

The read-only audit uses the Rust xtask, cargo metadata, bounded worker threads, and an explicit AWS-LC HTTP client, includes new prereleases, and marks the known incompatible Rustls preview separately. Other updates or a newer preview produce a nonzero exit. It does not modify manifests, lockfiles, or snapshots. Generate a reviewed snapshot with `cargo xtask check-dependencies --json > docs/dependency-versions.json`, then inspect changes and validate. If cargo update regenerates the lockfile, retain its repository copyright/license header.

Workflow actions follow the reference project's structure while using the currently checked major versions: checkout v7, upload-artifact v7, download-artifact v8, and action-gh-release v3. Go's actionlint latest is used in CI for current workflow syntax; local validation used actionlint 1.7.12. These build tools do not change the crate's runtime dependency graph.

`xtask` is a separate publish=false workspace member. Its blocking HTTP and audit dependencies do not enter the published library dependency graph. Repository source and workflows no longer require Python.
