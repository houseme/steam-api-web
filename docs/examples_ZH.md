<!--
Copyright 2026 steam-api-web contributors.
SPDX-License-Identifier: Apache-2.0
Licensed under the Apache License, Version 2.0.
See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.
-->

# Web API 示例与文档检查

[English](examples.md) | 简体中文

`examples/` 目录包含 **38 个独立程序**，下表将官方参考目录中的全部 **33 个接口族**逐一映射到可运行示例。全部 **160 个公开异步方法**（含兼容与分页辅助方法）也补有英文 rustdoc `no_run` 示例并通过编译检查。这些证据属于源码和编译验证；凭据、远程权限及业务结果仍需在授权环境中验收。

```sh
cargo run --example current_players -- 480
cargo run --example app_reviews -- 480
cargo run --example user_profiles -- 76561198000000000
cargo run --example banned_players -- 480 1700000000 1700086400
cargo check --examples --locked
cargo test --doc --package steam-api-web --locked
```

每个程序在源码顶部说明位置参数。User-key 示例读取 `STEAM_USER_KEY` 并使用 Public；Publisher 示例读取 `STEAM_PUBLISHER_KEY` 并使用 Partner；财务示例需要独立的 `STEAM_FINANCIAL_KEY`，Cloud 使用 `STEAM_OAUTH_ACCESS_TOKEN`。请在运行环境中设置这些变量。Steam 还会独立核验应用所有权与账户权限，本地 key 类型校验不代表远程授权成功。

大多数示例读取元数据。`broadcast_frame` 会向调用方指定的已有广播提交一帧数据，`community_report` 会提交调用方指定的举报。`legacy_login` 传输预先准备的密文，新集成应使用 `verify_ticket`。生产交易示例显式选择生产环境只读查询，Sandbox 示例保持独立环境。

不透明响应的 `Debug` 已脱敏，处理业务字段时需显式访问 `response.as_json()`。示例保留游标及数字 ID，不推测响应 schema、分页终止标记或写入完成结果。CI 编译这些程序及 rustdoc 示例，不执行其中的 Steam 请求。

## 接口映射

