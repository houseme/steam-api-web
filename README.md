<!--
Copyright 2026 steam-api-web contributors.
SPDX-License-Identifier: Apache-2.0
Licensed under the Apache License, Version 2.0.
See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.
-->

# steam-api-web

**Async Steam Web API bindings for Rust services and game backends.**

[![Crates.io](https://img.shields.io/crates/v/steam-api-web.svg)](https://crates.io/crates/steam-api-web)
[![Documentation](https://docs.rs/steam-api-web/badge.svg)](https://docs.rs/steam-api-web)
[![CI](https://github.com/houseme/steam-api-web/actions/workflows/ci.yml/badge.svg)](https://github.com/houseme/steam-api-web/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-stable-orange.svg)](#rust-toolchain-compatibility)

English | [简体中文](README_ZH.md)

Connect authentication, player data, Cloud, inventory, trading, Workshop and
partner operations through one reusable HTTP client. Runs independently of the
Steam client and native SDK, with **Rustls + AWS-LC** configured per client.

**33 interface families · 167 method routes · 38 runnable examples**

- **Controlled resource use:** shared connection pooling, concurrency limits,
  pacing, cooldowns, response limits and operation deadlines.
- **Explicit contracts:** credential kinds, endpoint versions and replay rules
  are defined per method; undocumented payloads retain their original JSON.
- **Exact data handling:** full-width SteamIDs and integer amounts, redacted
  errors, and zeroization of owned credential and ticket strings.

[Quick start](#quick-start) · [API coverage](#api-coverage) ·
[Credentials](#credentials) · [Reliability](#reliability-and-performance) ·
[Documentation](#documentation) · [Roadmap](#roadmap)

> **Version status:** `main` targets **0.5.0, currently unreleased**. The coverage
> above describes this development tree. The registry badge and docs.rs refer to
> published releases; installation choices are shown below.

## Quick start

Install a published release in your application:

```sh
cargo add steam-api-web
cargo add tokio --features macros,rt-multi-thread
```

<details>
<summary>Use the 0.5.0 development APIs from GitHub</summary>

Use this dependency configuration instead of the registry dependency above:

```toml
[dependencies]
steam-api-web = { git = "https://github.com/houseme/steam-api-web", branch = "main" }
tokio = { version = "1.54.1", features = ["macros", "rt-multi-thread"] }
```

</details>

Query a public endpoint without an API key:

```rust
use steam_api_web::SteamClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let steam = SteamClient::builder().build()?;
    let players = steam
        .user_stats()
        .get_number_of_current_players(480)
        .await?;
    println!("Online players: {}", players.player_count);
    Ok(())
}
```

From a repository checkout, run `cargo run --example current_players -- 480`.
Reuse or clone `SteamClient` across operations: clones share the pool, limits,
cooldowns and metrics. See [client configuration](examples/client_configuration.rs)
for a complete configuration example.

## API coverage

Choose an entry point by the feature you are building:

| Capability | Included operations | Example |
| --- | --- | --- |
| Identity and player data | Ticket verification, ownership, profiles, bans, achievements, statistics and libraries | [Verify a ticket](examples/verify_ticket.rs) |
| Apps and discovery | App/build administration, catalogue, reviews, news and API discovery | [Browse the catalogue](examples/store_catalog.rs) |
| Servers and rankings | Server accounts, historical player statistics and leaderboards | [List leaderboards](examples/leaderboard_list.rs) |
| Cloud and economy | Staged Cloud uploads, inventory, asset economy, trades and market data | [Read Cloud files](examples/cloud_files.rs) |
| Workshop and UGC | Search, metadata, votes, moderation, remote files and contributor rules | [Query Workshop](examples/workshop_query.rs) |
| Billing | Explicit Sandbox/production transactions, refunds, agreements and reports | [Query Sandbox](examples/query_sandbox_transaction.rs) |
| Trust and sessions | Anti-cheat, abuse reports, notifications and lobby operations | [Read cheat reports](examples/cheating_reports.rs) |
| Partner operations | Sales/wishlist reports, PC Cafe activity and broadcast frames | [Read financial changes](examples/financial_changed_dates.rs) |

All **157 explicit routes + 10 inherited Sandbox routes** in the reviewed
partner-reference snapshot are bound. This measures HTTP binding coverage;
credentialed permissions and business outcomes require separate acceptance.
The [example guide](https://github.com/houseme/steam-api-web/blob/main/docs/examples.md)
maps all 33 interfaces to programs, and the
[API audit](https://github.com/houseme/steam-api-web/blob/main/docs/api-audit.md)
records response forms, contract evidence and remaining work.

## Credentials

| Access | Configuration | Typical use |
| --- | --- | --- |
| Anonymous | `SteamClient::builder().build()?` | Public counts, catalogue and discovery |
| User API key | `ApiKey::user(key)?` | User-key methods allowed by the account |
| Publisher API key | `ApiKey::publisher(key)?` | Trusted backend and app administration |
| Financial API Group key | `ApiKey::financial(key)?` in a separate client | Partner-wide financial reports |
| Cloud OAuth | `cloud::OAuthAccessToken::new(token)?` passed to Cloud methods | User-authorized Cloud operations |

Anonymous and User-key clients default to Public; Publisher and Financial
clients default to Partner. Host selection is explicit, and local key-kind
validation is separate from Steam's app/group/IP permissions. API keys travel
in the `x-webapi-key` header. Keep Publisher and Financial keys on trusted servers.

For backend ticket verification, read secrets from the execution environment:

```rust
use steam_api_web::{ApiKey, SteamClient, user_auth::WebApiTicket};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let steam = SteamClient::builder()
        .api_key(ApiKey::publisher(std::env::var("STEAM_PUBLISHER_KEY")?)?)
        .build()?;
    let ticket = WebApiTicket::from_hex(std::env::var("STEAM_TICKET_HEX")?)?;
    let appid = std::env::var("STEAM_APP_ID")?.parse::<u32>()?;
    let identity = std::env::var("STEAM_TICKET_IDENTITY")?;
    let user = steam
        .user_auth()
        .authenticate_user_ticket(appid, &ticket, &identity)
        .await?;
    println!("Verified SteamID: {}", user.steamid);
    // Apply your application's ownership, ban and access policy here.
    Ok(())
}
```

The identity must match the one used when creating the native Web API ticket.
Use the returned Steam-verified ID for application authorization. See the
[credential and interoperability guide](https://github.com/houseme/steam-api-web/blob/main/docs/usage.md).

## Reliability and performance

One shared client enforces the following defaults; each setting is configurable:

| Control | Default |
| --- | --- |
| Connection / per-attempt timeout | 3 s / 10 s |
| Overall operation deadline | 30 s, including queueing, pacing and backoff |
| Concurrent in-flight requests | 32, shared by clones |
| Logical response-body limit | 8 MiB, checked in headers and while streaming |
| Replay-safe read retries | 2 additional attempts; 200 ms initial / 2 s maximum backoff |
| Minimum request interval | 0; configure for your Steam quota |

- **Replay rules:** only replay-safe GET transport failures and HTTP
  408/429/500/502/503/504 retry. Authentication and all mutations are single-send,
  including side-effecting GETs. Reconcile an ambiguous write before resubmitting.
- **Shared limits:** `Retry-After` creates a shared cooldown, cancellation releases
  permits, and `metrics()` exposes aggregate attempts, retries and completed calls.
- **Response contracts:** JSON and business errors fail explicitly. Documented
  schemas have typed models; `OpaqueResponse` preserves other JSON without
  inventing a successful business outcome. Inspect it with `as_json()`.
- **Transport and data:** per-client AWS-LC, disabled redirects, exact integer
  decoding and redacted structured errors. Owned secret strings are zeroized on
  drop; external HTTP/OS copies are outside that guarantee.

The encoding path writes scalars directly into the final form. Single-chunk
responses retain the transport buffer; multiple chunks are coalesced within the
logical body-size limit. Reproducible CPU measurements and their scope are in the
[global request review](https://github.com/houseme/steam-api-web/blob/main/docs/review-global.md).
Distributed quotas, reconciliation and deployment SLAs remain application concerns.

## Documentation

| Start here | Read next |
| --- | --- |
| [Rust API reference — published release](https://docs.rs/steam-api-web) | [Usage, configuration and accessor matrix](https://github.com/houseme/steam-api-web/blob/main/docs/usage.md) |
| [Examples for all 33 interfaces](https://github.com/houseme/steam-api-web/blob/main/docs/examples.md) | [Cloud/economy/Workshop contracts](https://github.com/houseme/steam-api-web/blob/main/docs/p2.md) · [Billing/trust/session contracts](https://github.com/houseme/steam-api-web/blob/main/docs/p3.md) · [Partner contracts](https://github.com/houseme/steam-api-web/blob/main/docs/p4.md) |
| [Feature roadmap and complete inventory](https://github.com/houseme/steam-api-web/blob/main/docs/roadmap.md) | [API completeness audit](https://github.com/houseme/steam-api-web/blob/main/docs/api-audit.md) |
| [Architecture and verification design](https://github.com/houseme/steam-api-web/blob/main/docs/design.md) | [Native interoperability proposal](https://github.com/houseme/steam-api-web/blob/main/docs/p5-design.md) |
| [Performance reviews and measurements](https://github.com/houseme/steam-api-web/blob/main/docs/performance.md) | [Global request-path review](https://github.com/houseme/steam-api-web/blob/main/docs/review-global.md) |
| [Dependency policy and compatibility](https://github.com/houseme/steam-api-web/blob/main/docs/dependencies.md) | [Release and archive verification](https://github.com/houseme/steam-api-web/blob/main/docs/releases.md) · [Changelog](CHANGELOG.md) |

To browse API documentation for your checkout, run `cargo doc --no-deps --open`.
Guides have English and Chinese editions; repository-only documents use GitHub
links so they remain accessible from the published crate.

## Roadmap

| Track | Current state | Next gate |
| --- | --- | --- |
| P0–P4 Web API bindings | 33 interfaces / 167 routes implemented with local protocol tests | Authorized read/permission acceptance and protected write/read-back |
| Response models | Typed results where schemas are documented; validated opaque JSON elsewhere | Extend models when authoritative schemas become available |
| P5 native companion | Design delivered; implementation not started | Ticket lifetimes, callback ownership and platform runtime acceptance |
| P6 discovery expansion | 53 discovery-only signatures inventoried as candidates | Review supported contracts, credentials, replay and lifecycle before binding |

[steamworks-rs](https://github.com/Noxime/steamworks-rs) provides native SDK access
for applications; this crate provides the HTTP side for services. Pass raw IDs
and validated ticket bytes across that boundary. The proposed native companion
remains separate from this SDK-independent crate. Full sequencing and the native
module comparison live in the [roadmap](https://github.com/houseme/steam-api-web/blob/main/docs/roadmap.md).

## Development and packaging

Run repository checks from a checkout:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
cargo xtask check-repository
cargo xtask coverage --json
cargo xtask audit-api --json
cargo xtask release verify-package
```

Local protocol tests use loopback HTTP fixtures and require no Steam credentials.
CI builds/tests on Linux, macOS and Windows, checks documentation and validates an
independently buildable crate archive. The commands above do not exercise live
Steam operations; the optional live audit is described in the API audit guide.

All direct dependencies are managed in the root workspace manifest. Maintenance
uses Rust `xtask`. The crate allowlist retains library sources, examples,
bilingual READMEs/changelogs, the license and Cargo metadata. Tests, benchmarks,
workflows, maintenance tools, guides and audit snapshots stay in the GitHub repository.

<a id="rust-toolchain"></a>

## Rust toolchain compatibility

Use the latest **stable Rust**, edition **2024**. CI follows stable on all three
platforms. An independently tested minimum Rust version is not promised; see the
[toolchain and dependency notes](https://github.com/houseme/steam-api-web/blob/main/docs/usage.md#rust-toolchain-compatibility).

## License and references

Licensed under [Apache-2.0](LICENSE).

[Steam Web API overview](https://partner.steamgames.com/doc/webapi_overview) ·
[API authentication](https://partner.steamgames.com/doc/webapi_overview/auth) ·
[Partner Web API reference](https://partner.steamgames.com/doc/webapi)
