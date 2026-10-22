<!--
Copyright 2026 steam-api-web contributors.
SPDX-License-Identifier: Apache-2.0
Licensed under the Apache License, Version 2.0.
See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.
-->

# Global request-path review and measured refactor

English | [简体中文](review-global_ZH.md)

Reviewed on 2026-10-22 against merged main `9d2cab1`, including shared transport,
request encoding, response validation, numeric models and the completed API/example
work. Rust edition 2024, stable 1.99.1; dependencies remain inherited from the root
workspace. All 33 interface families and 160 public async methods retain their APIs.

## Conclusion and scope

The shared architecture is coherent: endpoint adapters select a fixed contract,
one client pipeline owns access checks, deadlines, concurrency, pacing and replay,
and response policies own business validation. The review found two remaining
sources of avoidable work: older list conversions bypassed the shared scalar
encoder, and response collection copied every body even when it contained one
transport chunk. Both now use a shared implementation. These are informational performance findings: the original semantics were correct, and the measured extra work justifies the targeted refactor.

| Shared path | Decision and evidence |
| --- | --- |
| Public endpoint adapters and compatibility helpers | Retain their typed parameters and thin futures. They reuse the common client without allocating boxed futures or creating another runtime. |
| `call_with` / `execute` / `attempt` | Retain key-kind isolation, the query-free URL cache, pooled AWS-LC client, operation budget, semaphore, pacing and cooldown. The refactor adds no retry or host-fallback policy. |
| Response policy and typed/opaque decoding | Retain the complete standard error-envelope checks and owned model decoding. The existing policy-specific acknowledgement rules remain enforced. |
| CSV and numbered fields | Route all affected users/app filters and asset fields through the existing final form buffer; remove intermediate owned scalar strings, collect/join vectors and per-asset formatted keys. |
| HTTP body collection | Retain one owned chunk directly; concatenate only when a second nonempty chunk arrives, preserving streamed size checks and bounded capacity hints. |

## Request encoding

[`CommaSeparated`](../src/request.rs) implements the existing private `FormValue`
contract for borrowed scalar slices. It writes each value once into the final form
and writes `%2C` between values, exactly matching encoding a comma-joined string.
Empty elements, Unicode, embedded NUL, percent signs and full-width integers retain
the existing wire representation. Publisher application types use the same scalar
contract through their existing string mapping.

The affected production callers are profile/ban batches, app price queries and
publisher application filters. Legacy asset purchases also use the existing
`push_suffixed` helper for `assetid0`/`assetquantity0`, removing two temporary
`format!` strings per asset. Validation, field order and endpoint policies remain
unchanged. Generic scalar/form tests and existing endpoint protocol tests verify
the resulting bytes and semantic query/form values.

## Bounded response ownership

The private [`ResponseBody`](../src/body.rs) has three states:

| State | Next nonempty chunk |
| --- | --- |
| Empty | Take ownership of the chunk. |
| Single | Allocate a contiguous buffer, copy both chunks and release the first chunk's ownership. |
| Buffered | Extend the existing buffer. |

Every append checks the combined byte length before changing state or copying.
Empty chunks preserve the current state. A length hint is reserved only when
buffering becomes necessary and retains the existing 64 KiB cap. Header and
streamed response-length checks still produce the same public error type.
Parsing, business validation and DTO decoding consume the same complete bytes.

The single-chunk path eliminates a separate body `Vec` allocation and payload
copy. Multiple chunks still require concatenation; the first two transport chunks
and their destination can coexist briefly during that transition. The configured
body limit bounds logical bytes, as before; it is not a total process-memory cap.
The collector owns no tasks, credentials, connections or shared synchronization.

## Reproducible measurements

```sh
cargo bench --bench request --locked --offline -- --lists-only
cargo bench --bench codec --locked --offline -- --body-only
```

The baselines reproduce the algorithms from `9d2cab1`; the candidate code is
included directly from the production source modules. Fixtures are constructed
outside timing. Wire bytes and decoded models are checked for equality first.
Each case uses nine A/B/B/A rounds, medians from 18 samples per implementation,
and a 10% baseline-drift rejection gate. All reported cases passed; observed
absolute baseline drift was at most 1.4%. Timings include output destruction.

These are local optimized CPU measurements on a shared development host, with an
isolated Cargo target; this task serialized builds and timed runs. Request cases
exclude validation and HTTP. Body cases consume identical owned `Arc<[u8]>`
fixture chunks in both paths, excluding actual HTTP arrival and transport-buffer
allocation. Complete-response cases include body assembly, JSON parsing, error
checks, envelope selection, typed decoding and output destruction. Network/TLS
latency, Steam quotas and production throughput are separate measurements.

| Encoding case | Baseline ns/op | Refactored ns/op | Less CPU time | Baseline drift |
| --- | ---: | ---: | ---: | ---: |
| One full-width SteamID | 70 | 27 | 61.2% | -1.4% |
| 100 full-width SteamIDs | 3,750 | 757 | 79.8% | -0.4% |
| Six publisher app types | 89 | 54 | 39.8% | +0.3% |
| 100 legacy asset lines | 5,922 | 2,666 | 55.0% | -1.4% |

| Response case | Baseline ns/op | Refactored ns/op | Less CPU time | Baseline drift |
| --- | ---: | ---: | ---: | ---: |
| 64 B single-chunk assembly | 21 | 8 | 62.4% | +0.6% |
| 4 KiB single-chunk assembly | 73 | 5 | 92.7% | -0.6% |
| 64 KiB single-chunk assembly | 672 | 4 | 99.5% | -0.2% |
| 64 KiB / four chunks, known length | 919 | 849 | 7.5% | +0.4% |
| 64 KiB / four chunks, unknown length | 1,420 | 1,159 | 18.4% | -0.5% |
| Complete typed response / one chunk | 25,052 | 23,700 | 5.4% | -0.2% |
| Complete typed response / 8 KiB chunks | 25,163 | 25,296 | -0.5% | -0.0% |

Nanoseconds are rounded; percentages use unrounded medians. Retaining one chunk
has constant assembly work, which explains the large assembly-only percentages.
Complete multi-chunk decoding was effectively unchanged in this run and measured
0.5% slower. The results support the listed workloads rather than an application
or network throughput multiplier.

## Verification and delivery boundary

Seven new regressions cover byte-equivalent CSV scalars, single-chunk allocation
identity, empty frames, bounded concatenation, rejection without mutation,
release of the original chunk, and exact-limit HTTP chunks split inside a UTF-8
codepoint. Existing transport tests retain cancellation, pacing, cooldown,
authentication/write single-send and malformed-response checks.

Required gates are full workspace tests, all-target strict Clippy, default and
`serde_json/arbitrary_precision` library tests, warning-free rustdoc, repository
and API checks, and a fresh extracted crate build including all 38 examples.
The package allowlist includes the private body module while keeping benchmarks,
protocol tests, maintenance tools and this review on GitHub. No dependency,
public signature, credential kind or crypto backend is added.

Local validation passed: **399 workspace checks (238 unit/protocol/maintenance tests plus 161 rustdoc examples)**, all 29 library tests with `serde_json/arbitrary_precision`, strict all-target Clippy, warning-free rustdoc, formatting, repository/API checks and the 86-file package allowlist. The final clean-commit package is also checked after extraction.
