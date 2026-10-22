<!--
Copyright 2026 steam-api-web contributors.
SPDX-License-Identifier: Apache-2.0
Licensed under the Apache License, Version 2.0.
See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.
-->

# steam-api-web

**面向 Rust 服务与游戏后端的异步 Steam Web API 客户端。**

[![Crates.io](https://img.shields.io/crates/v/steam-api-web.svg)](https://crates.io/crates/steam-api-web)
[![Documentation](https://docs.rs/steam-api-web/badge.svg)](https://docs.rs/steam-api-web)
[![CI](https://github.com/houseme/steam-api-web/actions/workflows/ci.yml/badge.svg)](https://github.com/houseme/steam-api-web/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-stable-orange.svg)](#rust-toolchain-compatibility)

[English](README.md) | 简体中文

通过一个可复用的 HTTP 客户端接入认证、玩家数据、Cloud、库存、交易、Workshop 与合作伙伴业务。
独立于 Steam 客户端和原生 SDK 运行，每个客户端显式配置 **Rustls + AWS-LC**。

**33 个接口族 · 167 条方法路径 · 38 个可运行示例**

- **资源可控：**共享连接池、并发限制、请求节奏、冷却、响应大小限制和操作截止时间。
- **契约明确：**逐方法定义凭据种类、端点版本与重放规则；未公开 schema 的响应保留原始 JSON。
- **数据准确：**保留完整宽度 SteamID 和整数金额，错误脱敏，自有凭据与票据字符串在释放时清零。

[快速开始](#quick-start) · [接口覆盖](#api-coverage) · [凭据](#credentials) ·
[可靠性](#reliability-and-performance) · [文档导航](#documentation) · [路线图](#roadmap)

> **版本状态：**`main` 面向 **0.5.0，目前尚未发布**。上述覆盖范围对应当前开发源码。
> crates.io 徽章与 docs.rs 指向已发布版本；下方分别给出 registry 和开发版的安装方式。

<a id="quick-start"></a>

## 快速开始

在应用中安装已发布版本：

```sh
cargo add steam-api-web
cargo add tokio --features macros,rt-multi-thread
```

<details>
<summary>从 GitHub 使用 0.5.0 开发版 API</summary>

将上面的 registry 依赖替换为以下配置：

```toml
[dependencies]
steam-api-web = { git = "https://github.com/houseme/steam-api-web", branch = "main" }
tokio = { version = "1.54.1", features = ["macros", "rt-multi-thread"] }
```

</details>

无需 API key，即可查询公开接口：

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

在仓库 checkout 中可运行 `cargo run --example current_players -- 480`。
跨操作复用或 clone `SteamClient`：所有 clone 共享连接池、限制、冷却与指标。
完整配置见[客户端配置示例](examples/client_configuration.rs)。

<a id="api-coverage"></a>

## 接口覆盖

按业务场景选择接入入口：

| 能力 | 已实现操作 | 示例 |
| --- | --- | --- |
| 身份与玩家数据 | 票据验证、所有权、资料、封禁、成就、统计与游戏库 | [验证票据](examples/verify_ticket.rs) |
| 应用与接口发现 | 应用/构建管理、目录、评论、新闻与 API 发现 | [查询商店目录](examples/store_catalog.rs) |
| 服务器与排名 | 服务器账号、历史玩家统计与排行榜 | [读取排行榜](examples/leaderboard_list.rs) |
| Cloud 与经济系统 | Cloud 分阶段上传、库存、资产经济、交易与市场数据 | [读取 Cloud 文件](examples/cloud_files.rs) |
| Workshop 与 UGC | 搜索、元数据、投票、审核、远程文件与贡献者规则 | [查询 Workshop](examples/workshop_query.rs) |
| 计费 | 显式区分 Sandbox/生产交易、退款、协议与报表 | [查询 Sandbox](examples/query_sandbox_transaction.rs) |
| 信任与会话 | 反作弊、滥用举报、通知与大厅操作 | [读取作弊举报](examples/cheating_reports.rs) |
| 合作伙伴业务 | 销售/愿望单报表、PC Cafe 活动与广播帧 | [读取财务变更](examples/financial_changed_dates.rs) |

已绑定所审查合作伙伴参考快照中的 **157 条显式路径 + 10 条继承 Sandbox 路径**。
该数字表示 HTTP 绑定覆盖；凭据权限与实际业务结果需要独立验收。
[完整示例指南](https://github.com/houseme/steam-api-web/blob/main/docs/examples_ZH.md)将全部 33 个接口族映射到示例程序；
[API 审计](https://github.com/houseme/steam-api-web/blob/main/docs/api-audit_ZH.md)记录响应形式、契约证据与剩余工作。

<a id="credentials"></a>

## 凭据

| 访问方式 | 配置 | 典型用途 |
| --- | --- | --- |
| 匿名 | `SteamClient::builder().build()?` | 公开玩家数、目录与接口发现 |
| User API key | `ApiKey::user(key)?` | 账号允许访问的 User-key 方法 |
| Publisher API key | `ApiKey::publisher(key)?` | 可信后端与应用管理 |
| Financial API Group key | 在独立客户端中使用 `ApiKey::financial(key)?` | 合作伙伴范围的财务报表 |
| Cloud OAuth | 向 Cloud 方法传入 `cloud::OAuthAccessToken::new(token)?` | 用户授权的 Cloud 操作 |

匿名与 User-key 客户端默认使用 Public；Publisher 与 Financial 客户端默认使用 Partner。
主机选择显式可控，本地 key 种类校验与 Steam 的应用/用户组/IP 权限校验分开处理。
API key 通过 `x-webapi-key` 请求头传输。Publisher 与 Financial key 应保存在可信服务端。

后端票据验证从运行环境读取秘密信息：

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

identity 必须与创建原生 Web API 票据时使用的值相同。
应用授权应使用返回的、经过 Steam 验证的 ID。
详见[凭据与互操作指南](https://github.com/houseme/steam-api-web/blob/main/docs/usage_ZH.md)。

<a id="reliability-and-performance"></a>

## 可靠性与性能

共享客户端执行以下默认配置，所有设置均可调整：

| 控制项 | 默认值 |
| --- | --- |
| 连接 / 单次请求超时 | 3 秒 / 10 秒 |
| 整体操作截止时间 | 30 秒，包含排队、请求节奏与退避 |
| 并发在途请求 | 32，clone 之间共享 |
| 逻辑响应正文上限 | 8 MiB，同时检查响应头与流式累计长度 |
| 可安全重放的读取重试 | 额外 2 次；初始退避 200 毫秒 / 最大 2 秒 |
| 最小请求间隔 | 0；按实际 Steam 配额配置 |

- **重放规则：**仅对可安全重放的 GET 传输错误和 HTTP 408/429/500/502/503/504 重试。
  认证与所有修改操作均单次发送，包括带副作用的 GET。结果不明确的写入应先核对，再决定是否重新提交。
- **共享限制：**`Retry-After` 建立共享冷却，取消会释放并发许可；
  `metrics()` 提供尝试、重试和已完成调用的聚合指标。
- **响应契约：**JSON 与业务错误显式返回。已公开 schema 使用类型化模型；
  其他响应通过 `OpaqueResponse` 保留 JSON，不推断业务成功，可用 `as_json()` 读取。
- **传输与数据：**逐客户端使用 AWS-LC，禁用重定向，精确解码整数，并返回脱敏的结构化错误。
  自有秘密字符串在释放时清零；HTTP 库与操作系统中的外部副本不在这一保证范围内。

编码路径将标量直接写入最终表单。单块响应保留传输缓冲，多块响应按逻辑正文大小限制合并。
可复现的 CPU 测量及适用范围见[全局请求链路审查](https://github.com/houseme/steam-api-web/blob/main/docs/review-global_ZH.md)。
分布式配额、业务结果核对与部署 SLA 由应用管理。

<a id="documentation"></a>

## 文档导航

| 入口 | 延伸阅读 |
| --- | --- |
| [Rust API 参考：已发布版本](https://docs.rs/steam-api-web) | [使用、配置与接口访问器矩阵](https://github.com/houseme/steam-api-web/blob/main/docs/usage_ZH.md) |
| [全部 33 个接口族的示例](https://github.com/houseme/steam-api-web/blob/main/docs/examples_ZH.md) | [Cloud/经济/Workshop 契约](https://github.com/houseme/steam-api-web/blob/main/docs/p2_ZH.md) · [计费/信任/会话契约](https://github.com/houseme/steam-api-web/blob/main/docs/p3_ZH.md) · [合作伙伴契约](https://github.com/houseme/steam-api-web/blob/main/docs/p4_ZH.md) |
| [功能路线图与完整清单](https://github.com/houseme/steam-api-web/blob/main/docs/roadmap_ZH.md) | [API 完整性审计](https://github.com/houseme/steam-api-web/blob/main/docs/api-audit_ZH.md) |
| [架构与验证设计](https://github.com/houseme/steam-api-web/blob/main/docs/design_ZH.md) | [原生互操作提案](https://github.com/houseme/steam-api-web/blob/main/docs/p5-design_ZH.md) |
| [性能审查与测量](https://github.com/houseme/steam-api-web/blob/main/docs/performance_ZH.md) | [全局请求链路审查](https://github.com/houseme/steam-api-web/blob/main/docs/review-global_ZH.md) |
| [依赖策略与兼容性](https://github.com/houseme/steam-api-web/blob/main/docs/dependencies_ZH.md) | [发布与归档验证](https://github.com/houseme/steam-api-web/blob/main/docs/releases_ZH.md) · [变更记录](CHANGELOG_ZH.md) |

运行 `cargo doc --no-deps --open` 可浏览当前 checkout 对应的 API 文档。
指南提供中英文版本；仅保存在仓库的文档使用 GitHub 链接，以便从发布 crate 中继续访问。

<a id="roadmap"></a>

## 路线图

| 方向 | 当前状态 | 下一验收门槛 |
| --- | --- | --- |
| P0–P4 Web API 绑定 | 33 个接口族 / 167 条路径已实现，具备本地协议测试 | 授权环境的读取/权限验收，以及受保护写入与回读 |
| 响应模型 | 有公开 schema 的返回类型化结果，其余保留经校验的 JSON | 权威 schema 可用时扩展模型 |
| P5 原生 companion | 设计已交付，尚未开始实现 | 票据生命周期、回调所有权与平台运行时验收 |
| P6 接口发现扩展 | 已清点 53 条仅发现阶段的候选签名 | 绑定前审查受支持契约、凭据、重放与生命周期 |

[steamworks-rs](https://github.com/Noxime/steamworks-rs) 为应用提供原生 SDK 访问，
本 crate 为服务提供 HTTP 接口。跨边界传递原始 ID 与经过长度校验的票据字节。
规划中的原生 companion 与当前独立于 SDK 的 crate 保持分离。
完整实施顺序与原生模块对照见[路线图](https://github.com/houseme/steam-api-web/blob/main/docs/roadmap_ZH.md)。

## 开发与发布包

在仓库 checkout 中运行：

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
cargo xtask check-repository
cargo xtask coverage --json
cargo xtask audit-api --json
cargo xtask release verify-package
```

本地协议测试使用回环 HTTP fixture，无需 Steam 凭据。
CI 在 Linux、macOS、Windows 上构建和测试，检查文档，并验证 crate 归档可独立构建。
上述命令不执行线上 Steam 操作；可选的 live 审计见 API 审计指南。

全部直接依赖由根 workspace manifest 统一管理，维护工具使用 Rust `xtask`。
crate 白名单保留库源码、示例、中英文 README/CHANGELOG、许可证与 Cargo 元数据；
测试、基准、workflow、维护工具、指南与审计快照仅保留在 GitHub 仓库中。

<a id="rust-toolchain"></a>
<a id="rust-toolchain-compatibility"></a>

## Rust 工具链兼容性

使用最新 **stable Rust** 与 **edition 2024**，三个平台的 CI 均跟踪 stable。
目前不承诺独立验证过的最低 Rust 版本；详见[工具链与依赖说明](https://github.com/houseme/steam-api-web/blob/main/docs/usage_ZH.md#rust-工具链兼容性)。

## 许可证与参考

采用 [Apache-2.0](LICENSE) 许可证。

[Steam Web API 概览](https://partner.steamgames.com/doc/webapi_overview) ·
[API 认证](https://partner.steamgames.com/doc/webapi_overview/auth) ·
[合作伙伴 Web API 参考](https://partner.steamgames.com/doc/webapi)
