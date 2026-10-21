<!--
Copyright 2026 steam-api-web contributors.
SPDX-License-Identifier: Apache-2.0
Licensed under the Apache License, Version 2.0.
See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.
-->

# steam-api-web

[English](README.md) | 简体中文

独立异步 Rust crate，封装 ISteamUserAuth、ISteamUserStats、ISteamUser 的 18 个文档方法。
使用 Tokio、reqwest 和逐客户端显式配置的 AWS-LC/Rustls，无需原生 Steamworks SDK。

P0 包含票据验证、所有权与资料查询、统计和成就操作、连接池、响应及并发上限、共享请求节奏、
Retry-After 冷却和保守重试。认证及写入不会自动重放；Publisher 实测尚未完成，
未公开响应结构的方法保留原始 JSON。

```sh
cargo test --all-features --locked
cargo run --example current_players -- 480
```

许可：[Apache-2.0](LICENSE)。
