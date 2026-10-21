<!--
Copyright 2026 steam-api-web contributors.
SPDX-License-Identifier: Apache-2.0
Licensed under the Apache License, Version 2.0.
See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.
-->

# steam-api-web

English | [简体中文](README_ZH.md)

Independent asynchronous Rust bindings for the 18 documented methods in
ISteamUserAuth, ISteamUserStats and ISteamUser. Uses Tokio, reqwest and an explicit
per-client AWS-LC/Rustls provider without requiring the native Steamworks SDK.

The P0 implementation includes ticket verification, ownership and profile reads,
stat and achievement methods, pooled connections, bounded responses/concurrency,
shared request pacing, Retry-After cooldowns and conservative retry policies.
Authentication and writes are not replayed automatically. Publisher-only live
validation remains pending; undocumented response schemas retain opaque JSON.

```sh
cargo test --all-features --locked
cargo run --example current_players -- 480
```

Licensed under [Apache-2.0](LICENSE).
