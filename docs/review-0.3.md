<!--
Copyright 2026 steam-api-web contributors.
SPDX-License-Identifier: Apache-2.0
Licensed under the Apache License, Version 2.0.
See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.
-->

# 0.3 transport review and request benchmarks

English | [简体中文](review-0.3_ZH.md)

Reviewed revision: `2077350d9b8c6de4aad97e97f4f96f3fd07bf2cd`, including P2 and its shared transport. Its [platform CI](https://github.com/houseme/steam-api-web/actions/runs/39606087427) passed before this refactor. The interface modules remain thin wrappers around one client and do not need wholesale replacement. Two informational performance/maintenance issues were confirmed; no blocking protocol failure was established.

## Verified observations and changes

- **Temporary form values — Informational.** At [request.rs:110](https://github.com/houseme/steam-api-web/blob/2077350d9b8c6de4aad97e97f4f96f3fd07bf2cd/src/request.rs#L110), every scalar passed through ToString. Borrowed text, OAuth tokens, integers and Steam IDs acquired temporary heap strings before being copied into the final form. Private FormValue implementations now borrow text and use an itoa stack buffer for integers. Unescaped ASCII prefixes copy directly; the existing byte encoder handles remaining bytes, including non-UTF-8 rawbinary. Public signatures and wire encodings remain compatible.
- **Split response contracts — Informational.** BodyPolicy in request.rs, header checks/JSON parsing at [client.rs:678](https://github.com/houseme/steam-api-web/blob/2077350d9b8c6de4aad97e97f4f96f3fd07bf2cd/src/client.rs#L678), and business decoding in response.rs shared responsibility for a reply. ResponsePolicy now owns header validation, permitted empty bodies and JSON root parsing. The client retains bounded I/O, deadlines, concurrency, pacing and retries. Header validation still precedes body reads.

Policies distinguish ordinary JSON, optional HTTP 201 build-confirmation JSON, Cloud JSON with a successful result header, and bodyless Cloud HTTP 200 acknowledgements. Acknowledgements with bodies now fail directly in the policy; callers previously received a Decode rejection when decoding as unit. Strict headers, business errors, redaction and single-send authentication/mutation behavior remain covered.

The direct itoa dependency is 1.0.18, already present in the reviewed dependency graph. Adding a direct edge introduces no new resolved package. All 13 direct dependencies were audited including prereleases; the documented Rustls exception remains. See [dependency evidence](dependencies.md).

## Reproduce the measurements

```sh
cargo bench --bench request --locked --offline
```

The benchmark compiles the production encoder and retains the reviewed encoder in repository-only support code. Both receive identical inputs. Full wire equivalence is checked before timing, including all 256 raw byte values. Inputs are prepared outside timing; output destruction is included.

Each case warms up 100 times and runs nine A/B/B/A rounds, reporting medians of 18 samples per implementation. Baseline drift above 10% fails the run. The following optimized-build run with stable Rust 1.99.1 had less than 2% drift in every case:

| Case | Reviewed ns/op | Current ns/op | Observed time reduction |
| --- | ---: | ---: | ---: |
| Scalar form | 122 | 55 | 55.2% |
| Cloud form with approximately 4 KiB OAuth token | 2,863 | 1,790 | 37.5% |
| 100 indexed IDs | 4,867 | 3,047 | 37.4% |
| 100 JSON definitions | 107,610 | 106,274 | 1.2%, effectively unchanged |

These are local encoder CPU measurements, not end-to-end Steam latency or proof of global optimality. Small differences near noise do not prove improvement. Requests still allocate their final buffer, and indexed key formatting may allocate. No allocation counter or production load measurement was used.

A generic URL byte-iterator replacement was measured before selecting the implementation: it slowed JSON definitions by about 38%. Keeping one byte encoder and accelerating borrowed scalar prefixes avoids that regression; JSON retains its established direct-to-form path.

## Verification and distribution

130 workspace tests pass, including integer extremes, UTF-8/control-byte wire equivalence, the response-policy matrix and existing P0/P1/P2 protocol tests. Formatting, strict Clippy, warning-free rustdoc, archive policy and independent archive builds are checked separately. No live OAuth/Publisher writes were made.

Coverage remains 123 methods in 24 interfaces. Rust xtask and AWS-LC remain in place; no Python is introduced. The new benchmark and this review stay on GitHub through the existing allowlist. The distributable retains its 42-file source/example/license/README/changelog boundary.
