<!--
Copyright 2026 steam-api-web contributors.
SPDX-License-Identifier: Apache-2.0
Licensed under the Apache License, Version 2.0.
See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.
-->

# Usage guide

English | [简体中文](usage_ZH.md)

Detailed usage for `steam-api-web`. The repository README keeps only the quick
start and coverage summary; this guide holds configuration, reliability,
data handling, native SDK interoperability, toolchain and packaging notes.

## Credentials and hosts

API keys travel in Valve's sensitive `x-webapi-key` header; redirects are
disabled. Publisher keys default to Valve's recommended `partner.steam-api.com`
host, where **every request** requires a Publisher key, including otherwise
anonymous methods. Anonymous clients and User keys default to
`api.steampowered.com`. Select a host explicitly with `host(Host::Public/Partner)`;
automatic cross-host failover is disabled. Steam still checks actual key
permissions, AppID ownership and IP allow lists.

Each method's minimum credential (none, User or Publisher) is part of its
documented contract and is checked locally before sending. Rust methods use
snake_case; run `cargo doc --no-deps --open` for the complete API.

## API surface

| Interface | Rust accessor | Routes |
| --- | --- | ---: |
| ISteamUserAuth | `user_auth()` | 2 |
| ISteamUserStats | `user_stats()` | 7 |
| ISteamUser | `user()` | 9 |
| ISteamWebAPIUtil | `web_api_util()` | 2 |
| ISteamApps | `apps()` | 10 |
| IPlayerService | `player()` | 6 |
| IStoreService | `store()` | 1 |
| IUserReviewsService | `reviews()` | 1 |
| ISteamNews | `news()` | 2 |
| IGameServersService | `game_servers()` | 9 |
| ISteamGameServerStats | `game_server_stats()` | 1 |
| ISteamLeaderboards | `leaderboards()` | 7 |
| ICloudService | `cloud()` | 6 |
| IInventoryService | `inventory()` | 10 |
| IGameInventory | `game_inventory()` | 5 |
| ISteamEconomy | `economy()` | 8 |
| IEconService | `econ()` | 7 |
| IEconMarketService | `econ_market()` | 4 |
| IPublishedFileService | `published_file()` | 7 |
| ISteamPublishedItemSearch | `published_item_search()` | 4 |
| ISteamPublishedItemVoting | `published_item_voting()` | 2 |
| IPublishedFileModerationService | `published_file_moderation()` | 2 |
| ISteamRemoteStorage | `remote_storage()` | 7 |
| IWorkshopService | `workshop()` | 4 |
| ICheatReportingService | `cheat_reporting()` | 7 |
| IGameNotificationsService | `game_notifications()` | 7 |
| ILobbyMatchmakingService | `lobby_matchmaking()` | 3 |
| ISteamCommunity | `community()` | 1 |
| ISteamMicroTxn | `micro_txn_production()` | 10 |
| ISteamMicroTxnSandbox | `micro_txn_sandbox()` | 10 inherited |

The per-method inventory — paths, versions, HTTP verbs, credential kinds and
replay classification — lives in the [roadmap](roadmap.md) and the
machine-readable [api-inventory.json](api-inventory.json).

Contract notes by phase:

- **P0** — `AuthenticateUserTicket` never retries even though it uses GET.
  `GetPublisherAppOwnership` defaults to the printed v4 path with an explicit
  v5 option.
- **P1** — Service interfaces use `input_json`. Legacy `ISteamApps/GetAppList/v2`
  is deprecated; prefer paginated `IStoreService/GetAppList`. `SetAppBuildLive`
  preserves HTTP 201 mobile-confirmation state rather than equating acceptance
  with deployment.
- **P2** — Cloud uses user OAuth with strict `x-eresult` checks; upload stages
  return metadata while callers manage file transfer and batch completion.
  Inventory request IDs, concurrency timestamps, trade cursors and Workshop
  revisions stay explicit. Mutations never replay, including GET StartTrade and
  GET Delete. See [P2 contracts](p2.md).
