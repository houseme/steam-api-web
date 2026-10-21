<!--
Copyright 2026 steam-api-web contributors.
SPDX-License-Identifier: Apache-2.0
Licensed under the Apache License, Version 2.0.
See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.
-->

# Steam Web API design analysis

English | [简体中文](design_ZH.md)

Reviewed on 2026-10-21 against Valve's Web API documentation and the SDK types, ticket callbacks, and lifecycles in [steamworks-rs](https://github.com/Noxime/steamworks-rs).

## Service boundary

`SteamClient` owns a shared `Arc<Inner>`. Its borrowed `UserAuth`, `UserStats`, and `User` accessors expose 2, 7, and 9 methods without creating separate pools. Native SDK callbacks and backend HTTP requests have different execution models, so the crate does not depend on steamworks/steamworks-sys or implicitly initialize the SDK.

Valve's [overview](https://partner.steamgames.com/doc/webapi_overview) specifies HTTP/1.1, UTF-8 forms, and array parameters using `count` and `name[i]`. Methods use fixed paths and versions, GET query parameters, and POST `application/x-www-form-urlencoded` bodies. rawbinary values are escaped byte by byte, preserving non-UTF-8 bytes, NULs, and reserved form characters.

Publisher credentials default to Partner, where every request needs a Publisher key. Because 403 can cause additional IP throttling, the client neither retries it nor switches hosts automatically. The [key documentation](https://partner.steamgames.com/doc/webapi_overview/auth) supports `x-webapi-key`, avoiding keys in GET URLs. The header is sensitive, redirects are disabled, and reqwest's built-in retries are disabled.

Local validation checks the declared credential kind and parameters. Steam still validates actual key permissions, AppID ownership, and IP allow lists. Use Partner for Publisher-only methods.

## Request lifecycle

1. Validate IDs, text, batch counts, date ranges, stat names, and credential kinds.
2. Acquire a concurrency permit and wait for shared pacing and cooldown inside the overall deadline.
3. Send through the connection pool with separate connection and per-attempt timeouts.
4. Convert unsuccessful HTTP responses into structured errors. Parse Retry-After as seconds or an HTTP date and establish a shared cooldown for retryable statuses. Stored Instants cap extreme server delays at one year; each operation still has a much shorter deadline.
5. Check both declared and accumulated response lengths. Decode JSON, inspect business errors, and select the expected envelope.
6. Retry only replay-safe GET transport errors and HTTP 408/429/500/502/503/504, within a finite budget. Release the permit during exponential backoff with AWS-LC jitter, and honor the shared cooldown before another send.
7. Do not retry parameter, JSON, or business errors. Authentication and writes are never replayed automatically. Cancellation releases the permit. Response size and parser depth constrain synchronous JSON decoding, but Tokio timeouts cannot preempt synchronous CPU work.

These are client isolation and resource controls, not a Steam availability SLA. Applications manage distributed quotas, redundant deployments, circuit breaking, and alerts. Identity, authorization, and ban decisions are not implicitly cached. Persistent failures return explicit errors so applications can reject login, queue work, or offer a retry.

## TLS and credentials

reqwest default features and native TLS are disabled. Rustls TLS 1.2/1.3 uses an explicit `ClientConfig::builder_with_provider(aws_lc_rs::default_provider())` and platform certificate verification. The client neither changes nor adopts the host's global crypto provider. A separate test process installs an unusable global provider and still builds a working client configuration. Anonymous Steam HTTPS reads provide a separate network check.

ApiKey and WebApiTicket Debug output is redacted, and owned strings use zeroize. Copies in network libraries, response JSON, or OS buffers are not guaranteed to be erased. Errors retain fixed endpoints, HTTP statuses, numeric business codes, and parse coordinates, without URLs, bodies, free-form text, or original reqwest/serde error sources. Custom base URLs are trusted configuration and require HTTPS, except for loopback HTTP in tests.

## steamworks-rs comparison

The [user interface](https://github.com/Noxime/steamworks-rs/blob/master/src/user.rs) and [shared types](https://github.com/Noxime/steamworks-rs/blob/master/src/lib.rs) inform these mappings:

| SDK concept | Mapping in this crate |
| --- | --- |
| SteamId::raw/from_raw | A lossless 64-bit bridge for numeric or string JSON IDs |
| AppId / GameId | Separate u32 / u64 values, without narrowing |
| authentication_session_ticket_for_webapi(identity) | AuthenticateUserTicket uses the same nonempty identity |
| TicketForWebApiResponse.result | Check callback success before transmitting a prepared ticket |
| TicketForWebApiResponse.ticket_len / ticket | The reference wrapper copies the full fixed array; validate and extract the effective prefix |
| AuthTicket | A cancellation handle, not ticket content |
| GetAuthSessionTicket / BeginAuthSession | A P2P/server protocol, not a substitute for Web API tickets |
| cancel_authentication_ticket / run_callbacks | SDK initialization, callbacks, and cancellation remain application responsibilities |
| Integer / float stat operations | Preserve read values; the documented Web write API takes uint32, unlike SDK float setters |

The HTTP layer contains no unsafe code and does not copy native FFI or callback state machines. These source links follow master and describe the reviewed snapshot; the [roadmap](roadmap.md) records a fixed tree revision and maps every source module.

## Response modeling and unspecified contracts

Valve's [response documentation](https://partner.steamgames.com/doc/webapi_overview/responses) describes string-encoded uint64 values; the crate accepts both strings and numbers. `ownsapp` is required and never grants access when absent. Visibility-dependent and ban fields use Option, so missing values are not treated as negative assertions. Private friend lists remain HTTP 401 errors; a legitimate vanity no-match result returns None.

The linked AuthenticateUser, GetDeletedSteamIDs, and SetUserStatsForGame pages omit complete response schemas. OpaqueResponse retains the original JSON while standard error envelopes are checked. Applications must validate actual Publisher write results, deletion cursors, and pagination termination rules.

Legacy AuthenticateUser does not document its RSA public key, padding, AES mode, or IV. It accepts prepared ciphertext. The 32-byte length describes the random plaintext session key, not RSA ciphertext length. This is also separate from local Encrypted Application Ticket decryption.

[ISteamUser](https://partner.steamgames.com/doc/webapi/ISteamUser) prints a v4 GetPublisherAppOwnership URL but mentions v5 expiry fields in its history. The default follows the explicit URL, with an explicit v5 option and no automatic fallback; timeexpires is optional for v4. The anonymous public GetSupportedAPIList does not enumerate that Publisher method, so it cannot confirm v5 deployment.

## Verification layers

- The original P0 contract tests cover its 18 methods, fixed versions, GET/POST, headers, envelopes, indexed forms, all 256 byte values, integer precision, optional fields, business failures, replay policy, retry budgets, cooldowns, timeouts, cancellation, batching, response limits, redirects, and provider isolation.
- Local static validation uses stable Rust, fmt, strict Clippy, rustdoc, and example compilation.
- On 2026-10-21, current_players retrieved 23944 for AppID 480 from the Public host. This is one observation, demonstrating that anonymous HTTPS/response path at that time, not ongoing availability or Publisher validation.
- Credentialed Publisher checks remain necessary for mismatched/expired/banned tickets, AppID authorization, write/read-back, deletion cursors, v5 expiry, and actual quotas. No local credentials were searched and no live stat writes were performed.

CI targets Linux/macOS/Windows while retaining the existing audit and SARIF workflows. Remote CI results are distinct from local checks. The locked ICU dependencies declare Rust 1.88 as their minimum; local checks used stable 1.99.1 without installing an older toolchain.

## 0.2 P1 extension and repository boundary

The development version adds nine P1 interfaces and 39 methods for discovery, apps, players, store, reviews, news, game servers, server stats and leaderboards: 57 methods across 12 interfaces with P0. Service requests use input_json. Administrative POSTs never automatically replay. Server login-token inputs and response Debug output are redacted, and leaderboard details preserve raw bytes with a 256-byte bound.

SetAppBuildLive preserves HTTP status to distinguish accepted requests from pending mobile confirmation on 201. Only explicitly allowed 201 responses may omit a body; empty 200 remains Decode. Shared error checks recognize result/eresult and reject fractional result codes. Reviews and Partner app lists model officially published fields; unspecified schemas retain OpaqueResponse rather than inventing empty collections or success.

Maintenance and release guards move to a separate, nonpublishable Rust xtask; workflows no longer require Python. Cargo's include allowlist keeps repository tools, integration tests and reference material on GitHub. The public library does not depend on xtask. Complete workspace testing and independent minimal-archive builds validate both graphs, preventing tooling feature unification from hiding library dependency errors. Local P1 tests are not evidence of live Steam Publisher writes.
