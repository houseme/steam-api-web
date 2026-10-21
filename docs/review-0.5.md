<!--
Copyright 2026 steam-api-web contributors.
SPDX-License-Identifier: Apache-2.0
Licensed under the Apache License, Version 2.0.
See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.
-->

# 0.5 branch review and shared numeric decoding

English | [简体中文](review-0.5_ZH.md)

Reviewed on 2026-10-22: main `e54b0fc` through `0a4617b`, including financial
credentials, all six P4 callers, response models, date validation, coverage
tooling, packaging and CI. The review and refactor use the existing isolated
feature worktree and a dedicated Cargo target directory.

## Findings and refactor

P4 reuses the shared transport,
encoding, response validation and bounded read retries; it does not warrant a
wholesale rewrite. Financial authority stays distinct from other key kinds, and
broadcast writes stay single-send. Date validation is already centralized.
The global follow-up below fixes a dependency-feature failure for large statistics
while preserving this shared architecture.

**Informational: repeated integer parsing and speculative string branches.**
Financial reports introduced an untagged number/string decoder alongside the
existing unsigned decoder in `types`. Cloud also wrapped its optional integers
locally. An untagged decoder buffers the scalar and tries the numeric variant;
quoted numbers fall through to an owned String variant after that attempt fails.

One shared Serde Visitor now handles required and optional `u32`, `u64`, `i32`
and `i64` fields. It parses decimal text from `&str` and uses checked conversions
for numeric input, eliminating the variant retry and intermediate string copy.
Financial rows/cursors, Steam IDs, Cloud IDs, review IDs, user-group IDs and site
IDs use this path. Missing optional fields, null, zero, signed extremes and
full-width unsigned values retain their meaning. Floats, malformed text and
overflow remain errors. Public types and signatures are unchanged. `StatValue`
retains its separate floating-point contract.

The source-coverage guard deliberately checks module exports, method declarations
and exact route literals rather than parsing Rust ASTs. Its output already states
that it is static mapping coverage. No evidence justified introducing a compiler
frontend, changing the transport cache or adding a synchronization dependency.

## Reproduce the measurement

```sh
cargo bench --bench integer --locked --offline
```

The reference helper is extracted from `0a4617b` into
`benches/support/integer_v050.rs`; the candidate compiles `src/types.rs` directly.
Each operation decodes 100 signed or unsigned scalars, including zero and width
extremes. Numeric and decimal-string fixtures produce identical values before
timing. Both direct JSON decoding and the client's JSON-to-Value-to-typed stages
include parsing and output destruction. No input clones or fixture construction
are added to the timed region.

Each case warms up 100 times, then runs nine A/B/B/A rounds with 2,000 operations
per sample. The harness reports 18-sample medians and rejects baseline drift above
10%. One accepted local run, stable Rust 1.99.1 and optimized Cargo bench profile:

| Scalars / path | Reference ns/op | Current ns/op | Less CPU time | Baseline drift |
| --- | ---: | ---: | ---: | ---: |
| Signed numbers / direct | 1,618 | 1,221 | 24.6% | +0.4% |
| Signed numbers / Value | 2,073 | 1,748 | 15.7% | +1.0% |
| Signed strings / direct | 13,821 | 2,058 | 85.1% | -1.1% |
| Signed strings / Value | 14,614 | 3,150 | 78.4% | +0.0% |
| Unsigned numbers / direct | 1,513 | 1,132 | 25.2% | +0.8% |
| Unsigned numbers / Value | 1,965 | 1,656 | 15.7% | +1.7% |
| Unsigned strings / direct | 17,241 | 2,446 | 85.8% | +2.2% |
| Unsigned strings / Value | 16,996 | 3,600 | 78.8% | +1.9% |

These measurements cover scalar decoding, not complete reports, HTTP throughput
or production Steam latency. JSON Value parsing still owns its input strings;
the refactor avoids the additional copy within typed integer decoding. Timing
does not count allocations or prove a globally optimal implementation.

## Verification and distribution

Three new tests exercise every supported width through direct JSON, borrowed
Value and owned Value deserializers. They cover decimal spelling, escaped digits,
boundaries, overflow, malformed inputs and optional missing/null/zero distinctions.
Existing Financial, Site, Cloud, user and review protocol fixtures verify callers.

Local validation passes 210 workspace tests, strict all-target Clippy, formatting,
warning-free rustdoc, repository/coverage guards and release metadata checks.
The 52-file archive keeps benchmarks, tests, xtask, workflows and prose documents
on GitHub; its allowlist and independently extracted all-target build are checked.
No dependency or Python source was added, and AWS-LC remains the per-client TLS
backend. The earlier CI result belongs to `0a4617b`; CI for the refactor is tracked
separately in the PR.

No live financial data, site credentials or broadcast writes were used. Site
wrapper/array-omission conventions and remote account permissions still require
credentialed validation. This review does not create a tag or publish a crate.

## Global follow-up after refreshing main