- **P3** — Billing explicitly selects Sandbox or production and never switches
  environments; acknowledgements stay distinct from settlement, and VAC
  readiness stays distinct from verification. See [P3 contracts](p3.md).

## Configuration and reliability

| Setting | Default | Purpose |
| --- | --- | --- |
| connect_timeout | 3 seconds | Connection establishment deadline |
| request_timeout | 10 seconds | One HTTP attempt, including reading its response body |
| operation_timeout | 30 seconds | Overall queueing, pacing, network and backoff budget |
| max_in_flight | 32 | Concurrency limit shared by clones |
| max_response_bytes | 8 MiB | Checks both Content-Length and accumulated response chunks |
| RetryPolicy.max_retries | 2 | Additional replay-safe GET attempts; maximum 10 |
| initial_backoff / max_backoff | 200 ms / 2 seconds | Exponential backoff with AWS-LC jitter in 50%..100% |
| min_request_interval | 0 | Shared minimum request interval; set to your Steam quota |

Only read transport failures and HTTP 408/429/500/502/503/504 are retried.
Authorization, parameter, JSON and business errors never retry. `Retry-After`
accepts seconds or an HTTP date and establishes a shared cooldown; the local
backoff cap never causes an early retry. Exceeding the overall budget returns
`Error::Deadline`.

Authentication — including GET ticket verification — and stat writes are never
replayed automatically; reqwest's built-in retry layer is also disabled. A
timed-out write may have been applied: query its state before deciding whether
to compensate. Canceling a future releases its concurrency permit. Profile
batching performs sequential requests of at most 100 IDs, each with its own
operation deadline; wrap the complete batch in an outer Tokio timeout if needed.

`metrics()` exposes attempts/retries/succeeded/failed snapshots. Validation
failures and canceled operations do not count as completed calls. Deployment
code manages distributed quotas, application degradation, cross-instance
monitoring and remote SLAs. Authentication and authorization results are not
implicitly cached.

## Data handling and security

- SteamIDs accept JSON numbers or decimal strings and serialize as strings,
  preserving the entire 64-bit integer range. Statistics preserve signed
  integers, unsigned integers and floating-point values.
- Visibility-dependent fields use `Option`. Missing ban flags never default to
  "not banned". A private friend list returns HTTP 401 as an error; an unmatched
  vanity name returns `Ok(None)`.
- Methods whose responses Valve does not document — including `AuthenticateUser`,
  `GetDeletedSteamIDs` and `SetUserStatsForGame` — return `OpaqueResponse` with
  the original JSON. Standard business error envelopes are still checked. Access
  the JSON explicitly via `as_json/into_json`; the crate does not invent write
  success, pagination fields or termination rules. `OpaqueResponse` may contain
  sensitive data; do not log its JSON.
- Legacy `AuthenticateUser` transports prepared RSA/AES ciphertext. It does not
  guess the undocumented RSA key, padding, AES mode or IV. rawbinary values are
  byte-escaped rather than converted to UTF-8 or hex. Prefer Web API tickets for
  new integrations.
- Owned key and ticket strings are zeroized on drop; copies inside HTTP
  libraries or OS buffers are not guaranteed to be cleared. Public errors and
  client Debug output omit URLs, tickets, keys, raw responses and free-form
  server text. Custom base URLs must be trusted and use HTTPS, except loopback
  HTTP in tests.

## Interoperability with steamworks-rs

