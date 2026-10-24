<!--
Copyright 2026 steam-api-web contributors.
SPDX-License-Identifier: Apache-2.0
Licensed under the Apache License, Version 2.0.
See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.
-->

# 使用指南

[English](usage.md) | 简体中文

`steam-api-web` 的详细使用说明。仓库 README 仅保留快速开始与覆盖摘要；本指南包含配置、可靠性、数据处理、原生 SDK 互操作、工具链与打包说明。

## 凭据与域名

API key 通过敏感 `x-webapi-key` header 传递，重定向已关闭。Publisher 和 Financial API
Group 客户端默认选择推荐的 Partner，匿名与 User key 客户端默认使用 Public；可通过
`host(Host::Public/Partner)` 显式选择，配置后主机固定，不自动切换。真实组权限、
AppID 所属及 IP 白名单仍由 Steam 校验。

每个方法的最低凭据（无 / User / Publisher / Financial）在发送前本地检查。财务方法
要求独立 `ApiKey::financial(...)` 客户端，专属组 key 拥有伙伴账户范围财报可见性，
在这里不能满足其他凭据类别。声明 key 种类不等于已获远端授权；财务默认使用 Partner。
Site 方法页展示 Public，可由 Publisher 客户端显式选择；概览仍推荐可信后端使用 Partner。

方法使用 snake_case，`cargo doc --no-deps --open` 可查看完整 API。[P4 契约](p4_ZH.md)
说明精确金额、独立游标、PT/GMT 日期、Site wire 假设及广播单次发送。

## API 全貌

| 接口 | Rust 入口 | 路径数 |
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
| ISteamMicroTxnSandbox | `micro_txn_sandbox()` | 10（继承） |
| IPartnerFinancialsService | `partner_financials()` | 3 |
| ISiteLicenseService | `site_license()` | 2 |
| IBroadcastService | `broadcast()` | 1 |

逐方法清单——路径、版本、HTTP 动词、凭据种类与重放分类——见[完整路线图](roadmap_ZH.md)与机器可读的 [api-inventory.json](api-inventory.json)。

各阶段契约要点：

- **P0**——`AuthenticateUserTicket` 虽为 GET 但绝不重试。`GetPublisherAppOwnership` 默认使用明示的 v4 路径，另提供显式 v5 选项。
- **P1**——Service 接口使用 `input_json`。旧 `ISteamApps/GetAppList/v2` 已弃用，新接入使用可分页的 `IStoreService/GetAppList`。`SetAppBuildLive` 保留 HTTP 201 等待手机确认的状态，不把请求接受等同于已发布。
- **P2**——Cloud 通过用户 OAuth 授权并严格检查 `x-eresult`；上传分阶段返回元数据，文件传输与批次收尾由调用方管理。库存 requestid、并发时间戳、交易 cursor 与 Workshop 修订号原样保留。包括 GET StartTrade、GET Delete 在内的写操作不自动重放。见 [P2 契约](p2_ZH.md)。
- **P3**——微交易显式选择 Sandbox 或生产环境且不自动切换；支付确认与实际结算分开，VAC 未就绪与已验证分开。见 [P3 契约](p3_ZH.md)。

## 配置与可靠性

| 配置 | 默认值 | 作用 |
| --- | --- | --- |
| connect_timeout | 3 秒 | 建连超时 |
| request_timeout | 10 秒 | 单次 HTTP 尝试（含响应体读取）超时 |
| operation_timeout | 30 秒 | 排队、限流、网络与退避总预算 |
| max_in_flight | 32 | clone 共享并发上限 |
| max_response_bytes | 8 MiB | 同时检查 Content-Length 与实际分块累计大小 |
| RetryPolicy.max_retries | 2 | 安全 GET 的额外尝试次数，最大 10 |
| initial_backoff / max_backoff | 200 ms / 2 秒 | 指数退避，AWS-LC 随机抖动 50%..100% |
| min_request_interval | 0 | clone 共享最小请求间隔，按 Steam 配额配置 |

只对读取请求的传输错误、HTTP 408/429/500/502/503/504 重试。授权、参数、JSON 和业务错误不重试。`Retry-After` 支持秒数及 HTTP 日期，建立 clone 共享冷却；本地退避上限不会导致提前重试。超出总预算返回 `Error::Deadline`。

认证（包括 GET 票据验证）与统计写入均不自动重放；reqwest 内置重试也已关闭。写入超时可能已经生效，应先查询再决定是否补偿。取消 Future 会释放并发许可。profile 批处理按最多 100 个 ID 顺序调用，每批各有操作截止时间；整个批次的预算由调用方包一层 Tokio timeout。

`metrics()` 提供 attempts/retries/succeeded/failed 快照。校验失败与主动取消不算完成调用。集群级配额协调、故障期间业务降级、跨实例监控与远端 SLA 由部署侧负责；身份及授权结果不会被隐式缓存。

## 数据处理与安全

