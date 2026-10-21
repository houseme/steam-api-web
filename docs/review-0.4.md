<!--
Copyright 2026 steam-api-web contributors.
SPDX-License-Identifier: Apache-2.0
Licensed under the Apache License, Version 2.0.
See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.
-->

# 0.4 branch review and request preparation

English | [简体中文](review-0.4_ZH.md)

Reviewed on 2026-10-21: v0.3.0 (`852064d`) through `ee0e71b`, including
P3 callers, shared response handling, request encoding, URL caching and tests.
The work is isolated on `houseme/review-0-4` because another task is rewriting
the main checkout. It preserves that task's files and commit history.

## Findings and refactor

The interface modules share one transport, reuse their encoders and keep billing
environments explicit. They do not warrant a wholesale rewrite. The response
validator had developed a separate VAC traversal, and request preparation retained
an avoidable owned URL copy for calls that could never retry.

**Major / P2: explicit VAC errors lost their API error code.** The reviewed
validator required `response.success` before inspecting that body's errors.
`{"response":{"error":{"errorcode":15}}}` therefore returned `Error::Decode`
instead of `Error::Api { code: Some(15), .. }`. The new regression failed on the
reviewed revision, then passed after the refactor. The integration suite now
asserts the error variant/code for all eight anti-cheat/community methods,
instead of checking only that a response failed and secrets stayed redacted.

All response policies now use one recursive error traversal. Named scopes limit
VAC's deferred success check to the immediate canonical `response` body. After
errors have been checked, the policy requires a boolean VAC readiness field or
an explicit `OK` payment acknowledgement. Root, sibling and nested errors remain
strict. Typed and opaque callers still share validation before success metrics.

**Informational: redundant URL ownership in single sends.** The old execution
loop cloned an already owned URL for every attempt, including writes and GETs
with a zero replay budget. Single sends now move that URL and the POST buffer
directly into reqwest. Only observational reads enter the retry loop; their query
is encoded once, earlier attempts clone its URL and the last attempt consumes it.
The actual send function owns the concurrency permit, pacing and attempt metrics,
so these controls cover every path and release before retry backoff.

No public signatures, dependency versions or endpoint/replay classifications
changed. The per-client cache remains bounded by fixed internal paths and never
stores query parameters. Its short synchronous mutex is not a correctness defect;
no evidence justified replacing it with a new synchronization framework.

## Measurements and rejected designs

```sh
cargo bench --bench request --locked --offline -- --preparation-only
```

The reference preparation algorithm is extracted from `ee0e71b` in
`benches/support/preparation_v040.rs`. The candidate compiles the production
`request::Endpoint::request`. Both clone identical input/URL fixtures, build the
request and destroy the output in the timed region. Method, URL, headers and body
must match before timing. Nine A/B/B/A rounds use medians and reject baseline drift
above 10%. No request is sent, and URL-cache lookup is outside this measurement.

Stable Rust 1.99.1, optimized profile, one local accepted run:

| Preparation | Reference ns/op | Current ns/op | Less CPU time | Baseline drift |
| --- | ---: | ---: | ---: | ---: |
| Single-send GET | 148 | 122 | 17.2% | -0.7% |
| Single-send GET with 4 KiB input | 3,753 | 3,735 | 0.5% | -0.9% |
| POST | 131 | 105 | 19.6% | -1.9% |

The 4 KiB difference is not significant. These figures cover request construction,
not Steam latency, transport throughput or concurrent cache contention.

The first broad benchmark aborted at 35% baseline drift, so its values are not
used for attribution. A temporary per-attempt re-encoding design made a three-send
4 KiB read probe about 2.8 times as expensive and was rejected. A generic cloned
RequestBuilder iterator also regressed a small first-read preparation by about
38% and was rejected. Neither design remains in production or the benchmark.
The final read loop retains one encoded query without that extra iterator.

## Verification and distribution

Local gates passed: 170 workspace tests, formatting, strict all-target Clippy,
warning-free rustdoc, repository checks and release metadata checks. A regression
checks that a POST body retains the original String allocation and cached URLs
remain query-free across distinct reads. Existing fixtures cover retry exhaustion,
final-attempt recovery, cooldown, deadlines, concurrency and cancellation.

The 48-file archive still excludes tests, benchmarks, xtask, workflows and docs.
Packaging used a separate target directory with `--no-verify` to preserve the
other task's artifacts; archive policy and a separately extracted all-target
`cargo check --locked --offline` verified the result. There are no Python sources.
Cross-platform CI and live Publisher/payment/ban operations remain separate
validation results. This branch does not publish a crate or change version tags.
