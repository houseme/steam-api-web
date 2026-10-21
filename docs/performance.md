<!--
Copyright 2026 steam-api-web contributors.
SPDX-License-Identifier: Apache-2.0
Licensed under the Apache License, Version 2.0.
See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.
-->

# Transport review and codec measurements

English | [简体中文](performance_ZH.md)

Reviewed on 2026-10-21: the 0.2 development changes from v0.1.0 through
8b7613a, including shared transport, all P1 callers, maintenance commands and
package/workflow boundaries. The interface implementations follow a consistent
transport contract. The shared encoding/decoding path had accumulated avoidable
copies and endpoint switches; this review refactors those paths without changing
the public method signatures. It does not establish a globally optimal design
or production Steam availability.

## Findings and implementation

| Verified issue in the reviewed revision | Refactor |
| --- | --- |
| `client::param` owned every key/value before `encode_form` copied them into another buffer; POST cloned the resulting form again. | `request::Params` builds the final form directly; raw bytes are encoded from borrowed slices. POST takes ownership of that buffer. |
| Service JSON used separate serializers, including a helper owned by the player module and an intermediate `Value` in game servers. | All services use `Params::json` with borrowed serializable request structures. JSON writes directly through the percent encoder. |
| `call_with_status` deserialized a borrowed `Value`, copying strings and rebuilding opaque JSON. Standard error envelopes were checked again while selecting them. | `response::Response` moves selected JSON into typed deserialization. Opaque responses take the original JSON containers directly. Error checks visit standard envelopes once, while still checking other selected envelopes and sibling errors. |
| `Endpoint::auth(..., post)` also described administrative writes; independent booleans represented HTTP method, replay and empty-body behavior. | Named GET/read, single GET and POST operations keep method/replay choices together. Empty HTTP 201 acceptance has an explicit body policy. |

Concurrency permits still cover pacing and the complete response read. Waiting
for pacing before acquiring a permit could compress request starts after queueing
and violate the configured interval. Retries release permits during backoff;
cancellation releases them automatically. The existing shared cooldown, finite
retry budget, deadlines, AWS-LC isolation and secret redaction remain covered by
protocol tests. No speculative synchronization or runtime replacement was added.

## Reproduce the measurements

From a Git checkout:

```sh
cargo bench --bench codec --locked --offline
```

The harness compiles the production request/response modules. Its reference
encoding and business-error checks come from 8b7613a; the reference response
decoder uses the previous borrowed deserialization algorithm on valid fixtures.
The opaque wrapper has the same `Value` representation and transparent Serde
behavior as the library. Neither variant performs network I/O.

Each case warms up 100 times, then interleaves nine A/B/B/A rounds and reports
the median of 18 samples per implementation. Response cases parse identical
60,704-byte fixtures containing 100 games, including output destruction in the
timed region. Form and response equivalence checks run before timing. Service
JSON equivalence ignores object key order, which is not part of its contract.

One local run with stable Rust 1.99.1 and the optimized Cargo bench profile:

| Case | Previous ns/op | Current ns/op | Time reduction |
| --- | ---: | ---: | ---: |
| Ordinary form | 262 | 111 | 57.5% |
| Service JSON form | 474 | 144 | 69.7% |
| Typed response, including parsing | 26,235 | 22,983 | 12.4% |
| Opaque response, including parsing | 33,006 | 21,219 | 35.7% |

These are representative local CPU measurements, not an end-to-end throughput
claim or a CI timing threshold. Network latency, API quotas, payload shape and
allocator behavior affect application results. Timing does not count allocations;
separate regression tests verify that typed string storage and opaque array
storage transfer without copying.

## Verification and distribution

The five new focused tests cover JSON escaping/type fidelity, redacted serializer
errors, owned typed strings, unchanged opaque container storage, and failures in
selected/sibling/missing envelopes. Together with existing protocol, TLS,
maintenance and documentation tests, the workspace runs 80 passing tests.
Strict Clippy also compiles the benchmark. Package verification checks that the
benchmark, reference algorithm and this document stay in the repository; only
the new production modules join the published crate.

## 0.4 follow-up: indexed keys and endpoint URLs

Reviewed on 2026-10-21 during 0.4 development. Two residual per-call CPU costs
remained after the rounds above. Public signatures and wire encodings remain
unchanged.

**Indexed form keys.** Every indexed or array parameter formatted a temporary
key `String` per field (`format!("classid{index}")` and `name%5B0%5D` spellings)
across economy class/asset loops, remote-storage and voting ID batches,
user-stats name/value batches, published-item search tags, and the new
microtransaction cart encoding. `Params::push_indexed` and
`Params::push_suffixed` now write the key directly into the final form using an
itoa stack buffer. Brackets still travel percent-encoded (`name%5B0%5D`),
byte-for-byte like a preformatted key, which a new unit test pins against
`url::form_urlencoded`. Plain key names also borrow their unescaped ASCII
prefix instead of visiting every byte through the escaping encoder.

**Endpoint URLs.** Each call re-parsed its endpoint URL
(`Url::join` + `set_query`). Clients now parse each fixed endpoint path once
into a per-client cache guarded by a synchronous mutex; every call clones and
appends its query. The guard is never held across an await, and the cache is
bounded by the fixed internal endpoint set. Query parameters and credentials are
never stored in this cache. The encoder benchmark below does not measure this
cache or concurrent transport throughput.

Measured with `cargo bench --bench request --locked --offline`
(stable 1.99.1, optimized profile, A/B/B/A medians, wire equivalence asserted
before timing). The reference is commit 2077350, before the 0.3 transport review;
these figures include subsequent 0.3 and 0.4 encoder improvements:

| Case | Reference ns/op | Current ns/op | Time reduction | Baseline drift |
| --- | ---: | ---: | ---: | ---: |
| 100 indexed IDs | 5,329 | 1,619 | 69.6% | -0.3% |
| 100 microtransaction cart lines | 17,552 | 5,640 | 67.9% | -4.6% |

These are local encoder CPU measurements, not end-to-end latency claims; input
data construction is outside the timed region, and the final form buffer
allocation remains by design. The full
workspace (168 tests including the new wire-spelling test), strict Clippy with
`-D warnings`, rustdoc and formatting checks pass after the change.
