<!--
Copyright 2026 steam-api-web contributors.
SPDX-License-Identifier: Apache-2.0
Licensed under the Apache License, Version 2.0.
See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.
-->

# Web API examples and documentation checks

English | [简体中文](examples_ZH.md)

The `examples/` directory contains **38 standalone programs**. The matrix below maps every one of the official reference's **33 interface families** to a runnable example. All **160 public async methods**, including compatibility and paging helpers, also have compile-checked English `no_run` examples in rustdoc. These are source and compilation checks; credentials, remote permissions and business outcomes still require an authorized environment.

```sh
cargo run --example current_players -- 480
cargo run --example app_reviews -- 480
cargo run --example user_profiles -- 76561198000000000
cargo run --example banned_players -- 480 1700000000 1700086400
cargo check --examples --locked
cargo test --doc --package steam-api-web --locked
```

Each program documents its positional arguments at the top of its source. User-key examples read `STEAM_USER_KEY` and use the public host. Publisher examples read `STEAM_PUBLISHER_KEY` and use Partner. Financial examples require the separate `STEAM_FINANCIAL_KEY`; Cloud uses `STEAM_OAUTH_ACCESS_TOKEN`. Set these variables in the execution environment. Steam enforces app ownership and account-specific permissions independently of local key-kind validation.

Most examples read metadata. `broadcast_frame` submits a frame to the supplied existing broadcast, and `community_report` submits the supplied abuse report. `legacy_login` transports prepared ciphertext; new integrations should use `verify_ticket`. The production transaction example explicitly selects a production read; the Sandbox program retains its separate environment.

Opaque payloads have redacted `Debug`; inspect `response.as_json()` to handle the endpoint's business fields. The examples preserve cursors and numeric IDs without inventing response schemas, termination rules or completed write outcomes. CI compiles examples and rustdoc snippets without executing their Steam requests.

## Interface matrix

| Official interface | Routes | Standalone example |
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

## Additional workflows

- [Banned-player filtering and continuation](../examples/banned_players.rs): pass `starttime` and `endtime` as Unix seconds; supply the previous `continue_token` with the same interval. The compatible `get_players_banned(appid)` call still omits all three optional fields. The published reference does not define complete response wrapping or interval inclusivity.
- [Client configuration](../examples/client_configuration.rs): connection/request/operation deadlines, resource limits, pacing, read retries, shared clones and metrics.
- [Anonymous player count](../examples/current_players.rs): a small public read without an API key.
- [Financial change batch](../examples/financial_changed_dates.rs): receive a change boundary without advancing it before processing every sales page.
- [Prepared legacy login](../examples/legacy_login.rs): read encrypted binary material from files without guessing undocumented RSA/AES parameters.

## Audit result

A fresh download of the official index and all 33 method pages found **167 bound routes** and **735 formal parameter rows**. It exposed three missing optional `GetPlayersBanned` inputs; the new `BannedPlayersOptions` and `get_players_banned_with_options` implement them while preserving the existing API. The six focused protocol tests cover omission, zero/full-width timestamps, token escaping, validation, credentials and replay-safe retries.

Reproduce the captured-page check with `cargo xtask audit-api --reference-dir /path/to/fresh-pages --json`. In this check the reference pages were refreshed; anonymous discovery and the native SDK tree retained the previously reviewed snapshots because the direct discovery refresh failed at the network layer. No protected Steam operation was executed. See the [completeness audit](api-audit.md) for production and native-SDK acceptance boundaries.
