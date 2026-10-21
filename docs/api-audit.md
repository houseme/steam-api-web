<!--
Copyright 2026 steam-api-web contributors.
SPDX-License-Identifier: Apache-2.0
Licensed under the Apache License, Version 2.0.
See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.
-->

# Web API completeness audit

English | [简体中文](api-audit_ZH.md)

Audited on 2026-10-22 against the [official partner reference](https://partner.steamgames.com/doc/webapi), all 33 interface pages, a keyless [GetSupportedAPIList observation](https://api.steampowered.com/ISteamWebAPIUtil/GetSupportedAPIList/v1/), and the fixed steamworks-rs source tree. The executable report is `cargo xtask audit-api --json`; its data lives in [api-inventory.json](api-inventory.json) and [api-discovery.json](api-discovery.json).

## Conclusion and evidence levels

The partner-directory signature bindings are complete: **33 interfaces, 157 explicit signatures plus ten inherited Sandbox routes**. No missing method/path/version or missing static protocol-fixture reference was found. This does not make the entire Steam platform, every discovered method or production acceptance complete.

| Layer | Result | Meaning |
| --- | --- | --- |
| Partner reference | 167/167 routes mapped | Exact methods, verbs, versions and paths; numeric versions do not erase v001/v0001 spelling or trailing-slash differences. |
| Formal request tables | 732 rows: 585 required, 146 optional, one conditional | Source facts, not automatic proof of request semantics. Prose/example contradictions remain explicitly reviewed. Two methods publish no parameter table. |
| Protocol fixtures | 167 static method/interface references | The workspace executes aggregate wire/error/replay tests; this number is not per-method live-test coverage. |
| Response handling | 29 typed, two empty acknowledgements, 136 validated opaque | Opaque schemas remain inspectable JSON, with shared errors checked; they do not invent a successful business result. |
| Anonymous discovery | 27 interfaces / 63 signatures, ten overlap | 53 observed signatures are outside the bound reference set. Discovery depends on visibility and is not an authenticated/complete catalogue. |
| Native SDK | All 25 source modules mapped | [P5 design](p5-design.md) is delivered; native implementation/runtime acceptance is not complete. |
| Credentialed acceptance | Not completed | Financial, ownership v5, Sandbox reconciliation and protected write/read-back still require authorized environments. |

## Shared capabilities

The shared layer supplies key-kind isolation, fixed hosts/gateway prefixes,
per-client AWS-LC, pooled connections, concurrency/body-size bounds, pacing and
Retry-After cooldown, overall deadlines, conservative replay and cancellation
release. Metrics, error redaction, secret clearing, exact wire numbers and batched
profile/ban helpers reuse that layer. Rust xtask supports release workflows,
archive allowlists and checksum checks. See the [usage guide](usage.md) for limits.
Legacy login transports prepared ciphertext; undocumented RSA/AES login creation,
local Encrypted App Ticket decryption and native session callbacks are separate
capabilities, not implemented HTTP operations.

## Implemented feature inventory

T = typed model; A = explicit empty acknowledgement; O = validated opaque JSON. These labels describe the public return type, not whether Steam executed a business operation. Full per-method parameters and paths are in the machine inventory.

| Interface | Routes | Implemented functions | Response form |
| --- | ---: | --- | --- |
| IBroadcastService | 1 | Single-send game data frames | O1 |
| ICheatReportingService | 7 | Cheat reports, ban requests/removal, VAC readiness and secure sessions | O5 / T2 |
| ICloudService | 6 | OAuth file listing, staged upload batches, commit/completion and deletion | T4 / A2 |
| IEconMarketService | 4 | Eligibility, listing cancellation, sold asset IDs and popular items | O4 |
| IEconService | 7 | Trade history, offers, summaries and cache invalidation | O7 |
| IGameInventory | 5 | History, administrative commands, asset support history and definition import | O5 |
| IGameNotificationsService | 7 | Session create/update/list/details, notification requests and deletion | O7 |
| IGameServersService | 9 | Account/token administration and server IP/ID lookup | O9 |
| IInventoryService | 10 | Item/promo grants, consumption, crafting, definitions, prices, quantity and properties | O10 |
| ILobbyMatchmakingService | 3 | Lobby creation, member removal and data reads | O3 |
| IPartnerFinancialsService | 3 | Changed dates, detailed sales and wishlist reports | T3 |
| IPlayerService | 6 | Recent/single-game playtime, owned games, level, badges and badge progress | O6 |
| IPublishedFileModerationService | 2 | Approval queues and revision-aware moderation | O2 |
| IPublishedFileService | 7 | Workshop queries/deletion, metadata, bans, compatibility and tags | O7 |
| ISiteLicenseService | 2 | Current PC Cafe connections and interval playtime reports | T2 |
| ISteamApps | 10 | Betas, builds, depots, app lists, bans, servers, live builds and version checks | O8 / T2 |
| ISteamCommunity | 1 | Explicit abuse reports | O1 |
| ISteamEconomy | 8 | Tradeability, trade start, classes/prices/exports and asset transactions | O8 |
| ISteamGameServerStats | 1 | Historical player statistics with local-time ranges | O1 |
| ISteamLeaderboards | 7 | Create/find/delete/reset boards, read entries and set/delete scores | O7 |
| ISteamMicroTxn | 10 | Production orders, refunds, recurring agreements, reports and user info | O10 |
| ISteamMicroTxnSandbox | 10 | Inherited transaction workflows against the Sandbox database | O10 |
| ISteamNews | 2 | Public and authenticated app news | O2 |
| ISteamPublishedItemSearch | 4 | Publication/trend/vote ranking and result summaries | O4 |
| ISteamPublishedItemVoting | 2 | Item and user vote summaries | O2 |
| ISteamRemoteStorage | 7 | Subscriptions, collection/file/UGC details and GC usage | O7 |
| ISteamUser | 9 | Ownership, prices, deleted IDs, friends, bans, profiles, groups and vanity | T8 / O1 |
| ISteamUserAuth | 2 | Web ticket verification and prepared legacy encrypted login | O1 / T1 |
| ISteamUserStats | 7 | Global/user stats, achievements, schemas and uint32 stat writes | T6 / O1 |
| ISteamWebAPIUtil | 2 | Server metadata and API discovery | O2 |
| IStoreService | 1 | Filtered and paginated store catalogue | O1 |
| IUserReviewsService | 1 | Review filters, hardware/playtime selectors and cursor pages | T1 |
| IWorkshopService | 4 | Payment rules, contributors, revenue and localized descriptions | O4 |

## Contract details checked

- `AuthenticateUserTicket` accepts a game-server User key on Public and Publisher keys on trusted backends, with identity binding and no replay. A new mock fixture explicitly checks the User-key case and a 503 response under a nonzero read retry budget.
- `SetAppBuildLive.steamid` is conditional: public branch plus released application. The client preserves omission when release state is unknown, and 201 remains a confirmation request rather than deployment completion. A new fixture checks the omitted-account public-branch request.
- HTML5 parsing recovers AddItem's `trade_restriction` row despite a missing closing row tag. Existing bindings already transmit the optional flag; the formal required marker conflicts with the page's optional prose.
- `ISteamApps/GetAppList` is explicitly deprecated and publishes no parameter table; `GetServerInfo` also publishes none. The audit never borrows the following method's table. Store pagination remains available through `IStoreService/GetAppList`.
- Financial paths retain v001; UpdateItemDefs retains v0001 without inventing a trailing slash. Cloud OAuth, array spelling, conditional integers, signed prices/cursors and UTC/local-date boundaries retain their existing contracts.
- Complex Service messages remain explicit JSON when no full nested schema is published. Undocumented responses remain opaque. Caller-managed uploads, cursors, reconciliation and write outcome checks are separate workflow responsibilities.

## Remaining work

The [discovery snapshot](api-discovery.json) contains all 63 observed signatures and parameter names/types/optional flags, without invoking them. The report lists all 53 unbound observations. They include legacy versions, application-suffixed namespaces, extra methods on existing interfaces, and other services such as Authentication, Directory, ContentServerDirectory, Wishlist and HelpRequestLogs. They are candidates, not permission grants or complete protobuf specifications. The current preferred-version binding may already replace a legacy signature.

| Priority | Work | Completion gate |
| --- | --- | --- |
| A1 | Authorized P0–P4 read/permission acceptance | App-scoped credentials, identity/ownership evidence and documented failure states; no private payloads in reports. |
| A2 | Sandbox transaction reconciliation and protected write/read-back | Explicit disposable test resources and outcome reconciliation, including ambiguous network outcomes; production writes are separate. |
| A3 | Typed expansions where an authoritative response schema exists | Preserve conditional/private fields, exact numbers and unknown data; no guessed empty successes. |
| P5 | Native companion implementation | Follow P5.1–P5.4 design gates; the HTTP crate stays SDK-independent. |
| P6 | Triage the 53 discovery-only observations | Confirm supported contracts, credentials, replay and lifecycle before adding explicit bindings. Auth session/Steam Guard and game-specific methods need separate review. |

P0–P4 binding delivery, operational acceptance, P5 implementation and discovery-only expansion are tracked separately. The project cannot honestly claim universal feature or production completeness at this stage.

## Reproduce and maintain

```sh
cargo xtask coverage --json
cargo xtask audit-api
cargo xtask audit-api --json
cargo xtask audit-api --live
cargo xtask audit-api --reference-dir /path/to/captured-pages --json
```

Default mode uses committed snapshots and performs no network calls. Live mode fetches the index and every indexed page with four bounded workers, refreshes anonymous discovery and checks the upstream SDK tree. It never invokes listed methods beyond the keyless discovery read. Changed contracts or SDK references require review and produce a nonzero exit; the tool does not rewrite bindings or snapshots. `--output` saves a legal-header JSON report. Captured-page mode reads index.html and interface-named HTML files; SDK/discovery facts in that mode remain the committed baseline.

The maintenance-only scraper 0.29.0 dependency uses HTML5 DOM semantics with default features disabled. It is not a library dependency. All audit snapshots, this report, P5 design, tests and xtask stay outside the published archive. CI stores offline audit JSON alongside coverage in `steam-api-web-coverage`.
