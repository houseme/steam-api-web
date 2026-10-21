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
[![Rust](https://img.shields.io/badge/Rust-stable-orange.svg)](#rust-toolchain-compatibility)

[English](README.md) | 简体中文

独立异步 Rust Steam Web API crate。**0.2.0 开发版**覆盖 P0 + P1 共 **12 个接口、57 个方法**，包括用户认证、统计、应用、玩家、商店、评论、新闻、服务器与排行榜。使用 Tokio、reqwest、Rustls 和 **aws-lc-rs**，无需 Steam 客户端或原生 SDK。

## 使用

当前已发布的 0.1.0 提供 P0 的 18 个方法。0.2.0 尚未发布，使用 P1 功能请在 GitHub checkout 中使用路径依赖。下面的配置安装已发布版本；本地开发时将 crate 依赖改为 `{ path = "../steam-api-web" }`。crates.io 和 docs.rs 徽章以实际发布版本为准。

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

复用或 clone `SteamClient`，所有 clone 共享连接池、并发限制、请求节奏、服务端限流冷却和指标。

## 接口覆盖

### P0：18 个方法

| 接口            | 方法                                  | 版本 / HTTP           | 最低凭据（Public 域名） |
|-----------------|---------------------------------------|-----------------------|-------------------------|
| ISteamUserAuth  | AuthenticateUser                      | v1 POST               | 无；需已加密登录材料    |
| ISteamUserAuth  | AuthenticateUserTicket                | v1 GET                | User / Publisher        |
| ISteamUserStats | GetGlobalAchievementPercentagesForApp | v2 GET                | 无                      |
| ISteamUserStats | GetGlobalStatsForGame                 | v1 GET                | Publisher               |
| ISteamUserStats | GetNumberOfCurrentPlayers             | v1 GET                | 无                      |
| ISteamUserStats | GetPlayerAchievements                 | v1 GET                | User / Publisher        |
| ISteamUserStats | GetSchemaForGame                      | v2 GET                | User / Publisher        |
| ISteamUserStats | GetUserStatsForGame                   | v2 GET                | User / Publisher        |
| ISteamUserStats | SetUserStatsForGame                   | v1 POST               | Publisher               |
| ISteamUser      | CheckAppOwnership                     | v4 GET                | Publisher               |
| ISteamUser      | GetAppPriceInfo                       | v1 GET                | Publisher               |
| ISteamUser      | GetDeletedSteamIDs                    | v1 GET                | Publisher               |
| ISteamUser      | GetFriendList                         | v1 GET                | User / Publisher        |
| ISteamUser      | GetPlayerBans                         | v1 GET                | User / Publisher        |
| ISteamUser      | GetPlayerSummaries                    | v2 GET                | User / Publisher        |
| ISteamUser      | GetPublisherAppOwnership              | v4 GET，另提供显式 v5 | Publisher               |
| ISteamUser      | GetUserGroupList                      | v1 GET                | Publisher               |
| ISteamUser      | ResolveVanityURL                      | v1 GET                | User / Publisher        |

### P1：新增 39 个方法

| 接口 / 入口 | 方法 | HTTP / 版本 |
| --- | --- | --- |
| `IGameServersService` / `game_servers` | `GetAccountList` | GET / v1 |
| `IGameServersService` / `game_servers` | `CreateAccount` | POST / v1 |
| `IGameServersService` / `game_servers` | `SetMemo` | POST / v1 |
| `IGameServersService` / `game_servers` | `ResetLoginToken` | POST / v1 |
| `IGameServersService` / `game_servers` | `DeleteAccount` | POST / v1 |
| `IGameServersService` / `game_servers` | `GetAccountPublicInfo` | GET / v1 |
| `IGameServersService` / `game_servers` | `QueryLoginToken` | GET / v1 |
| `IGameServersService` / `game_servers` | `GetServerSteamIDsByIP` | GET / v1 |
| `IGameServersService` / `game_servers` | `GetServerIPsBySteamID` | GET / v1 |
| `IPlayerService` / `player` | `GetRecentlyPlayedGames` | GET / v1 |
| `IPlayerService` / `player` | `GetSingleGamePlaytime` | GET / v1 |
| `IPlayerService` / `player` | `GetOwnedGames` | GET / v1 |
| `IPlayerService` / `player` | `GetSteamLevel` | GET / v1 |
| `IPlayerService` / `player` | `GetBadges` | GET / v1 |
| `IPlayerService` / `player` | `GetCommunityBadgeProgress` | GET / v1 |
| `ISteamApps` / `apps` | `GetAppBetas` | GET / v1 |
| `ISteamApps` / `apps` | `GetAppBuilds` | GET / v1 |
| `ISteamApps` / `apps` | `GetAppDepotVersions` | GET / v1 |
| `ISteamApps` / `apps` | `GetAppList` | GET / v2 |
| `ISteamApps` / `apps` | `GetPartnerAppListForWebAPIKey` | GET / v2 |
| `ISteamApps` / `apps` | `GetPlayersBanned` | GET / v1 |
| `ISteamApps` / `apps` | `GetServerList` | GET / v1 |
| `ISteamApps` / `apps` | `GetServersAtAddress` | GET / v1 |
| `ISteamApps` / `apps` | `SetAppBuildLive` | POST / v2 |
| `ISteamApps` / `apps` | `UpToDateCheck` | GET / v1 |
| `ISteamGameServerStats` / `game_server_stats` | `GetGameServerPlayerStatsForGame` | GET / v1 |
| `ISteamLeaderboards` / `leaderboards` | `DeleteLeaderboard` | POST / v1 |
| `ISteamLeaderboards` / `leaderboards` | `DeleteLeaderboardScore` | POST / v1 |
| `ISteamLeaderboards` / `leaderboards` | `FindOrCreateLeaderboard` | POST / v2 |
| `ISteamLeaderboards` / `leaderboards` | `GetLeaderboardEntries` | GET / v1 |
| `ISteamLeaderboards` / `leaderboards` | `GetLeaderboardsForGame` | GET / v2 |
| `ISteamLeaderboards` / `leaderboards` | `ResetLeaderboard` | POST / v1 |
| `ISteamLeaderboards` / `leaderboards` | `SetLeaderboardScore` | POST / v1 |
| `ISteamNews` / `news` | `GetNewsForApp` | GET / v2 |
| `ISteamNews` / `news` | `GetNewsForAppAuthed` | GET / v2 |
| `ISteamWebAPIUtil` / `web_api_util` | `GetServerInfo` | GET / v1 |
| `ISteamWebAPIUtil` / `web_api_util` | `GetSupportedAPIList` | GET / v1 |
| `IStoreService` / `store` | `GetAppList` | GET / v1 |
| `IUserReviewsService` / `reviews` | `GetAppReviews` | GET / v1 |

Service 接口使用 `input_json`；读取与管理写入分开控制凭据和重放策略。旧 `ISteamApps/GetAppList/v2` 已弃用，新接入使用可分页的 `IStoreService/GetAppList`。`SetAppBuildLive` 保留 HTTP 201 等待手机确认的状态，绝不把请求接受等同于已经发布。

Publisher key 默认选择官方推荐的 `partner.steam-api.com`，该域名的 **每个请求**都需要 Publisher key，包括通常可以匿名的方法。无
key / User key 默认使用 `api.steampowered.com`。`host(Host::Public/Partner)` 可显式选择；Publisher 专用方法应使用
Partner，不会自动跨域切换。AppID 所属、key 权限及 IP 白名单仍由 Steam 校验。

方法使用 snake_case；入口为 `client.user_auth()`、`client.user_stats()`、`client.user()`。`cargo doc --no-deps --open` 可查看完整
API。

## 与 steamworks-rs 配合

参考 [Noxime/steamworks-rs](https://github.com/Noxime/steamworks-rs)：它封装本地 SDK 与回调，这里封装 HTTPS
API，两者可分别运行于客户端与后端。

- `steamworks::SteamId::raw()` 的 `u64` 可传入这里的用户方法；这里提供 `SteamId::from_raw/raw` 双向转换。
- AppID 使用 `u32`（SDK 的 `AppId(x).0`）；64 位 GameID 独立处理，不截断为 AppID。
- SDK 注册 `TicketForWebApiResponse` 回调，调用 `authentication_session_ticket_for_webapi(identity)` 并驱动
  `run_callbacks()`；检查回调结果成功后，向后端发送有效票据。
- 参考源码中 `TicketForWebApiResponse::ticket` 可能包含整个固定缓冲区。使用
  `from_callback_bytes(&response.ticket, response.ticket_len)`，检查长度并截取有效前缀。

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

SDK 的 `AuthTicket` 是取消票据用的句柄，不能作为票据字节。P2P/服务器 `GetAuthSessionTicket`、Web API ticket、Encrypted App
Ticket 不可混用。SDK 初始化、回调保活、结果检查、票据取消由客户端管理。详见[设计分析](https://github.com/houseme/steam-api-web/blob/main/docs/design_ZH.md)。

## 可用性配置

| 配置                          | 默认值        | 作用                                      |
|-------------------------------|---------------|-------------------------------------------|
| connect_timeout               | 3 秒          | 建连超时                                  |
| request_timeout               | 10 秒         | 单次 HTTP 及响应体读取超时                |
| operation_timeout             | 30 秒         | 等待、限流、网络、退避总预算              |
| max_in_flight                 | 32            | clone 共享并发上限                        |
| max_response_bytes            | 8 MiB         | 检查 Content-Length 及实际分块累计大小    |
| RetryPolicy.max_retries       | 2             | 安全 GET 的额外尝试次数，最大 10          |
| initial_backoff / max_backoff | 200 ms / 2 秒 | 指数退避，AWS-LC 随机抖动 50%..100%       |
| min_request_interval          | 0             | clone 共享最小请求间隔，按 Steam 配额配置 |

只对读取请求的传输错误、HTTP 408/429/500/502/503/504 重试。401/403、参数、JSON 和业务错误不重试。`Retry-After` 支持秒数及
HTTP 日期，创建 clone 共享冷却期，不因本地 backoff 上限而提前重试。超出总预算返回 `Error::Deadline`。

认证（包括 GET 票据验证）、统计写入均不重放；reqwest 内置重试也关闭。写入超时可能已经生效，应先查询再决定是否补偿。取消 Future
会释放并发许可。profile 批处理按 100 人顺序调用；每批各有超时，全批次预算由调用方包一层 Tokio timeout。

`metrics()` 提供 attempts/retries/succeeded/failed 快照。校验失败及主动取消不算完成调用。集群级配额协调、故障期间业务降级、跨实例监控和远端
SLA 由部署侧负责；身份及授权结果不会被隐式缓存。

## 数据与错误

- SteamID 接受数字或十进制字符串，序列化为字符串；完整保留 64 位整数。统计值保留有符号、无符号整数及浮点数。
- 可见性受限字段使用 `Option`；缺失封禁标记不会默认成“未封禁”。私密好友列表的 401 返回错误，vanity 无匹配返回 `Ok(None)`。
- 官方未公开响应 schema 的 `AuthenticateUser`、`GetDeletedSteamIDs`、`SetUserStatsForGame` 及 P1 中未公开结构的方法返回 `OpaqueResponse`，仍检测标准业务错误
  envelope。显式 `as_json/into_json` 访问原始数据，不虚构写入成功、分页结构或终止规则。
- `GetPublisherAppOwnership` 官方路径为 v4，版本历史列出 v5。默认使用明确路径，另提供版本枚举；v5 到期字段需 Publisher 环境实测。
- 旧式 `AuthenticateUser` 运输准备好的 RSA/AES 密文，不猜测文档未给出的 RSA key、padding、AES 模式或 IV。rawbinary 按字节转义，不转
  UTF-8/hex。新接入优先使用 Web API ticket。

API key 使用官方 `x-webapi-key` header 并标记 sensitive；关闭重定向。错误及客户端 Debug 不含 URL、票据、key、原始响应和服务器自由文本。自有
key/ticket 字符串释放时清零，但 HTTP 栈及操作系统复制内容不保证清零。`OpaqueResponse` 可能含敏感数据，不应记录 JSON。自定义
base URL 必须可信，只允许 HTTPS 及测试用 loopback HTTP。

<a id="rust-toolchain-compatibility"></a>

## Rust 工具链兼容性

使用最新 stable Rust 和 edition 2024。本次本地验证使用 stable **1.99.1**。当前锁定 ICU 依赖声明最低 Rust
1.88，但项目不承诺经过独立验证的最小编译器版本。CI 跟踪 Linux/macOS/Windows 的 stable 工具链。

## 发布包与仓库工具

Cargo.toml 使用显式 `include` 白名单，发布包仅保留 Rust 库源码、示例、LICENSE、双语 README/changelog，以及 Cargo 必需的 manifest、lockfile 和版本元数据。`.github/`、`xtask/`、`scripts/`、`tests/`、`docs/` 和参考 JSON 数据留在 GitHub，不进入下载给用户的 crate。Cargo 会自动生成/包含必要 manifest 元数据，最终以 `cargo package --list` 和归档检查为准。

仓库维护已迁移为 Rust `xtask`，项目不再保留 Python 维护文件或 workflow 中的 Python 代码。`cargo xtask ...` 仅在 GitHub 源码 checkout 中使用；通过 crates.io 使用库不需要这些工具。README 中的仓库文档链接指向 GitHub，因此精简归档后仍然有效。

## 依赖与验证

以下开发和仓库检查命令在 GitHub checkout 中运行；本地协议测试不会执行 Steam 管理写入。

依赖按 **2026-10-21** crates.io 最新非 yanked 版本核对，包括预发布版本，并保留 Cargo.lock。Rustls 使用 reqwest 兼容范围内最新的
0.23.46；更高的 0.24.0-dev.2 开发预览版类型不兼容，这是最高版本要求的兼容性例外。其余直接依赖采用最高语义版本，当前没有更新的
rc/beta。显式按客户端设置 AWS-LC provider，不受宿主 Rustls 全局 provider 影响。详见[依赖记录](https://github.com/houseme/steam-api-web/blob/main/docs/dependencies_ZH.md)。

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
cargo bench --bench codec --locked --offline
cargo doc --no-deps --locked
cargo xtask check-repository
cargo xtask release verify-package
cargo run --example current_players -- 480
```

本地 mock 测试覆盖所有方法的路径、版本、参数及响应，以及重试、限流、超时、取消、批处理、整数精度、脱敏。测试无需 Steam
key，需要允许监听本地端口。离线测试通过不代表 Publisher 认证、写入及生产并发已获 Steam 实测验证。

## 路线图

[传输层 Review 与编解码测量](https://github.com/houseme/steam-api-web/blob/main/docs/performance_ZH.md)记录 0.2 重构依据与可复跑的离线基准。

P1 的 39 个请求封装及本地契约测试已加入 0.2.0 开发版；公开 schema 有对应类型，未公开响应显式保留原始 JSON。凭据环境和生产验证仍需单独完成。

[完整路线图](https://github.com/houseme/steam-api-web/blob/main/docs/roadmap_ZH.md)列出全部 **33 个文档 Web API 接口**、已公开方法签名和 steamworks-rs
源码模块。以下版本仅为推进顺序，不代表发布日期或功能已完成。

| 阶段          | 规划范围                                                   |
|---------------|------------------------------------------------------------|
| P0 / 0.1      | 当前 18 个封装；Publisher 验证、文档与发布基础             |
| P1 / 0.2      | API 发现、应用、玩家、商店、评论、新闻、服务器管理和排行榜 |
| P2 / 0.3      | Cloud、库存、经济、交易和 Workshop 接口                    |
| P3 / 0.4      | 先 Sandbox 后生产微交易、反作弊、社区、会话和大厅服务      |
| P4 / 0.5      | 发行商财务、站点许可、广播和公开参考的完整覆盖             |
| P5 / 单独提案 | 为无 HTTP 对应能力设计可选原生 SDK 互操作                  |

每阶段需要明确方法/版本、读取/写入重放分类、已公开结构的类型契约、错误 fixtures 和真实凭据验证，才可声明生产可用。原生渲染、输入、P2P
传输及回调执行需要本地 SDK，详见逐模块映射。

## 发布与变更记录

[CHANGELOG_ZH.md](CHANGELOG_ZH.md)记录变更，[发布流程](https://github.com/houseme/steam-api-web/blob/main/docs/releases_ZH.md)说明标签/版本检查、默认手动 dry-run、共享
CI、归档校验和、crates.io 发布及 GitHub 发布说明。真正发布需要配置 CARGO_REGISTRY_TOKEN。

## 参考

[概览](https://partner.steamgames.com/doc/webapi_overview) · [API key](https://partner.steamgames.com/doc/webapi_overview/auth) · [响应与错误](https://partner.steamgames.com/doc/webapi_overview/responses) · [ISteamUserAuth](https://partner.steamgames.com/doc/webapi/ISteamUserAuth) · [ISteamUserStats](https://partner.steamgames.com/doc/webapi/ISteamUserStats) · [ISteamUser](https://partner.steamgames.com/doc/webapi/ISteamUser)

许可：[Apache-2.0](LICENSE)。
