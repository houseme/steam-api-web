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

[English](README.md) | 简体中文

独立异步 Rust Steam Web API 客户端——无需 Steam 客户端与原生 SDK。**0.4.0 开发版**绑定 P0–P3 共 **30 个接口、161 条方法路径**，覆盖认证、统计、Cloud、库存、交易、Workshop、微交易与反作弊。

**为什么选择这个 crate**

- **资源占用有界**——共享并发许可、请求节奏、服务端冷却、响应大小上限，以及每个操作的整体截止时间。
- **保守重试**——仅对可安全重放的 GET 传输错误及 HTTP 408/429/500/502/503/504 重试；认证与所有写入均单次发送。
- **严格的响应契约**——Valve 公开 schema 的方法返回类型化结果；未公开的保留原始 JSON，同时仍检查业务错误。
- **安全卫生**——key 与票据脱敏、清零，绝不进入错误信息；错误不含 URL、响应体或服务器自由文本。
- **数值无损**——完整 64 位 SteamID、微交易金额与统计值在编解码中不截断。

## 快速开始

0.4.0 尚未发布；在版本进入 registry 之前，可从 GitHub checkout 使用 `{ path = "../steam-api-web" }` 路径依赖。

```toml
[dependencies]
steam-api-web = "0.4.0"
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

复用或 clone `SteamClient`：所有 clone 共享连接池、并发限制、请求节奏、服务端冷却和指标。可运行示例：[当前玩家数](examples/current_players.rs)、[票据验证](examples/verify_ticket.rs)、[Cloud 文件](examples/cloud_files.rs)、[Sandbox 交易查询](examples/query_sandbox_transaction.rs)。

## 接口覆盖

| 阶段 | 版本 | 接口数 | 路径数 | 范围 |
| --- | --- | ---: | ---: | --- |
| P0 | 0.1 | 3 | 18 | 认证、资料、所有权、封禁、统计 |
| P1 | 0.2 | 9 | 39 | 发现、应用、商店、评论、新闻、服务器、排行榜 |
| P2 | 0.3 | 12 | 66 | Cloud、库存、经济、交易、Workshop |
| P3 | 0.4 | 6 | 38 | 计费、反作弊、社区、通知、大厅 |

总计包括 151 条显式文档路径和 10 条继承 Sandbox 路径。逐方法的路径、版本、凭据种类与重放规则见[使用指南](https://github.com/houseme/steam-api-web/blob/main/docs/usage_ZH.md)与完整[路线图清单](https://github.com/houseme/steam-api-web/blob/main/docs/roadmap_ZH.md)。

## 文档

| 主题 | 文档 |
| --- | --- |
| 配置、可靠性、数据处理、steamworks-rs 互操作 | [使用指南](https://github.com/houseme/steam-api-web/blob/main/docs/usage_ZH.md) |
| 设计分析与验证层次 | [设计分析](https://github.com/houseme/steam-api-web/blob/main/docs/design_ZH.md) |
| Cloud、库存、经济与 Workshop 契约 | [P2 契约](https://github.com/houseme/steam-api-web/blob/main/docs/p2_ZH.md) |
| 计费、反作弊与会话契约 | [P3 契约](https://github.com/houseme/steam-api-web/blob/main/docs/p3_ZH.md) |
| 路线图、方法清单与 steamworks-rs 模块映射 | [路线图](https://github.com/houseme/steam-api-web/blob/main/docs/roadmap_ZH.md) |
| 传输层 Review 与编解码测量 | [性能](https://github.com/houseme/steam-api-web/blob/main/docs/performance_ZH.md) |
| 依赖审计 | [依赖记录](https://github.com/houseme/steam-api-web/blob/main/docs/dependencies_ZH.md) |
| 发布流程 | [发布说明](https://github.com/houseme/steam-api-web/blob/main/docs/releases_ZH.md) |

变更记录见 [CHANGELOG_ZH.md](CHANGELOG_ZH.md)。

<a id="rust-toolchain"></a>

## Rust 工具链

使用最新 stable Rust 与 edition 2024；CI 跟踪 Linux/macOS/Windows。锁定依赖的最低版本见[使用指南](https://github.com/houseme/steam-api-web/blob/main/docs/usage_ZH.md#rust-toolchain-compatibility)。

## 参考

[Steam Web API 概览](https://partner.steamgames.com/doc/webapi_overview) · [API key](https://partner.steamgames.com/doc/webapi_overview/auth) · [完整 Web API 参考](https://partner.steamgames.com/doc/webapi)

许可：[Apache-2.0](LICENSE)。