- SteamID 接受 JSON 数字或十进制字符串，序列化为字符串，完整保留 64 位整数。统计值保留有符号、无符号整数及浮点数。
- 可见性受限字段使用 `Option`；缺失封禁标记不会默认成"未封禁"。私密好友列表的 401 返回错误，vanity 无匹配返回 `Ok(None)`。
- 官方未公开响应 schema 的方法——包括 `AuthenticateUser`、`GetDeletedSteamIDs`、`SetUserStatsForGame`——返回保留原始 JSON 的 `OpaqueResponse`，同时仍检查标准业务错误 envelope。通过 `as_json/into_json` 显式访问，不虚构写入成功、分页结构或终止规则。`OpaqueResponse` 可能含敏感数据，不应记录其 JSON。
- 旧式 `AuthenticateUser` 运输准备好的 RSA/AES 密文，不猜测文档未给出的 RSA key、padding、AES 模式或 IV。rawbinary 按字节转义，不转 UTF-8/hex。新接入优先使用 Web API ticket。
- 自有 key/ticket 字符串释放时清零；HTTP 栈及操作系统中的复制内容不保证清零。错误与客户端 Debug 不含 URL、票据、key、原始响应和服务器自由文本。自定义 base URL 必须可信，只允许 HTTPS 及测试用 loopback HTTP。

## 与 steamworks-rs 配合

本 crate 的设计与 [Noxime/steamworks-rs](https://github.com/Noxime/steamworks-rs)（封装原生 SDK 与回调）做过对照：这里封装 HTTPS API，两者可分别运行于游戏客户端与后端。

- `steamworks::SteamId::raw()` 的 `u64` 可传入这里的用户方法；提供 `SteamId::from_raw/raw` 双向转换。
- AppID 使用 `u32`（SDK 的 `AppId(x).0`）；64 位 GameID 独立处理，不截断为 AppID。
- SDK 注册 `TicketForWebApiResponse` 回调，调用 `authentication_session_ticket_for_webapi(identity)` 并驱动 `run_callbacks()`；检查回调结果成功后，才向后端发送有效票据。
- 参考封装的 `TicketForWebApiResponse::ticket` 可能包含整个固定缓冲区。使用 `from_callback_bytes(&response.ticket, response.ticket_len)` 校验长度并截取有效前缀。

客户端回调转换示意（客户端程序自行依赖 steamworks）：

```rust,ignore
if response.result.is_ok() {
    let ticket = WebApiTicket::from_callback_bytes(&response.ticket, response.ticket_len)?;
    // Send ticket.expose_hex() to your backend over a secure connection.
}
```

后端使用与创建票据时相同的 identity：

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

SDK 的 `AuthTicket` 是取消句柄而非票据字节。P2P/服务器 `GetAuthSessionTicket`、Web API ticket 与 Encrypted App Ticket 是不同协议，不可混用。SDK 初始化、回调保活、结果检查与取消由客户端管理。详见[设计分析](design_ZH.md)。

## Rust 工具链兼容性

使用最新 stable Rust 和 edition 2024。本次本地验证使用 stable **1.99.1**。当前锁定的 ICU 依赖声明最低 Rust 1.88，但项目不承诺经过独立验证的最小编译器版本。CI 跟踪 Linux/macOS/Windows 的 stable 工具链。

## 仓库与打包

Cargo.toml 使用显式 `include` 白名单，发布包仅保留 Rust 库源码、示例、LICENSE、双语 README/changelog，以及 Cargo 必需的 manifest、lockfile 和版本元数据。`.github/`、`xtask/`、`scripts/`、`tests/`、`docs/` 和参考 JSON 留在 GitHub，不进入下载的 crate。仓库维护使用 Rust `xtask`，无 Python 维护文件；通过 crates.io 使用库不需要这些工具。仓库文档链接指向 GitHub，精简归档后仍然有效。

依赖按 **2026-10-22** crates.io 最新非 yanked 版本核对（含预发布版本），并保留 Cargo.lock。Rustls 使用 reqwest 兼容范围内最新的 0.23.46；更高的 0.24.0-dev.2 开发预览版类型不兼容，是最高版本要求的文档化例外。其余直接依赖采用最高语义版本。显式按客户端设置 AWS-LC provider，不受宿主 Rustls 全局 provider 影响。详见[依赖记录](dependencies_ZH.md)。

贡献者检查命令，在 GitHub checkout 中运行（本地协议测试不会执行 Steam 管理写入）：

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

本地 mock 测试覆盖所有已实现方法的路径、版本、参数及响应，以及重试、冷却、超时、取消、批处理、整数精度与脱敏。无需 Steam key，但需要允许监听本地端口。离线测试通过不代表 Publisher 认证、写入及生产并发已获 Steam 实测验证。

## 延伸阅读

| 主题 | 文档 |
| --- | --- |
| 设计分析与验证层次 | [design_ZH.md](design_ZH.md) |
| P2 Cloud、库存、经济与 Workshop 契约 | [p2_ZH.md](p2_ZH.md) |
| P3 计费、反作弊与会话契约 | [p3_ZH.md](p3_ZH.md) |
| 路线图与完整方法清单 | [roadmap_ZH.md](roadmap_ZH.md) |
| 传输层 Review 与编解码测量 | [performance_ZH.md](performance_ZH.md)、[review-0.3_ZH.md](review-0.3_ZH.md) |
| 依赖审计 | [dependencies_ZH.md](dependencies_ZH.md) |
| 发布流程 | [releases_ZH.md](releases_ZH.md) |

## 封装覆盖报告

`cargo xtask coverage --json` 将 33 接口、167 路径快照与公开模块、Rust 方法及准确路径字面量逐项核对，区分 157 条显式签名和 10 条 Sandbox 继承路径。CI 将静态报告保留为 `steam-api-web-coverage`，它与协议 fixture、真实权限及生产验证分别处理。覆盖数据和本指南仅留在 GitHub，不进入 crate 白名单。

`cargo xtask audit-api --json` 补充正式参数表、响应形式、匿名发现候选及未完成验收；`--live` 只读取公开来源。详见 [API 审计](api-audit_ZH.md)和 [P5 设计](p5-design_ZH.md)。
