<!--
Copyright 2026 steam-api-web contributors.
SPDX-License-Identifier: Apache-2.0
Licensed under the Apache License, Version 2.0.
See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.
-->

# 功能路线图与参考清单

[English](roadmap.md) | 简体中文

核对日期：2026-10-21。范围覆盖[官方 Web API 索引](https://partner.steamgames.com/doc/webapi)的全部 **33 个接口**，包含 **157 个显式 HTTP 签名**，以及 Sandbox 从 ISteamMicroTxn 继承的 **10 个签名**。这是文档快照清单，不包含私有或未公开方法，也不是已实现覆盖率声明。

共享机器清单：[api-inventory.json](api-inventory.json)。0.4.0 开发版已封装 P0–P3，共 30 个接口、161 条路径，包含 151 条显式文档路径及 10 条继承 Sandbox 路径，并有本地契约测试。未公开响应结构保持 opaque，Publisher 实测尚未完成。

## 推进阶段

| 阶段 / 规划版本 | 交付重点 | 验收门槛 |
| --- | --- | --- |
| P0 / 0.1 | 当前认证、用户、统计与发布基础 | 完成 Publisher 契约、删除游标、写入后读回、v5 及生产验证 |
| P1 / 0.2 | 发现、应用、玩家、商店、新闻、服务器、排行榜 | 全方法/版本请求模型，读取与管理写入分开验证 |
| P2 / 0.3 | Cloud、库存、经济、交易与 Workshop | 分页、上传阶段、交易幂等性、库存并发和权限验证 |
| P3 / 0.4 | 微交易 Sandbox、生产微交易、反作弊和会话 | 先 Sandbox、账务核对、写入不重放、撤销/封禁权限验证 |
| P4 / 0.5 | 财务、站点许可、广播及覆盖收敛 | 数据访问权限、账期/时间范围、所有 33 个接口的覆盖报告 |
| P5 / 单独提案 | 可选原生 SDK 互操作 companion crate | 生命周期/线程/回调契约；基础 HTTP crate 继续不依赖原生 SDK |

版本是规划顺序，不是发布日期；P0–P3 已有封装实现，P4–P5 仍处于规划阶段。每个 HTTP 方法交付须包含准确路径/版本、凭据种类、重放分类、参数约束、错误 fixtures、文档和适用的凭据验证。

## 全部 Web API 接口

| 接口 | 签名数 | 阶段 | 当前状态 |
| --- | --- | --- | --- |
| [ISteamUserAuth](https://partner.steamgames.com/doc/webapi/ISteamUserAuth) | 2 | P0 | 已封装；本地契约测试 |
| [ISteamUserStats](https://partner.steamgames.com/doc/webapi/ISteamUserStats) | 7 | P0 | 已封装；本地契约测试 |
| [ISteamUser](https://partner.steamgames.com/doc/webapi/ISteamUser) | 9 | P0 | 已封装；本地契约测试 |
| [ISteamWebAPIUtil](https://partner.steamgames.com/doc/webapi/ISteamWebAPIUtil) | 2 | P1 | 已封装；本地契约测试 |
| [ISteamApps](https://partner.steamgames.com/doc/webapi/ISteamApps) | 10 | P1 | 已封装；本地契约测试 |
| [IPlayerService](https://partner.steamgames.com/doc/webapi/IPlayerService) | 6 | P1 | 已封装；本地契约测试 |
| [IStoreService](https://partner.steamgames.com/doc/webapi/IStoreService) | 1 | P1 | 已封装；本地契约测试 |
| [IUserReviewsService](https://partner.steamgames.com/doc/webapi/IUserReviewsService) | 1 | P1 | 已封装；本地契约测试 |
| [ISteamNews](https://partner.steamgames.com/doc/webapi/ISteamNews) | 2 | P1 | 已封装；本地契约测试 |
| [IGameServersService](https://partner.steamgames.com/doc/webapi/IGameServersService) | 9 | P1 | 已封装；本地契约测试 |
| [ISteamGameServerStats](https://partner.steamgames.com/doc/webapi/ISteamGameServerStats) | 1 | P1 | 已封装；本地契约测试 |
| [ISteamLeaderboards](https://partner.steamgames.com/doc/webapi/ISteamLeaderboards) | 7 | P1 | 已封装；本地契约测试 |
| [ICloudService](https://partner.steamgames.com/doc/webapi/ICloudService) | 6 | P2 | 已封装；本地契约测试 |
| [IInventoryService](https://partner.steamgames.com/doc/webapi/IInventoryService) | 10 | P2 | 已封装；本地契约测试 |
| [IGameInventory](https://partner.steamgames.com/doc/webapi/IGameInventory) | 5 | P2 | 已封装；本地契约测试 |
| [ISteamEconomy](https://partner.steamgames.com/doc/webapi/ISteamEconomy) | 8 | P2 | 已封装；本地契约测试 |
| [IEconService](https://partner.steamgames.com/doc/webapi/IEconService) | 7 | P2 | 已封装；本地契约测试 |
| [IEconMarketService](https://partner.steamgames.com/doc/webapi/IEconMarketService) | 4 | P2 | 已封装；本地契约测试 |
| [IPublishedFileService](https://partner.steamgames.com/doc/webapi/IPublishedFileService) | 7 | P2 | 已封装；本地契约测试 |
| [ISteamPublishedItemSearch](https://partner.steamgames.com/doc/webapi/ISteamPublishedItemSearch) | 4 | P2 | 已封装；本地契约测试 |
| [ISteamPublishedItemVoting](https://partner.steamgames.com/doc/webapi/ISteamPublishedItemVoting) | 2 | P2 | 已封装；本地契约测试 |
| [ISteamRemoteStorage](https://partner.steamgames.com/doc/webapi/ISteamRemoteStorage) | 7 | P2 | 已封装；本地契约测试 |
| [IPublishedFileModerationService](https://partner.steamgames.com/doc/webapi/IPublishedFileModerationService) | 2 | P2 | 已封装；本地契约测试 |
| [IWorkshopService](https://partner.steamgames.com/doc/webapi/IWorkshopService) | 4 | P2 | 已封装；本地契约测试 |
| [ISteamMicroTxnSandbox](https://partner.steamgames.com/doc/webapi/ISteamMicroTxnSandbox) | 10（继承） | P3 | 已封装；本地契约测试 |
| [ISteamMicroTxn](https://partner.steamgames.com/doc/webapi/ISteamMicroTxn) | 10 | P3 | 已封装；本地契约测试 |
| [ICheatReportingService](https://partner.steamgames.com/doc/webapi/ICheatReportingService) | 7 | P3 | 已封装；本地契约测试 |
| [ISteamCommunity](https://partner.steamgames.com/doc/webapi/ISteamCommunity) | 1 | P3 | 已封装；本地契约测试 |
| [IGameNotificationsService](https://partner.steamgames.com/doc/webapi/IGameNotificationsService) | 7 | P3 | 已封装；本地契约测试 |
| [ILobbyMatchmakingService](https://partner.steamgames.com/doc/webapi/ILobbyMatchmakingService) | 3 | P3 | 已封装；本地契约测试 |
| [IPartnerFinancialsService](https://partner.steamgames.com/doc/webapi/IPartnerFinancialsService) | 3 | P4 | 规划中 |
| [ISiteLicenseService](https://partner.steamgames.com/doc/webapi/ISiteLicenseService) | 2 | P4 | 规划中 |
| [IBroadcastService](https://partner.steamgames.com/doc/webapi/IBroadcastService) | 1 | P4 | 规划中 |

Sandbox 页面仅说明与生产微交易相同的方法，本清单将继承关系显式记录；不是从该页面发现了独立 HTTP 签名。页面未公开的参数、响应或额外私有方法必须进一步核对，不能从计数推断。

## steamworks-rs 逐模块对照

参考源码树：[`d8114185613be4dbfe4897c8484d0b2670fd3cd7`](https://github.com/Noxime/steamworks-rs/tree/d8114185613be4dbfe4897c8484d0b2670fd3cd7)。以下覆盖此次源码树的全部 25 个 Rust 源文件（包括内部基础设施与统计子模块），并不表示它们都是公开接口。

| 模块 | 语义映射与边界 | 阶段 |
| --- | --- | --- |
| [`app.rs`](https://github.com/Noxime/steamworks-rs/blob/d8114185613be4dbfe4897c8484d0b2670fd3cd7/src/app.rs) | ISteamApps 覆盖远程应用管理；本地安装/DLC 检查仍依赖原生 SDK。 | P1 / P5 |
| [`callback.rs`](https://github.com/Noxime/steamworks-rs/blob/d8114185613be4dbfe4897c8484d0b2670fd3cd7/src/callback.rs) | 原生回调执行需要 SDK 适配器；记录回调生命周期。 | P5 |
| [`error.rs`](https://github.com/Noxime/steamworks-rs/blob/d8114185613be4dbfe4897c8484d0b2670fd3cd7/src/error.rs) | 已有结构化 Web 错误；显式映射 SDK/EResult 差异。 | P0 / P5 |
| [`friends.rs`](https://github.com/Noxime/steamworks-rs/blob/d8114185613be4dbfe4897c8484d0b2670fd3cd7/src/friends.rs) | ISteamUser 和 IPlayerService 覆盖远程资料；聊天、覆盖层和原生头像纹理保留本地。 | P0 / P1 / P5 |
| [`input.rs`](https://github.com/Noxime/steamworks-rs/blob/d8114185613be4dbfe4897c8484d0b2670fd3cd7/src/input.rs) | 控制器/输入状态没有直接 HTTP 对应；规划可选 SDK 适配器。 | P5 |
| [`lib.rs`](https://github.com/Noxime/steamworks-rs/blob/d8114185613be4dbfe4897c8484d0b2670fd3cd7/src/lib.rs) | 共享客户端与 SDK 生命周期；原生初始化保留在本地。 | P0 / P5 |
| [`matchmaking.rs`](https://github.com/Noxime/steamworks-rs/blob/d8114185613be4dbfe4897c8484d0b2670fd3cd7/src/matchmaking.rs) | ILobbyMatchmakingService 提供文档服务操作；原生大厅回调需要适配器。 | P3 / P5 |
| [`matchmaking_servers.rs`](https://github.com/Noxime/steamworks-rs/blob/d8114185613be4dbfe4897c8484d0b2670fd3cd7/src/matchmaking_servers.rs) | ISteamApps 和 IGameServersService 提供服务器查询/管理；原生查询回调保留本地。 | P1 / P5 |
| [`networking.rs`](https://github.com/Noxime/steamworks-rs/blob/d8114185613be4dbfe4897c8484d0b2670fd3cd7/src/networking.rs) | P2P 数据包传输没有 HTTP 替代；保留于原生 SDK 集成。 | P5 |
| [`networking_messages.rs`](https://github.com/Noxime/steamworks-rs/blob/d8114185613be4dbfe4897c8484d0b2670fd3cd7/src/networking_messages.rs) | 原生网络消息需要 SDK 传输与回调。 | P5 |
| [`networking_sockets.rs`](https://github.com/Noxime/steamworks-rs/blob/d8114185613be4dbfe4897c8484d0b2670fd3cd7/src/networking_sockets.rs) | 原生 socket/连接无法通过后端 Web API 调用实现。 | P5 |
| [`networking_sockets_callback.rs`](https://github.com/Noxime/steamworks-rs/blob/d8114185613be4dbfe4897c8484d0b2670fd3cd7/src/networking_sockets_callback.rs) | 连接状态回调需要本地 SDK 事件循环。 | P5 |
| [`networking_types.rs`](https://github.com/Noxime/steamworks-rs/blob/d8114185613be4dbfe4897c8484d0b2670fd3cd7/src/networking_types.rs) | 可选适配类型保留原生身份/地址语义，不向基础 crate 引入 FFI。 | P5 |
| [`networking_utils.rs`](https://github.com/Noxime/steamworks-rs/blob/d8114185613be4dbfe4897c8484d0b2670fd3cd7/src/networking_utils.rs) | 中继配置、ping 与传输诊断仍属于原生 SDK 能力。 | P5 |
| [`remote_play.rs`](https://github.com/Noxime/steamworks-rs/blob/d8114185613be4dbfe4897c8484d0b2670fd3cd7/src/remote_play.rs) | Remote Play 会话与输入/渲染依赖原生客户端。 | P5 |
| [`remote_storage.rs`](https://github.com/Noxime/steamworks-rs/blob/d8114185613be4dbfe4897c8484d0b2670fd3cd7/src/remote_storage.rs) | ICloudService 和 ISteamRemoteStorage 提供远程文件/UGC 操作；本地同步保留原生。 | P2 / P5 |
| [`screenshots.rs`](https://github.com/Noxime/steamworks-rs/blob/d8114185613be4dbfe4897c8484d0b2670fd3cd7/src/screenshots.rs) | 截图采集和 Steam 覆盖层集成需要原生适配器。 | P5 |
| [`server.rs`](https://github.com/Noxime/steamworks-rs/blob/d8114185613be4dbfe4897c8484d0b2670fd3cd7/src/server.rs) | IGameServersService、ISteamGameServerStats 和反作弊服务覆盖管理功能；原生游戏服务器生命周期保留本地。 | P1 / P3 / P5 |
| [`timeline.rs`](https://github.com/Noxime/steamworks-rs/blob/d8114185613be4dbfe4897c8484d0b2670fd3cd7/src/timeline.rs) | 时间线事件依赖原生 SDK 状态，没有文档化的 Web API 等价接口。 | P5 |
| [`ugc.rs`](https://github.com/Noxime/steamworks-rs/blob/d8114185613be4dbfe4897c8484d0b2670fd3cd7/src/ugc.rs) | Workshop、发布文件、搜索和审核接口提供服务端 UGC 流程。 | P2 |
| [`user.rs`](https://github.com/Noxime/steamworks-rs/blob/d8114185613be4dbfe4897c8484d0b2670fd3cd7/src/user.rs) | 当前 ISteamUser/ISteamUserAuth 桥接 ID 与 Web API 票据；原生会话回调保留本地。 | P0 / P5 |
| [`user_stats.rs`](https://github.com/Noxime/steamworks-rs/blob/d8114185613be4dbfe4897c8484d0b2670fd3cd7/src/user_stats.rs) | 用户统计与排行榜 Web 封装；原生异步通知需要 SDK 集成。 | P0 / P1 / P5 |
| [`user_stats/stat_callback.rs`](https://github.com/Noxime/steamworks-rs/blob/d8114185613be4dbfe4897c8484d0b2670fd3cd7/src/user_stats/stat_callback.rs) | 统计回调保留原生；在可选适配器中记录顺序与生命周期。 | P5 |
| [`user_stats/stats.rs`](https://github.com/Noxime/steamworks-rs/blob/d8114185613be4dbfe4897c8484d0b2670fd3cd7/src/user_stats/stats.rs) | 保留读取数值类型；Web 写入接受 uint32，不假定等价于 SDK 浮点 setter。 | P0 / P5 |
| [`utils.rs`](https://github.com/Noxime/steamworks-rs/blob/d8114185613be4dbfe4897c8484d0b2670fd3cd7/src/utils.rs) | ISteamWebAPIUtil 覆盖服务器信息/发现；原生工具对话框与运行时状态保留本地。 | P1 / P5 |

原生输入、P2P、截图、时间线、Remote Play、覆盖层与回调没有相应 HTTP 数据面。对齐目标是明确的互操作/适配方案，不虚构 HTTP 等价性；可选原生桥接必须另行设计，不能让服务端 HTTP 基础 crate 被迫加载 SDK。raw-bindings 和平台 FFI 同样属于这一边界。

## 文档方法明细

### IBroadcastService

- `PostGameDataFrame / v1 / POST`

### ICheatReportingService

- `ReportPlayerCheating / v1 / POST`
- `RequestPlayerGameBan / v1 / POST`
- `RemovePlayerGameBan / v1 / POST`
- `GetCheatingReports / v1 / GET`
- `RequestVacStatusForUser / v1 / POST`
- `StartSecureMultiplayerSession / v1 / POST`
- `EndSecureMultiplayerSession / v1 / POST`

### ICloudService

- `EnumerateUserFiles / v1 / GET`
- `BeginAppUploadBatch / v1 / POST`
- `CompleteAppUploadBatch / v1 / POST`
- `BeginHTTPUpload / v1 / POST`
- `CommitHTTPUpload / v1 / POST`
- `Delete / v1 / POST`

### IEconMarketService

- `GetMarketEligibility / v1 / GET`
- `CancelAppListingsForUser / v1 / POST`
- `GetAssetID / v1 / GET`
- `GetPopular / v1 / GET`

### IEconService

- `GetTradeHistory / v1 / GET`
- `FlushInventoryCache / v1 / POST`
- `FlushAssetAppearanceCache / v1 / POST`
- `FlushContextCache / v1 / POST`
- `GetTradeOffers / v1 / GET`
- `GetTradeOffer / v1 / GET`
- `GetTradeOffersSummary / v1 / GET`

### IGameInventory

- `GetHistoryCommandDetails / v1 / GET`
- `GetUserHistory / v1 / GET`
- `HistoryExecuteCommands / v1 / POST`
- `SupportGetAssetHistory / v1 / GET`
- `UpdateItemDefs / v1 / POST`

### IGameNotificationsService

- `CreateSession / v1 / POST`
- `UpdateSession / v1 / POST`
- `EnumerateSessionsForApp / v1 / GET`
- `GetSessionDetailsForApp / v1 / GET`
- `RequestNotifications / v1 / POST`
- `DeleteSession / v1 / POST`
- `DeleteSessionBatch / v1 / POST`

### IGameServersService

- `GetAccountList / v1 / GET`
- `CreateAccount / v1 / POST`
- `SetMemo / v1 / POST`
- `ResetLoginToken / v1 / POST`
- `DeleteAccount / v1 / POST`
- `GetAccountPublicInfo / v1 / GET`
- `QueryLoginToken / v1 / GET`
- `GetServerSteamIDsByIP / v1 / GET`
- `GetServerIPsBySteamID / v1 / GET`

### IInventoryService

- `AddItem / v1 / POST`
- `AddPromoItem / v1 / POST`
- `ConsumeItem / v1 / POST`
- `ExchangeItem / v1 / POST`
- `GetInventory / v1 / GET`
- `GetItemDefs / v1 / GET`
- `GetPriceSheet / v1 / GET`
- `Consolidate / v1 / POST`
- `GetQuantity / v1 / GET`
- `ModifyItems / v1 / POST`

### ILobbyMatchmakingService

- `CreateLobby / v1 / POST`
- `RemoveUserFromLobby / v1 / POST`
- `GetLobbyData / v1 / GET`

### IPartnerFinancialsService

- `GetChangedDatesForPartner / v1 / GET`
- `GetDetailedSales / v1 / GET`
- `GetAppWishlistReporting / v1 / GET`

### IPlayerService

- `GetRecentlyPlayedGames / v1 / GET`
- `GetSingleGamePlaytime / v1 / GET`
- `GetOwnedGames / v1 / GET`
- `GetSteamLevel / v1 / GET`
- `GetBadges / v1 / GET`
- `GetCommunityBadgeProgress / v1 / GET`

### IPublishedFileModerationService

- `GetAllApprovalRequests / v1 / GET`
- `ProcessApprovalRequest / v1 / POST`

### IPublishedFileService

- `Delete / v1 / GET`
- `QueryFiles / v1 / GET`
- `SetDeveloperMetadata / v1 / POST`
- `UpdateAppUGCBan / v1 / POST`
- `UpdateBanStatus / v1 / POST`
- `UpdateIncompatibleStatus / v1 / POST`
- `UpdateTags / v1 / POST`

### ISiteLicenseService

- `GetCurrentClientConnections / v1 / GET`
- `GetTotalPlaytime / v1 / GET`

### ISteamApps

- `GetAppBetas / v1 / GET`
- `GetAppBuilds / v1 / GET`
- `GetAppDepotVersions / v1 / GET`
- `GetAppList / v2 / GET`
- `GetPartnerAppListForWebAPIKey / v2 / GET`
- `GetPlayersBanned / v1 / GET`
- `GetServerList / v1 / GET`
- `GetServersAtAddress / v1 / GET`
- `SetAppBuildLive / v2 / POST`
- `UpToDateCheck / v1 / GET`

### ISteamCommunity

- `ReportAbuse / v1 / POST`

### ISteamEconomy

- `CanTrade / v1 / GET`
- `FinalizeAssetTransaction / v1 / POST`
- `GetAssetClassInfo / v1 / GET`
- `GetAssetPrices / v1 / GET`
- `GetExportedAssetsForUser / v1 / GET`
- `GetMarketPrices / v1 / GET`
- `StartAssetTransaction / v1 / POST`
- `StartTrade / v1 / GET`

### ISteamGameServerStats

- `GetGameServerPlayerStatsForGame / v1 / GET`

### ISteamLeaderboards

- `DeleteLeaderboard / v1 / POST`
- `DeleteLeaderboardScore / v1 / POST`
- `FindOrCreateLeaderboard / v2 / POST`
- `GetLeaderboardEntries / v1 / GET`
- `GetLeaderboardsForGame / v2 / GET`
- `ResetLeaderboard / v1 / POST`
- `SetLeaderboardScore / v1 / POST`

### ISteamMicroTxn

- `AdjustAgreement / v1 / POST`
- `CancelAgreement / v1 / POST`
- `FinalizeTxn / v2 / POST`
- `GetReport / v5 / GET`
- `GetUserAgreementInfo / v2 / GET`
- `GetUserInfo / v2 / GET`
- `InitTxn / v3 / POST`
- `ProcessAgreement / v1 / POST`
- `QueryTxn / v3 / GET`
- `RefundTxn / v2 / POST`

### ISteamMicroTxnSandbox

继承 ISteamMicroTxn 的方法与版本，使用 Sandbox interface。

- `AdjustAgreement / v1 / POST`
- `CancelAgreement / v1 / POST`
- `FinalizeTxn / v2 / POST`
- `GetReport / v5 / GET`
- `GetUserAgreementInfo / v2 / GET`
- `GetUserInfo / v2 / GET`
- `InitTxn / v3 / POST`
- `ProcessAgreement / v1 / POST`
- `QueryTxn / v3 / GET`
- `RefundTxn / v2 / POST`

### ISteamNews

- `GetNewsForApp / v2 / GET`
- `GetNewsForAppAuthed / v2 / GET`

### ISteamPublishedItemSearch

- `RankedByPublicationOrder / v1 / POST`
- `RankedByTrend / v1 / POST`
- `RankedByVote / v1 / POST`
- `ResultSetSummary / v1 / POST`

### ISteamPublishedItemVoting

- `ItemVoteSummary / v1 / POST`
- `UserVoteSummary / v1 / POST`

### ISteamRemoteStorage

- `EnumerateUserSubscribedFiles / v1 / POST`
- `GetCollectionDetails / v1 / POST`
- `GetPublishedFileDetails / v1 / POST`
- `GetUGCFileDetails / v1 / GET`
- `SetUGCUsedByGC / v1 / POST`
- `SubscribePublishedFile / v1 / POST`
- `UnsubscribePublishedFile / v1 / POST`

### ISteamUser

- `CheckAppOwnership / v4 / GET`
- `GetAppPriceInfo / v1 / GET`
- `GetDeletedSteamIDs / v1 / GET`
- `GetFriendList / v1 / GET`
- `GetPlayerBans / v1 / GET`
- `GetPlayerSummaries / v2 / GET`
- `GetPublisherAppOwnership / v4 / GET`
- `GetUserGroupList / v1 / GET`
- `ResolveVanityURL / v1 / GET`

### ISteamUserAuth

- `AuthenticateUser / v1 / POST`
- `AuthenticateUserTicket / v1 / GET`

### ISteamUserStats

- `GetGlobalAchievementPercentagesForApp / v2 / GET`
- `GetGlobalStatsForGame / v1 / GET`
- `GetNumberOfCurrentPlayers / v1 / GET`
- `GetPlayerAchievements / v1 / GET`
- `GetSchemaForGame / v2 / GET`
- `GetUserStatsForGame / v2 / GET`
- `SetUserStatsForGame / v1 / POST`

### ISteamWebAPIUtil

- `GetServerInfo / v1 / GET`
- `GetSupportedAPIList / v1 / GET`

### IStoreService

- `GetAppList / v1 / GET`

### IUserReviewsService

- `GetAppReviews / v1 / GET`

### IWorkshopService

- `SetItemPaymentRules / v1 / POST`
- `GetFinalizedContributors / v1 / GET`
- `GetItemDailyRevenue / v1 / GET`
- `PopulateItemDescriptions / v1 / POST`
