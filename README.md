<!--
Copyright 2026 steam-api-web contributors.
SPDX-License-Identifier: Apache-2.0
Licensed under the Apache License, Version 2.0.
See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.
-->

# steam-api-web

[![Crates.io Version](https://img.shields.io/crates/v/steam-api-web)](https://crates.io/crates/steam-api-web)
[![Documentation](https://docs.rs/steam-api-web/badge.svg)](https://docs.rs/steam-api-web)
[![CI](https://github.com/houseme/steam-api-web/actions/workflows/ci.yml/badge.svg)](https://github.com/houseme/steam-api-web/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-stable-orange.svg)](#rust-toolchain)

English | [简体中文](README_ZH.md)

An independent asynchronous Rust client for Steam's Web API — no Steam client,
no native SDK. The **0.5.0 development version** binds **167 method routes
across 33 interfaces** (P0–P4), from authentication and stats to Cloud,
inventory, trading, Workshop, microtransactions, anti-cheat, financial reports,
site licensing and broadcasts.

**Why this crate**

- **Bounded by design** — shared concurrency permits, request pacing, server
  cooldowns, response-size caps and an overall deadline on every operation.
- **Conservative retries** — only replay-safe GET transport failures and HTTP
  408/429/500/502/503/504 retry; authentication and every write are single-send.
- **Strict response contracts** — typed results where Valve documents a schema;
  otherwise the original JSON is preserved while business errors are still checked.
- **Security hygiene** — keys and tickets are redacted, zeroized and never leak
  into errors; errors carry no URLs, bodies or server text.
- **Lossless values** — full 64-bit SteamIDs, microtransaction amounts and
  statistic values survive encoding and decoding without narrowing.

## Quick start

Version 0.5.0 is unreleased; until it reaches the registry, use a
`{ path = "../steam-api-web" }` dependency from a GitHub checkout.

```toml
[dependencies]
steam-api-web = "0.5.0"
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

Reuse or clone `SteamClient`: all clones share the connection pool, concurrency
limit, request pacing, server cooldown and metrics. Runnable examples:
[current players](examples/current_players.rs), [ticket verification](examples/verify_ticket.rs),
[Cloud files](examples/cloud_files.rs), [Sandbox transaction query](examples/query_sandbox_transaction.rs),
[financial change batch](examples/financial_changed_dates.rs). The [complete example guide](https://github.com/houseme/steam-api-web/blob/main/docs/examples.md) maps runnable programs to all 33 interface families.

## API coverage

| Phase | Version | Interfaces | Routes | Scope |
| --- | --- | ---: | ---: | --- |
| P0 | 0.1 | 3 | 18 | Authentication, profiles, ownership, bans, stats |
| P1 | 0.2 | 9 | 39 | Discovery, apps, store, reviews, news, servers, leaderboards |
| P2 | 0.3 | 12 | 66 | Cloud, inventory, economy, trading, Workshop |
| P3 | 0.4 | 6 | 38 | Billing, anti-cheat, community, notifications, lobbies |
| P4 | 0.5 | 3 | 6 | Financial reports, site activity, broadcasts |

Totals include 157 explicit documented routes and ten inherited Sandbox routes.
Financial endpoints require a separate `ApiKey::financial(...)` client.
`cargo xtask coverage --json` reports source-checked mappings and remaining routes.
Per-method paths, versions, credential kinds and replay rules live in the
[usage guide](https://github.com/houseme/steam-api-web/blob/main/docs/usage.md)
and the complete [roadmap inventory](https://github.com/houseme/steam-api-web/blob/main/docs/roadmap.md).

## Documentation

| Topic | Document |
| --- | --- |
| Runnable examples for every interface and documentation checks | [Examples](https://github.com/houseme/steam-api-web/blob/main/docs/examples.md) |
| Configuration, reliability, data handling, steamworks-rs interop | [Usage guide](https://github.com/houseme/steam-api-web/blob/main/docs/usage.md) |
| Design analysis and verification layers | [Design](https://github.com/houseme/steam-api-web/blob/main/docs/design.md) |
| Cloud, inventory, economy and Workshop contracts | [P2 contracts](https://github.com/houseme/steam-api-web/blob/main/docs/p2.md) |
| Billing, anti-cheat and session contracts | [P3 contracts](https://github.com/houseme/steam-api-web/blob/main/docs/p3.md) |
| Financial authority, site reports, broadcasts and coverage | [P4 contracts](https://github.com/houseme/steam-api-web/blob/main/docs/p4.md) |
| Roadmap, inventory and steamworks-rs module map | [Roadmap](https://github.com/houseme/steam-api-web/blob/main/docs/roadmap.md) |
| Feature completeness, discovery candidates and remaining acceptance | [API audit](https://github.com/houseme/steam-api-web/blob/main/docs/api-audit.md) |
| Native SDK interoperability design and implementation gates | [P5 design](https://github.com/houseme/steam-api-web/blob/main/docs/p5-design.md) |
| Transport reviews and codec measurements | [Performance](https://github.com/houseme/steam-api-web/blob/main/docs/performance.md) |
| Dependency audit | [Dependencies](https://github.com/houseme/steam-api-web/blob/main/docs/dependencies.md) |
| Release process | [Releases](https://github.com/houseme/steam-api-web/blob/main/docs/releases.md) |

Changes are recorded in [CHANGELOG.md](CHANGELOG.md).

<a id="rust-toolchain"></a>

## Rust toolchain

Latest stable Rust with edition 2024; CI tracks Linux, macOS and Windows. See
the [usage guide](https://github.com/houseme/steam-api-web/blob/main/docs/usage.md#rust-toolchain-compatibility)
for the locked-dependency minimum.

## References and license

[Steam Web API overview](https://partner.steamgames.com/doc/webapi_overview) · [API keys](https://partner.steamgames.com/doc/webapi_overview/auth) · [Full Web API reference](https://partner.steamgames.com/doc/webapi)

Licensed under [Apache-2.0](LICENSE).