The design is compared with [Noxime/steamworks-rs](https://github.com/Noxime/steamworks-rs),
which wraps the native SDK and callbacks. This crate wraps HTTPS APIs, so the
two libraries can run in the game client and on the backend respectively.

- Pass the `u64` from `steamworks::SteamId::raw()` into user methods; this crate
  exposes `SteamId::from_raw/raw` for conversion in either direction.
- AppIDs use `u32`, accessible through the SDK's `AppId(x).0`. A 64-bit GameID
  is handled separately and is never narrowed to an AppID.
- Register a `TicketForWebApiResponse` callback, call
  `authentication_session_ticket_for_webapi(identity)`, and drive
  `run_callbacks()`. Send the effective ticket to your backend only after
  checking the callback result.
- The reference wrapper's `TicketForWebApiResponse::ticket` can contain the
  entire fixed buffer. Use `from_callback_bytes(&response.ticket, response.ticket_len)`
  to validate the length and extract its valid prefix.

Client callback conversion, in an application that depends on steamworks:

```rust,ignore
if response.result.is_ok() {
    let ticket = WebApiTicket::from_callback_bytes(&response.ticket, response.ticket_len)?;
    // Send ticket.expose_hex() to your backend over a secure connection.
}
```

The backend must use the same identity used to create the ticket:

```rust
use steam_api_web::{SteamClient, user_auth::WebApiTicket};

async fn verify(client: &SteamClient, bytes: &[u8]) -> steam_api_web::Result<()> {
    let ticket = WebApiTicket::from_bytes(bytes)?;
    let user = client.user_auth()
        .authenticate_user_ticket(480, &ticket, "my-game-backend")
        .await?;
    // Use the Steam-verified user.steamid, then apply your ban and access policy.
    Ok(())
}
```

The SDK's `AuthTicket` is a cancellation handle, not ticket bytes. P2P/server
`GetAuthSessionTicket`, Web API tickets and Encrypted App Tickets are separate
protocols. The client application manages SDK initialization, callback
lifetimes, result checks and cancellation. See the
[design analysis](design.md).

## Rust toolchain compatibility

Use the latest stable Rust toolchain and edition 2024. Local validation used
stable **1.99.1**. The current locked ICU dependencies declare Rust 1.88 as
their minimum; the project does not promise an independently tested minimum
compiler version. CI tracks stable on Linux, macOS and Windows.

## Repository and packaging

Cargo.toml uses an explicit `include` allowlist. The distributable retains
library Rust sources, examples, LICENSE, bilingual READMEs/changelogs and
Cargo's required manifests, lockfile and VCS metadata. `.github/`, `xtask/`,
`scripts/`, `tests/`, `docs/` and reference JSON stay on GitHub rather than in
the downloaded crate. Repository maintenance uses Rust `xtask`, with no Python
maintenance files; library consumers do not need these tools. Repository
documentation links point to GitHub so they remain usable after packaging
exclusions.

Dependencies were checked against the latest non-yanked crates.io versions on
**2026-10-21**, including prereleases, with Cargo.lock retained. Rustls uses
**0.23.46**, the latest version compatible with reqwest; the newer
**0.24.0-dev.2** preview has incompatible types and is the documented
exception. Other direct dependencies use the highest semantic version found.
An explicit per-client AWS-LC provider prevents a host application's Rustls
global provider from overriding this choice. See the
[dependency record](dependencies.md).

Contributor checks, run in a GitHub checkout (local protocol tests issue no
live Steam administrative writes):

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
cargo bench --bench codec --locked --offline
cargo bench --bench request --locked --offline
cargo doc --no-deps --all-features --locked
cargo xtask check-repository
cargo xtask release verify-package
cargo run --example current_players -- 480
```

Local mock tests cover every implemented method's path, version, parameters
and response, plus retries, cooldowns, timeouts, cancellation, batching,
integer precision and redaction. They need no Steam key but require local TCP
binding permission. Passing them does not establish live Publisher
authentication, writes or production concurrency.

## Further reading

| Topic | Document |
| --- | --- |
| Design analysis and verification layers | [design.md](design.md) |
| P2 Cloud, inventory, economy and Workshop contracts | [p2.md](p2.md) |
| P3 billing, anti-cheat and session contracts | [p3.md](p3.md) |
| Roadmap and complete method inventory | [roadmap.md](roadmap.md) |
| Transport reviews and codec measurements | [performance.md](performance.md), [review-0.3.md](review-0.3.md) |
| Dependency audit | [dependencies.md](dependencies.md) |
| Release process | [releases.md](releases.md) |
