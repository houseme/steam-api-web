<!--
Copyright 2026 steam-api-web contributors.
SPDX-License-Identifier: Apache-2.0
Licensed under the Apache License, Version 2.0.
See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.
-->

# Feature roadmap and reference inventory

English | [简体中文](roadmap_ZH.md)

Reviewed on 2026-10-21 against every interface in the [official Web API index](https://partner.steamgames.com/doc/webapi): **33 interfaces**, **157 explicit HTTP signatures**, and **10 Sandbox signatures inherited from ISteamMicroTxn**. This is a documentation snapshot, not an inventory of private methods or a claim that all bindings are implemented.

The shared machine inventory is [api-inventory.json](api-inventory.json). The 0.2.0 development version binds all 18 P0 and 39 P1 methods (57 total across 12 interfaces), with local contract tests. Undocumented response contracts remain opaque, and Publisher validation is pending.

## Delivery phases

| Phase / proposed version | Scope | Exit criteria |
| --- | --- | --- |
| P0 / 0.1 | Current authentication, users, stats and release foundation | Publisher contracts, deletion cursors, write/read-back, v5 and production validation |
| P1 / 0.2 | Discovery, apps, players, store, news, servers and leaderboards | Every method/version modeled; reads and administrative writes validated separately |
| P2 / 0.3 | Cloud, inventory, economy, trading and Workshop | Pagination, upload stages, trade idempotency, inventory concurrency and permissions |
| P3 / 0.4 | Microtransaction Sandbox, production transactions, anti-cheat and sessions | Sandbox first, reconciliation, no blind write replay, revocation/ban permissions |
| P4 / 0.5 | Finance, site licenses, broadcasts and coverage closure | Data access controls, reporting periods and coverage report for all 33 interfaces |
| P5 / separate proposal | Optional native SDK interoperability companion crate | Lifecycle/thread/callback contracts; base HTTP crate stays SDK-independent |

Versions describe sequencing, not release dates. P0/P1 bindings are implemented; P2–P5 remain planned. Each HTTP method needs an exact path/version, credential kind, replay classification, parameter constraints, error fixtures, documentation, and applicable credentialed validation.

## Complete Web API interface matrix

| Interface | Signatures | Phase | Current status |
| --- | --- | --- | --- |
| [ISteamUserAuth](https://partner.steamgames.com/doc/webapi/ISteamUserAuth) | 2 | P0 | Bound; local contract tests |
| [ISteamUserStats](https://partner.steamgames.com/doc/webapi/ISteamUserStats) | 7 | P0 | Bound; local contract tests |
| [ISteamUser](https://partner.steamgames.com/doc/webapi/ISteamUser) | 9 | P0 | Bound; local contract tests |
| [ISteamWebAPIUtil](https://partner.steamgames.com/doc/webapi/ISteamWebAPIUtil) | 2 | P1 | Bound; local contract tests |
| [ISteamApps](https://partner.steamgames.com/doc/webapi/ISteamApps) | 10 | P1 | Bound; local contract tests |
| [IPlayerService](https://partner.steamgames.com/doc/webapi/IPlayerService) | 6 | P1 | Bound; local contract tests |
| [IStoreService](https://partner.steamgames.com/doc/webapi/IStoreService) | 1 | P1 | Bound; local contract tests |
| [IUserReviewsService](https://partner.steamgames.com/doc/webapi/IUserReviewsService) | 1 | P1 | Bound; local contract tests |
| [ISteamNews](https://partner.steamgames.com/doc/webapi/ISteamNews) | 2 | P1 | Bound; local contract tests |
| [IGameServersService](https://partner.steamgames.com/doc/webapi/IGameServersService) | 9 | P1 | Bound; local contract tests |
| [ISteamGameServerStats](https://partner.steamgames.com/doc/webapi/ISteamGameServerStats) | 1 | P1 | Bound; local contract tests |
| [ISteamLeaderboards](https://partner.steamgames.com/doc/webapi/ISteamLeaderboards) | 7 | P1 | Bound; local contract tests |
| [ICloudService](https://partner.steamgames.com/doc/webapi/ICloudService) | 6 | P2 | Planned |
| [IInventoryService](https://partner.steamgames.com/doc/webapi/IInventoryService) | 10 | P2 | Planned |
| [IGameInventory](https://partner.steamgames.com/doc/webapi/IGameInventory) | 5 | P2 | Planned |
| [ISteamEconomy](https://partner.steamgames.com/doc/webapi/ISteamEconomy) | 8 | P2 | Planned |
| [IEconService](https://partner.steamgames.com/doc/webapi/IEconService) | 7 | P2 | Planned |
| [IEconMarketService](https://partner.steamgames.com/doc/webapi/IEconMarketService) | 4 | P2 | Planned |
| [IPublishedFileService](https://partner.steamgames.com/doc/webapi/IPublishedFileService) | 7 | P2 | Planned |
| [ISteamPublishedItemSearch](https://partner.steamgames.com/doc/webapi/ISteamPublishedItemSearch) | 4 | P2 | Planned |
| [ISteamPublishedItemVoting](https://partner.steamgames.com/doc/webapi/ISteamPublishedItemVoting) | 2 | P2 | Planned |
| [ISteamRemoteStorage](https://partner.steamgames.com/doc/webapi/ISteamRemoteStorage) | 7 | P2 | Planned |
| [IPublishedFileModerationService](https://partner.steamgames.com/doc/webapi/IPublishedFileModerationService) | 2 | P2 | Planned |
| [IWorkshopService](https://partner.steamgames.com/doc/webapi/IWorkshopService) | 4 | P2 | Planned |
| [ISteamMicroTxnSandbox](https://partner.steamgames.com/doc/webapi/ISteamMicroTxnSandbox) | 10 inherited | P3 | Planned |
| [ISteamMicroTxn](https://partner.steamgames.com/doc/webapi/ISteamMicroTxn) | 10 | P3 | Planned |
| [ICheatReportingService](https://partner.steamgames.com/doc/webapi/ICheatReportingService) | 7 | P3 | Planned |
| [ISteamCommunity](https://partner.steamgames.com/doc/webapi/ISteamCommunity) | 1 | P3 | Planned |
| [IGameNotificationsService](https://partner.steamgames.com/doc/webapi/IGameNotificationsService) | 7 | P3 | Planned |
| [ILobbyMatchmakingService](https://partner.steamgames.com/doc/webapi/ILobbyMatchmakingService) | 3 | P3 | Planned |
| [IPartnerFinancialsService](https://partner.steamgames.com/doc/webapi/IPartnerFinancialsService) | 3 | P4 | Planned |
| [ISiteLicenseService](https://partner.steamgames.com/doc/webapi/ISiteLicenseService) | 2 | P4 | Planned |
| [IBroadcastService](https://partner.steamgames.com/doc/webapi/IBroadcastService) | 1 | P4 | Planned |

The Sandbox page describes the production transaction interface with a dummy database. Its signatures are explicitly marked as inherited, not independently discovered on that page. Missing parameter/response contracts or additional private methods require further review; counts do not infer them.

## steamworks-rs module mapping

Reference tree: [`d8114185613be4dbfe4897c8484d0b2670fd3cd7`](https://github.com/Noxime/steamworks-rs/tree/d8114185613be4dbfe4897c8484d0b2670fd3cd7). The matrix covers all 25 Rust source files, including internal infrastructure and stat submodules; it does not imply that every file is a public interface.

| Module | Mapping and boundary | Phase |
| --- | --- | --- |
| [`app.rs`](https://github.com/Noxime/steamworks-rs/blob/d8114185613be4dbfe4897c8484d0b2670fd3cd7/src/app.rs) | ISteamApps covers remote app administration; local installation/DLC checks remain native. | P1 / P5 |
| [`callback.rs`](https://github.com/Noxime/steamworks-rs/blob/d8114185613be4dbfe4897c8484d0b2670fd3cd7/src/callback.rs) | Native callback execution needs an SDK adapter; document callback lifetimes. | P5 |
| [`error.rs`](https://github.com/Noxime/steamworks-rs/blob/d8114185613be4dbfe4897c8484d0b2670fd3cd7/src/error.rs) | Structured Web errors exist; map SDK/EResult differences explicitly. | P0 / P5 |
| [`friends.rs`](https://github.com/Noxime/steamworks-rs/blob/d8114185613be4dbfe4897c8484d0b2670fd3cd7/src/friends.rs) | ISteamUser and IPlayerService cover remote profile data; chat, overlays and native avatar textures stay local. | P0 / P1 / P5 |
| [`input.rs`](https://github.com/Noxime/steamworks-rs/blob/d8114185613be4dbfe4897c8484d0b2670fd3cd7/src/input.rs) | No direct HTTP counterpart for controller/input state; optional SDK adapter. | P5 |
| [`lib.rs`](https://github.com/Noxime/steamworks-rs/blob/d8114185613be4dbfe4897c8484d0b2670fd3cd7/src/lib.rs) | Shared client and SDK lifecycle; native initialization stays local. | P0 / P5 |
| [`matchmaking.rs`](https://github.com/Noxime/steamworks-rs/blob/d8114185613be4dbfe4897c8484d0b2670fd3cd7/src/matchmaking.rs) | ILobbyMatchmakingService provides documented service operations; native lobby callbacks need an adapter. | P3 / P5 |
| [`matchmaking_servers.rs`](https://github.com/Noxime/steamworks-rs/blob/d8114185613be4dbfe4897c8484d0b2670fd3cd7/src/matchmaking_servers.rs) | ISteamApps and IGameServersService provide server queries/administration; native query callbacks remain local. | P1 / P5 |
| [`networking.rs`](https://github.com/Noxime/steamworks-rs/blob/d8114185613be4dbfe4897c8484d0b2670fd3cd7/src/networking.rs) | P2P packet transport has no HTTP replacement; keep it in native SDK integration. | P5 |
| [`networking_messages.rs`](https://github.com/Noxime/steamworks-rs/blob/d8114185613be4dbfe4897c8484d0b2670fd3cd7/src/networking_messages.rs) | Native networking messages require SDK transport and callbacks. | P5 |
| [`networking_sockets.rs`](https://github.com/Noxime/steamworks-rs/blob/d8114185613be4dbfe4897c8484d0b2670fd3cd7/src/networking_sockets.rs) | Native sockets/connections cannot be implemented by backend Web API calls. | P5 |
| [`networking_sockets_callback.rs`](https://github.com/Noxime/steamworks-rs/blob/d8114185613be4dbfe4897c8484d0b2670fd3cd7/src/networking_sockets_callback.rs) | Connection-state callbacks require a local SDK event loop. | P5 |
| [`networking_types.rs`](https://github.com/Noxime/steamworks-rs/blob/d8114185613be4dbfe4897c8484d0b2670fd3cd7/src/networking_types.rs) | Optional adapter types must preserve native identity/address semantics without adding FFI to the base crate. | P5 |
| [`networking_utils.rs`](https://github.com/Noxime/steamworks-rs/blob/d8114185613be4dbfe4897c8484d0b2670fd3cd7/src/networking_utils.rs) | Relay configuration, ping and transport diagnostics remain native SDK capabilities. | P5 |
| [`remote_play.rs`](https://github.com/Noxime/steamworks-rs/blob/d8114185613be4dbfe4897c8484d0b2670fd3cd7/src/remote_play.rs) | Remote Play sessions and input/rendering depend on the native client. | P5 |
| [`remote_storage.rs`](https://github.com/Noxime/steamworks-rs/blob/d8114185613be4dbfe4897c8484d0b2670fd3cd7/src/remote_storage.rs) | ICloudService and ISteamRemoteStorage provide remote file/UGC operations; local synchronization stays native. | P2 / P5 |
| [`screenshots.rs`](https://github.com/Noxime/steamworks-rs/blob/d8114185613be4dbfe4897c8484d0b2670fd3cd7/src/screenshots.rs) | Screenshot capture and Steam overlay integration need a native adapter. | P5 |
| [`server.rs`](https://github.com/Noxime/steamworks-rs/blob/d8114185613be4dbfe4897c8484d0b2670fd3cd7/src/server.rs) | IGameServersService, ISteamGameServerStats and cheat services cover administration; native game-server lifecycle remains local. | P1 / P3 / P5 |
| [`timeline.rs`](https://github.com/Noxime/steamworks-rs/blob/d8114185613be4dbfe4897c8484d0b2670fd3cd7/src/timeline.rs) | Timeline events require native SDK state; no documented Web API equivalent. | P5 |
| [`ugc.rs`](https://github.com/Noxime/steamworks-rs/blob/d8114185613be4dbfe4897c8484d0b2670fd3cd7/src/ugc.rs) | Workshop, published-file, search and moderation interfaces provide server-side UGC workflows. | P2 |
| [`user.rs`](https://github.com/Noxime/steamworks-rs/blob/d8114185613be4dbfe4897c8484d0b2670fd3cd7/src/user.rs) | Current ISteamUser/ISteamUserAuth bindings bridge IDs and Web API tickets; native session callbacks stay local. | P0 / P5 |
| [`user_stats.rs`](https://github.com/Noxime/steamworks-rs/blob/d8114185613be4dbfe4897c8484d0b2670fd3cd7/src/user_stats.rs) | Current user stats plus planned leaderboard bindings; native asynchronous notifications need SDK integration. | P0 / P1 / P5 |
| [`user_stats/stat_callback.rs`](https://github.com/Noxime/steamworks-rs/blob/d8114185613be4dbfe4897c8484d0b2670fd3cd7/src/user_stats/stat_callback.rs) | Stat callbacks remain native; document ordering and lifetime in an optional adapter. | P5 |
| [`user_stats/stats.rs`](https://github.com/Noxime/steamworks-rs/blob/d8114185613be4dbfe4897c8484d0b2670fd3cd7/src/user_stats/stats.rs) | Preserve numeric read types; Web writes accept uint32 rather than assuming SDK float setters are equivalent. | P0 / P5 |
| [`utils.rs`](https://github.com/Noxime/steamworks-rs/blob/d8114185613be4dbfe4897c8484d0b2670fd3cd7/src/utils.rs) | ISteamWebAPIUtil covers server info/discovery; native utility dialogs and runtime state remain local. | P1 / P5 |

Native input, P2P, screenshots, timeline, Remote Play, overlays and callbacks have no equivalent HTTP data plane. Alignment means an explicit interoperability/adapter design, not invented HTTP equivalence. Any native bridge needs a separate proposal and must not force SDK loading into the backend HTTP crate. raw-bindings and platform FFI remain within that boundary.

## Documented method inventory

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

Inherits ISteamMicroTxn methods and versions under the Sandbox interface.

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