On 2026-10-22, `git fetch origin` followed by `git merge origin/main` reported
`Already up to date`: main `e54b0fc` was already an ancestor of `e051ee0`.
No merge commit or history rewrite was necessary. This follow-up reviews the
whole request lifecycle rather than treating P4 modules as isolated code.

| Shared path | Decision and evidence |
| --- | --- |
| Concurrency, pacing, cooldown and retry | Retain the shared send boundary. Permits cover request pacing and body reads; backoff releases them; writes stay single-send and fixed hosts prevent cross-host credential fallback. Existing protocol fixtures cover these behaviors. |
| Request encoding | Retain direct form/Service JSON writing and indexed-key stack formatting. A replacement using `url::form_urlencoded::byte_serialize` regressed small forms and mixed bytes, despite improving long unescaped byte runs; that candidate was discarded. |
| Response handling | Retain one JSON parse and shared business-error traversal before consuming typed/opaque data. Direct typed parsing alone would omit required error-envelope checks and opaque/unknown fields. Synchronous decoding remains bounded by body/parser limits and cannot be preempted by a Tokio timeout, as documented in the design. |
| Wire scalars | Keep the checked integer Visitor and give `StatValue` its own numeric Visitor. Integers reject floats; statistics retain finite floats and Number's feature-dependent precision. These are different wire contracts, not separate transport layers. |

`StatValue` was the remaining number/string trial decoder in the public wire
scalar layer. Global totals, user stats, and schema/achievement defaults share
that type. Its new Visitor handles unsigned, signed, finite float and borrowed
numeric text directly. It delegates Number's map representation to Number's own
deserializer, preserving downstream `serde_json/arbitrary_precision` feature
unification without relying on a private field-name literal. Default behavior,
negative zero, integer limits, exponent notation and JSON serialization remain
compatible. No new runtime, generic framework, dependency or transport switch is
introduced.

**Minor correctness finding: feature-unified large numbers were rejected.**
With downstream `serde_json/arbitrary_precision` enabled, a numeric statistic
such as `340282366920938463463374607431768211455` parses into a valid Number.
The Value-to-typed path then presents it as u128, which the old untagged buffering
visitor rejects before Number can read it. The client therefore returned Decode
for that value. An isolated probe using the original decoder reproduces the
rejection, while the new decoder preserves the exact value. The new Visitor
delegates both 128-bit scalar variants and Number's map representation to Number
itself. Tests cover u128::MAX, i128::MIN and long decimal precision through the
client's owned-Value stage as well as direct/borrowed decoding.

### Complete response measurement

```sh
cargo bench --bench integer --locked --offline -- --statistics-only
```

The scalar reference is the original StatValue decoder from `e051ee0`. Complete
response cases compile the production `response` and `error` modules and use the
public `GlobalStats` model. Their reference models have the same fields, optional
totals, BTreeMap types and flattened unknown metadata, substituting only the old
scalar type. Totals, result codes and unknown fields are checked for equality
before timing. Each response contains 100 stats with mixed signed/unsigned and
floating-point totals, plus retained metadata. Input construction is outside the
timed region; parsing, business checks, envelope selection, model construction
and output destruction are included. There is no HTTP I/O.

Stable Rust 1.99.1, optimized profile, nine ABBA rounds and the same 10% drift gate:

| Case | Reference ns/op | Current ns/op | Less CPU time | Baseline drift |
| --- | ---: | ---: | ---: | ---: |
| Numeric scalars / direct JSON | 1,682 | 1,174 | 30.2% | -7.0% |
| Numeric scalars / Value | 1,952 | 1,632 | 16.4% | -0.0% |
| String scalars / direct JSON | 14,167 | 2,039 | 85.6% | +1.4% |
| String scalars / Value | 14,774 | 3,348 | 77.3% | -0.6% |
| Complete numeric response, 6,166 bytes | 41,485 | 41,141 | 0.8% | -0.0% |
| Complete string response, 6,366 bytes | 56,561 | 44,133 | 22.0% | +0.1% |

The complete numeric response is effectively neutral; its DOM/map work dominates.
The complete string response retains a material CPU improvement. Neither result
is a network-throughput or global-optimality claim. The discarded standard byte
encoder made the small scalar form about 42% slower and arbitrary 4 KiB octets
about 127% slower in the local counterexperiment; its implementation is absent.

### Follow-up validation

The workspace passes 212 tests, strict all-target Clippy, warning-free rustdoc,
formatting, repository/coverage and release metadata guards. All 23 library tests
also pass with `--features serde_json/arbitrary_precision`; Linux CI now keeps
that feature-unification check. The new scalar tests cover direct JSON, borrowed
and owned Value, precision, negative zero, exponent notation, malformed values
and non-finite deserializers. Existing user-stats HTTP fixtures cover callers.

The archive remains 52 files and is verified after extraction in a fresh target
directory. Tests, benchmarks, workflows and this bilingual analysis stay on
GitHub; no Python or dependency was added, and AWS-LC isolation is unchanged.
Current-head CI is recorded in the PR separately from these local results.
