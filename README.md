<!--
Copyright 2026 steam-api-web contributors.
SPDX-License-Identifier: Apache-2.0
Licensed under the Apache License, Version 2.0.
See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.
-->

# steam-api-web

[![Crates.io](https://img.shields.io/crates/v/steam-api-web.svg)](https://crates.io/crates/steam-api-web)
[![Documentation](https://docs.rs/steam-api-web/badge.svg)](https://docs.rs/steam-api-web)
[![CI](https://github.com/houseme/steam-api-web/actions/workflows/ci.yml/badge.svg)](https://github.com/houseme/steam-api-web/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-stable-orange.svg)](#rust-toolchain-compatibility)

English | [简体中文](README_ZH.md)

An independent asynchronous Rust client covering all **18 methods** documented for `ISteamUserAuth`, `ISteamUserStats`, and `ISteamUser`. Built with Tokio, reqwest, Rustls, and **aws-lc-rs**, without requiring the Steam client or native Steamworks SDK.

## Usage

Install a published release from crates.io using the manifest below. Before the initial publication or for local development, replace the crate dependency with `{ path = "../steam-api-web" }`. The crates.io and docs.rs badges reflect published versions.

```toml
[dependencies]
steam-api-web = "0.1.0"
tokio = { version = "1.54.0", features = ["macros", "rt-multi-thread"] }
```

```rust
use steam_api_web::{ApiKey, SteamClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = SteamClient::builder()
        .api_key(ApiKey::publisher(std::env::var("STEAM_PUBLISHER_KEY")?)?)
        .build()?;
    let license = client.user()
        .check_app_ownership(76561198000000000_u64, 480)
        .await?;
    println!("active license: {}", license.ownsapp);
    let players = client.user_stats().get_number_of_current_players(480).await?;
    println!("online: {}", players.player_count);
    Ok(())
}
```

Reuse or clone `SteamClient`: all clones share the connection pool, concurrency limit, request pacing, server cooldown, and metrics.

## API coverage

| Interface | Method | Version / HTTP | Minimum credentials on the Public host |
| --- | --- | --- | --- |
| ISteamUserAuth | AuthenticateUser | v1 POST | None; requires prepared encrypted login material |
| ISteamUserAuth | AuthenticateUserTicket | v1 GET | User / Publisher |
| ISteamUserStats | GetGlobalAchievementPercentagesForApp | v2 GET | None |
| ISteamUserStats | GetGlobalStatsForGame | v1 GET | Publisher |
| ISteamUserStats | GetNumberOfCurrentPlayers | v1 GET | None |
| ISteamUserStats | GetPlayerAchievements | v1 GET | User / Publisher |
| ISteamUserStats | GetSchemaForGame | v2 GET | User / Publisher |
| ISteamUserStats | GetUserStatsForGame | v2 GET | User / Publisher |
| ISteamUserStats | SetUserStatsForGame | v1 POST | Publisher |
| ISteamUser | CheckAppOwnership | v4 GET | Publisher |
| ISteamUser | GetAppPriceInfo | v1 GET | Publisher |
| ISteamUser | GetDeletedSteamIDs | v1 GET | Publisher |
| ISteamUser | GetFriendList | v1 GET | User / Publisher |
| ISteamUser | GetPlayerBans | v1 GET | User / Publisher |
| ISteamUser | GetPlayerSummaries | v2 GET | User / Publisher |
| ISteamUser | GetPublisherAppOwnership | v4 GET; explicit v5 option | Publisher |
| ISteamUser | GetUserGroupList | v1 GET | Publisher |
| ISteamUser | ResolveVanityURL | v1 GET | User / Publisher |

Publisher keys default to Valve's recommended `partner.steam-api.com` host, which requires a Publisher key for **every request**, including otherwise anonymous methods. Anonymous clients and User keys default to `api.steampowered.com`. Select a host explicitly with `host(Host::Public/Partner)`. Use Partner for Publisher-only methods; automatic cross-host failover is disabled. Steam checks the actual key permissions, AppID ownership, and IP allow list.

Rust methods use snake_case through `client.user_auth()`, `client.user_stats()`, and `client.user()`. Run `cargo doc --no-deps --open` for the complete Rust API.

## Interoperability with steamworks-rs

The design is compared with [Noxime/steamworks-rs](https://github.com/Noxime/steamworks-rs), which wraps the native SDK and callbacks. This crate wraps HTTPS APIs, allowing the two libraries to run in the game client and backend respectively.

- Pass the `u64` from `steamworks::SteamId::raw()` into user methods; this crate exposes `SteamId::from_raw/raw` for conversion in either direction.
- AppIDs use `u32`, accessible through the SDK's `AppId(x).0`. A 64-bit GameID is handled separately and is never narrowed to an AppID.
- Register a `TicketForWebApiResponse` callback, call `authentication_session_ticket_for_webapi(identity)`, and drive `run_callbacks()`. Send the effective ticket to your backend only after checking the callback result.
- The reference wrapper's `TicketForWebApiResponse::ticket` can contain the entire fixed buffer. Use `from_callback_bytes(&response.ticket, response.ticket_len)` to validate the length and extract its valid prefix.

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

The SDK's `AuthTicket` is a cancellation handle, not ticket bytes. P2P/server `GetAuthSessionTicket`, Web API tickets, and Encrypted App Tickets are separate protocols. The client application manages SDK initialization, callback lifetimes, result checks, and cancellation. See the [design analysis](docs/design.md).

## Availability configuration

| Setting | Default | Purpose |
| --- | --- | --- |
| connect_timeout | 3 seconds | Connection establishment deadline |
| request_timeout | 10 seconds | One HTTP attempt, including reading its response body |
| operation_timeout | 30 seconds | Overall queueing, pacing, network, and backoff budget |
| max_in_flight | 32 | Concurrency limit shared by clones |
| max_response_bytes | 8 MiB | Check both Content-Length and accumulated response chunks |
| RetryPolicy.max_retries | 2 | Additional replay-safe GET attempts; maximum 10 |
| initial_backoff / max_backoff | 200 ms / 2 seconds | Exponential backoff with AWS-LC jitter in 50%..100% |
| min_request_interval | 0 | Shared minimum request interval; configure for your Steam quota |

Only read transport failures and HTTP 408/429/500/502/503/504 are retried. Authorization, parameter, JSON, and business errors are not retried. `Retry-After` accepts seconds or an HTTP date and establishes a shared cooldown. The local backoff cap never causes an early retry. Exceeding the overall budget returns `Error::Deadline`.

Authentication, including GET ticket verification, and stat writes are never replayed automatically; reqwest's built-in retry layer is also disabled. A timed-out write may have been applied: query its state before deciding whether to compensate. Canceling a Future releases its concurrency permit. Profile batching performs sequential requests of at most 100 IDs, each with its own operation deadline; wrap the complete batch in an outer Tokio timeout if needed.

`metrics()` exposes attempts/retries/succeeded/failed snapshots. Validation failures and canceled operations do not count as completed calls. Deployment code manages distributed quotas, application degradation, cross-instance monitoring, and remote SLAs. Authentication and authorization results are not implicitly cached.

## Data and errors

- SteamIDs accept JSON numbers or decimal strings and serialize as strings, preserving the entire 64-bit integer range. Statistics preserve signed integers, unsigned integers, and floating-point values.
- Visibility-dependent fields use `Option`. Missing ban flags never default to “not banned.” A private friend list returns HTTP 401 as an error; an unmatched vanity name returns `Ok(None)`.
- `AuthenticateUser`, `GetDeletedSteamIDs`, and `SetUserStatsForGame` return `OpaqueResponse` because their linked references omit complete response schemas. Standard business error envelopes are still checked. Access JSON explicitly using `as_json/into_json`; the library does not invent write success, pagination fields, or termination rules.
- Valve prints a v4 URL for `GetPublisherAppOwnership` but mentions v5 in its change history. The default follows the explicit URL; an enum allows choosing v5. Its expiry fields require live Publisher validation.
- Legacy `AuthenticateUser` transports prepared RSA/AES ciphertext. It does not guess the undocumented RSA key, padding, AES mode, or IV. rawbinary is byte-escaped rather than converted to UTF-8 or hex. Prefer Web API tickets for new integrations.

API keys use Valve's sensitive `x-webapi-key` header, and redirects are disabled. Public errors and client Debug output omit URLs, tickets, keys, raw responses, and free-form server text. Owned key/ticket strings are zeroized on drop; copies inside HTTP libraries or OS buffers are not guaranteed to be cleared. `OpaqueResponse` may contain sensitive data; do not log its JSON. Custom base URLs must be trusted and use HTTPS, except for loopback HTTP in tests.

<a id="rust-toolchain-compatibility"></a>

## Rust toolchain compatibility

Use the latest stable Rust toolchain and edition 2024. Local validation used stable **1.99.1**. The current locked ICU dependencies declare Rust 1.88 as their minimum; the project does not promise an independently tested minimum compiler version. CI tracks stable on Linux, macOS, and Windows.

## Dependencies and validation

Dependencies were checked against the latest non-yanked crates.io versions on **2026-10-21**, including prereleases, with Cargo.lock retained. Rustls uses **0.23.46**, the latest version compatible with reqwest; the newer **0.24.0-dev.2** preview has incompatible types and is the documented exception to the highest-version requirement. Other direct dependencies use the highest semantic version found, with no newer rc/beta currently available. An explicit per-client AWS-LC provider prevents a host application's Rustls global provider from overriding this choice. See the [dependency record](docs/dependencies.md).

```sh
cargo fmt --all --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --all-features --locked
cargo doc --no-deps --all-features --locked
python3 scripts/check_repository.py
python3 -m unittest discover -s scripts/tests -v
cargo run --example current_players -- 480
```

Local mock tests cover every implemented method's path, version, parameters, and response, plus retries, cooldowns, timeouts, cancellation, batching, integer precision, and redaction. They need no Steam key but require local TCP binding permission. Passing them does not establish live Publisher authentication, writes, or production concurrency. Repository checks cover bilingual links, licensing, reference inventory, and the AWS-LC dependency graph.

## Roadmap

The [complete roadmap](docs/roadmap.md) inventories all **33 documented Web API interfaces**, their documented method signatures, and the `steamworks-rs` source modules. Planned versions are sequencing proposals, not release dates or implemented functionality.

| Phase | Planned scope |
| --- | --- |
| P0 / 0.1 | Current 18 bindings; Publisher validation, documentation and release foundation |
| P1 / 0.2 | Web API discovery, apps, players, store, reviews, news, server administration and leaderboards |
| P2 / 0.3 | Cloud, inventory, economy, trading and Workshop interfaces |
| P3 / 0.4 | Microtransactions in Sandbox first, anti-cheat, community, sessions and lobby services |
| P4 / 0.5 | Publisher finance, site licenses, broadcasting and full public-reference coverage |
| P5 / separate proposal | Optional native SDK interoperability for capabilities without an HTTP counterpart |

Each phase requires explicit method/version coverage, read/write replay classification, typed contracts where documented, error fixtures, and credentialed validation before claiming production readiness. Native rendering, input, P2P transport, and callback execution require a local SDK integration; see the SDK mapping for each module.

## Releases and changelog

[CHANGELOG.md](CHANGELOG.md) records changes; [release instructions](docs/releases.md) describe tag/version checks, default manual dry runs, shared CI, package checksums, crates.io publication, and GitHub release notes. The release workflow requires a configured `CARGO_REGISTRY_TOKEN` for actual publication.

## References and license

[Overview](https://partner.steamgames.com/doc/webapi_overview) · [API keys](https://partner.steamgames.com/doc/webapi_overview/auth) · [Responses](https://partner.steamgames.com/doc/webapi_overview/responses) · [ISteamUserAuth](https://partner.steamgames.com/doc/webapi/ISteamUserAuth) · [ISteamUserStats](https://partner.steamgames.com/doc/webapi/ISteamUserStats) · [ISteamUser](https://partner.steamgames.com/doc/webapi/ISteamUser) · [Full Web API reference](https://partner.steamgames.com/doc/webapi)

Licensed under [Apache-2.0](LICENSE).