| 官方接口 | 路由数 | 独立示例 |
| --- | ---: | --- |
| [IBroadcastService](https://partner.steamgames.com/doc/webapi/IBroadcastService) | 1 | [broadcast_frame.rs](../examples/broadcast_frame.rs) |
| [ICheatReportingService](https://partner.steamgames.com/doc/webapi/ICheatReportingService) | 7 | [cheating_reports.rs](../examples/cheating_reports.rs) |
| [ICloudService](https://partner.steamgames.com/doc/webapi/ICloudService) | 6 | [cloud_files.rs](../examples/cloud_files.rs) |
| [IEconMarketService](https://partner.steamgames.com/doc/webapi/IEconMarketService) | 4 | [market_popular.rs](../examples/market_popular.rs) |
| [IEconService](https://partner.steamgames.com/doc/webapi/IEconService) | 7 | [trade_history.rs](../examples/trade_history.rs) |
| [IGameInventory](https://partner.steamgames.com/doc/webapi/IGameInventory) | 5 | [inventory_history.rs](../examples/inventory_history.rs) |
| [IGameNotificationsService](https://partner.steamgames.com/doc/webapi/IGameNotificationsService) | 7 | [notification_sessions.rs](../examples/notification_sessions.rs) |
| [IGameServersService](https://partner.steamgames.com/doc/webapi/IGameServersService) | 9 | [game_server_accounts.rs](../examples/game_server_accounts.rs) |
| [IInventoryService](https://partner.steamgames.com/doc/webapi/IInventoryService) | 10 | [inventory_items.rs](../examples/inventory_items.rs) |
| [ILobbyMatchmakingService](https://partner.steamgames.com/doc/webapi/ILobbyMatchmakingService) | 3 | [lobby_data.rs](../examples/lobby_data.rs) |
| [IPartnerFinancialsService](https://partner.steamgames.com/doc/webapi/IPartnerFinancialsService) | 3 | [financial_reports.rs](../examples/financial_reports.rs) |
| [IPlayerService](https://partner.steamgames.com/doc/webapi/IPlayerService) | 6 | [player_library.rs](../examples/player_library.rs) |
| [IPublishedFileModerationService](https://partner.steamgames.com/doc/webapi/IPublishedFileModerationService) | 2 | [workshop_approvals.rs](../examples/workshop_approvals.rs) |
| [IPublishedFileService](https://partner.steamgames.com/doc/webapi/IPublishedFileService) | 7 | [workshop_query.rs](../examples/workshop_query.rs) |
| [ISiteLicenseService](https://partner.steamgames.com/doc/webapi/ISiteLicenseService) | 2 | [site_connections.rs](../examples/site_connections.rs) |
| [ISteamApps](https://partner.steamgames.com/doc/webapi/ISteamApps) | 10 | [app_builds.rs](../examples/app_builds.rs) |
| [ISteamCommunity](https://partner.steamgames.com/doc/webapi/ISteamCommunity) | 1 | [community_report.rs](../examples/community_report.rs) |
| [ISteamEconomy](https://partner.steamgames.com/doc/webapi/ISteamEconomy) | 8 | [asset_prices.rs](../examples/asset_prices.rs) |
| [ISteamGameServerStats](https://partner.steamgames.com/doc/webapi/ISteamGameServerStats) | 1 | [game_server_stats.rs](../examples/game_server_stats.rs) |
| [ISteamLeaderboards](https://partner.steamgames.com/doc/webapi/ISteamLeaderboards) | 7 | [leaderboard_list.rs](../examples/leaderboard_list.rs) |
| [ISteamMicroTxn](https://partner.steamgames.com/doc/webapi/ISteamMicroTxn) | 10 | [query_production_transaction.rs](../examples/query_production_transaction.rs) |
| [ISteamMicroTxnSandbox](https://partner.steamgames.com/doc/webapi/ISteamMicroTxnSandbox) | 10 | [query_sandbox_transaction.rs](../examples/query_sandbox_transaction.rs) |
| [ISteamNews](https://partner.steamgames.com/doc/webapi/ISteamNews) | 2 | [app_news.rs](../examples/app_news.rs) |
| [ISteamPublishedItemSearch](https://partner.steamgames.com/doc/webapi/ISteamPublishedItemSearch) | 4 | [published_item_search.rs](../examples/published_item_search.rs) |
| [ISteamPublishedItemVoting](https://partner.steamgames.com/doc/webapi/ISteamPublishedItemVoting) | 2 | [published_item_votes.rs](../examples/published_item_votes.rs) |
| [ISteamRemoteStorage](https://partner.steamgames.com/doc/webapi/ISteamRemoteStorage) | 7 | [remote_file_details.rs](../examples/remote_file_details.rs) |
| [ISteamUser](https://partner.steamgames.com/doc/webapi/ISteamUser) | 9 | [user_profiles.rs](../examples/user_profiles.rs) |
| [ISteamUserAuth](https://partner.steamgames.com/doc/webapi/ISteamUserAuth) | 2 | [verify_ticket.rs](../examples/verify_ticket.rs) |
| [ISteamUserStats](https://partner.steamgames.com/doc/webapi/ISteamUserStats) | 7 | [user_game_stats.rs](../examples/user_game_stats.rs) |
| [ISteamWebAPIUtil](https://partner.steamgames.com/doc/webapi/ISteamWebAPIUtil) | 2 | [api_discovery.rs](../examples/api_discovery.rs) |
| [IStoreService](https://partner.steamgames.com/doc/webapi/IStoreService) | 1 | [store_catalog.rs](../examples/store_catalog.rs) |
| [IUserReviewsService](https://partner.steamgames.com/doc/webapi/IUserReviewsService) | 1 | [app_reviews.rs](../examples/app_reviews.rs) |
| [IWorkshopService](https://partner.steamgames.com/doc/webapi/IWorkshopService) | 4 | [workshop_contributors.rs](../examples/workshop_contributors.rs) |

## 补充流程

- [封禁用户时间过滤与分页](../examples/banned_players.rs)：以 Unix 秒传入 `starttime`、`endtime`，下一页的 `continue_token` 配合原时间区间使用。兼容方法 `get_players_banned(appid)` 仍省略这三个可选字段。官方参考未给出完整响应外层结构或时间边界的包含规则。
- [客户端配置](../examples/client_configuration.rs)：连接/请求/整体超时、资源上限、请求节奏、只读重试、clone 共享及指标。
- [匿名在线人数](../examples/current_players.rs)：无需 API key 的简单公开读取。
- [财务变更批次](../examples/financial_changed_dates.rs)：接收变更边界，在处理完全部销售分页之前不推进游标。
- [旧版预加密登录](../examples/legacy_login.rs)：从文件读取加密二进制材料，不猜测未公开的 RSA/AES 参数。

## 本次核查结果

重新下载官方索引与全部 33 个接口页后，核对结果为 **167 条已绑定路由、735 行正式参数**。本次发现 `GetPlayersBanned` 的三个可选输入未实现，已通过 `BannedPlayersOptions` 和 `get_players_banned_with_options` 补齐，保留已有 API。六项专门协议测试覆盖省略、零值/完整范围时间戳、token 转义、参数校验、鉴权及可重放读取的重试。

使用 `cargo xtask audit-api --reference-dir /path/to/fresh-pages --json` 可复核下载页面。本次参考文档已重新抓取；匿名发现接口刷新在网络层失败，因此匿名发现及原生 SDK 树沿用此前审阅的快照。未执行任何受保护的 Steam 操作。生产验收与原生 SDK 的边界见[完整性审计](api-audit_ZH.md)。
