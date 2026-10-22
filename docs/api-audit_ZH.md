<!--
Copyright 2026 steam-api-web contributors.
SPDX-License-Identifier: Apache-2.0
Licensed under the Apache License, Version 2.0.
See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.
-->

# Web API 功能完整性核对

[English](api-audit.md) | 简体中文

2026-10-22 重新下载[官方伙伴索引](https://partner.steamgames.com/doc/webapi)及全部 33 个接口页进行核查。匿名发现与固定 steamworks-rs 源码树沿用此前审阅的快照，本次直接刷新匿名发现时遭遇网络层失败。可执行报告为 `cargo xtask audit-api --json`，下载页面通过 `--reference-dir` 复核；数据见 [api-inventory.json](api-inventory.json) 和 [api-discovery.json](api-discovery.json)。

## 结论与证据层次

伙伴目录内签名已绑定完整：**33 个接口，157 条显式签名及 10 条 Sandbox 继承路径**。
本轮未发现漏绑方法/路径/版本或缺失的静态协议 fixture 引用。这不能证明整个 Steam
平台、所有发现方法或生产验收已经完成。

| 层次 | 核对结果 | 含义 |
| --- | --- | --- |
| 伙伴目录 | 167/167 路径映射 | 方法、动词、版本及路径准确；版本数字不抹掉 v001/v0001 或尾斜杠差异。 |
| 正式请求表 | 735 行：585 必填、149 可选、1 条件必填 | 是来源事实，不自动证明请求语义；正文/示例与标记矛盾仍单独复核。两个方法没有参数表。 |
| 协议 fixture | 167 条方法/接口静态引用 | workspace 运行聚合 wire/错误/重放测试；这不是逐方法线上测试覆盖率。 |
| 响应处理 | 29 类型化、2 空确认、136 校验后 opaque | 公开完整 JSON、统一检查错误，不虚构业务执行成功。 |
| 匿名发现 | 27 接口 / 63 签名，与绑定重叠 10 条 | 53 条观察签名在当前绑定目录之外；发现结果受可见性影响，不是认证后的完整目录。 |
| 原生 SDK | 25 个源码模块均已映射 | [P5 设计](p5-design_ZH.md)已交付，原生实现与运行验收未完成。 |
| 凭据验收 | 未完成 | 财务权限、所有权 v5、Sandbox 对账及受保护写入后读回仍需授权环境。 |

## 公共能力

共享层已经提供 key 类别隔离、固定主机和网关前缀、每客户端 AWS-LC、连接池复用、
并发/响应大小上限、节流与 Retry-After 冷却、整体截止时间、保守重试及取消释放。
指标、错误脱敏、凭据清零、精确数值和用户资料/封禁批量便捷方法继续复用该层。
发布工作流、归档白名单及校验和检查由 Rust xtask 支持。配置与限制见[使用指南](usage_ZH.md)。
旧式登录运输已准备的密文；自动构造未公开的 RSA/AES 登录协议、本地 Encrypted App
Ticket 解密和原生会话回调不属于当前 HTTP 封装能力。

## 已实现功能梳理

T = 类型化模型，A = 明确空确认，O = 校验后的 opaque JSON。这些标签表示公开返回
类型，不表示 Steam 已完成业务操作。逐方法参数和路径保存在机器清单。

| 接口 | 路径数 | 已实现功能 | 响应形式 |
| --- | ---: | --- | --- |
| IBroadcastService | 1 | 单次发送的游戏广播数据帧 | O1 |
| ICheatReportingService | 7 | 作弊举报、封禁申请/撤销、VAC 就绪、受保护会话 | O5 / T2 |
| ICloudService | 6 | OAuth 文件列举、分阶段上传批次、提交/完成、删除 | T4 / A2 |
| IEconMarketService | 4 | 市场资格、取消挂单、售出资产 ID、热门物品 | O4 |
| IEconService | 7 | 交易历史、报价、摘要、缓存失效 | O7 |
| IGameInventory | 5 | 历史记录、管理命令、资产支持历史、定义导入 | O5 |
| IGameNotificationsService | 7 | 会话创建/更新/列举/详情、通知请求、删除 | O7 |
| IGameServersService | 9 | 服务器账户/token 管理、IP/ID 查询 | O9 |
| IInventoryService | 10 | 物品/促销发放、消耗、合成、定义、价格、数量、属性修改 | O10 |
| ILobbyMatchmakingService | 3 | 大厅创建、移除成员、数据读取 | O3 |
| IPartnerFinancialsService | 3 | 变更日期、详细销售、愿望单报表 | T3 |
| IPlayerService | 6 | 最近/单游戏时长、已拥有游戏、等级、徽章及进度 | O6 |
| IPublishedFileModerationService | 2 | 审批队列、带 revision 的审核操作 | O2 |
| IPublishedFileService | 7 | 创意工坊查询/删除、元数据、封禁、兼容性、标签 | O7 |
| ISiteLicenseService | 2 | PC Cafe 当前连接、时间区间游玩报表 | T2 |
| ISteamApps | 10 | 分支、构建、仓库、应用列表、封禁、服务器、发布和版本检查 | O8 / T2 |
| ISteamCommunity | 1 | 显式滥用举报 | O1 |
| ISteamEconomy | 8 | 可交易性、发起交易、类别/价格/导出、资产交易 | O8 |
| ISteamGameServerStats | 1 | 按当地时间范围读取历史玩家统计 | O1 |
| ISteamLeaderboards | 7 | 创建/查找/删除/重置榜单、读取条目、设置/删除分数 | O7 |
| ISteamMicroTxn | 10 | 生产订单、退款、周期协议、报表、用户信息 | O10 |
| ISteamMicroTxnSandbox | 10 | 使用 Sandbox 数据库的继承交易流程 | O10 |
| ISteamNews | 2 | 公开及认证应用新闻 | O2 |
| ISteamPublishedItemSearch | 4 | 发布时间/趋势/投票排序及结果摘要 | O4 |
| ISteamPublishedItemVoting | 2 | 物品与用户投票摘要 | O2 |
| ISteamRemoteStorage | 7 | 订阅、集合/文件/UGC 详情、GC 使用状态 | O7 |
| ISteamUser | 9 | 所有权、价格、删除 ID、好友、封禁、资料、用户组、短网址 | T8 / O1 |
| ISteamUserAuth | 2 | Web 票据验证、已准备的旧式加密登录 | O1 / T1 |
| ISteamUserStats | 7 | 全局/用户统计、成就、schema、uint32 统计写入 | T6 / O1 |
| ISteamWebAPIUtil | 2 | 服务器元数据、API 发现 | O2 |
| IStoreService | 1 | 商店目录过滤与分页 | O1 |
| IUserReviewsService | 1 | 评论过滤、硬件/时长筛选、cursor 分页 | T1 |
| IWorkshopService | 4 | 付款规则、贡献者、收益、本地化描述 | O4 |

## 示例与公开文档

[示例指南](examples_ZH.md)将 38 个可运行程序映射到全部 33 个接口族；全部 160 个公开异步方法均有英文错误说明及可编译 rustdoc 调用示例。本次重新读取索引和全部方法页，发现基线缺少 GetPlayersBanned 的三个可选过滤参数；现已通过 BannedPlayersOptions 补齐 starttime、endtime、continue_token，原便捷方法保持兼容。受保护操作的真实执行仍单独验收。匿名发现直接刷新遭遇网络失败，本次独立更新参考页面，下文发现及 SDK 数据保留此前快照。

## 已核对的契约细节

- `AuthenticateUserTicket` 支持 Public 上的游戏服务器 User key 和可信后端 Publisher
  key，保留 identity 绑定且不重放。新增 mock 验证 User-key 成功及非零重试预算下的 503 单次发送。
- `SetAppBuildLive.steamid` 仅在 public 分支且应用已发布时要求。客户端不知道发布状态
  时保留省略，201 仍表示等待确认，不代表发布完成；新增 public 分支省略账户的请求 fixture。
- HTML5 解析恢复 AddItem 因缺失闭合行标签而易漏掉的 `trade_restriction`；现有绑定已经
  支持该可选参数，表格必填标记与正文 Optional 的矛盾分别记录。
- `ISteamApps/GetAppList` 已明确弃用且没有参数表，GetServerInfo 也未列参数表。
  审计器不借用后续方法的表格；目录分页仍可使用 `IStoreService/GetAppList`。
- 财务保留 v001，UpdateItemDefs 保留 v0001 且不追加不存在的斜杠；Cloud OAuth、数组
  拼写、条件整数、带符号价格/游标以及 UTC/当地时间边界继续遵守原契约。
- 未公开完整嵌套 schema 的 Service 消息仍使用显式 JSON，未公开响应保留 opaque。
  文件传输、游标保存、对账和写入结果确认仍由调用方工作流负责。

## 剩余规划

[发现快照](api-discovery.json)保存全部 63 条观察签名及参数名/类型/可选标记，未调用
这些方法。审计报告列出 53 条未绑定观察，包括旧版本、应用编号命名空间、既有接口
扩展和 Authentication、Directory、ContentServerDirectory、Wishlist、HelpRequestLogs
等服务。它们是候选，不代表权限或完整 protobuf schema；当前首选版本也可能已替代旧签名。

| 优先级 | 后续工作 | 完成门槛 |
| --- | --- | --- |
| A1 | P0–P4 授权读取/权限验收 | 对应 App 的凭据、identity/所有权证据与失败状态，报告不含私密载荷。 |
| A2 | Sandbox 交易对账和受保护写入后读回 | 明确的可丢弃测试资源与结果核对，覆盖网络结果不明；生产写入另行处理。 |
| A3 | 有权威响应 schema 的类型化扩展 | 保留条件/私密字段、精确数字与未知数据，不推测空成功。 |
| P5 | 原生 companion 实现 | 按 P5.1–P5.4 设计验收，HTTP 包继续不依赖 SDK。 |
| P6 | 53 条发现候选分类 | 先确认支持契约、凭据、重放和生命周期，再增加显式绑定；登录/Steam Guard 和游戏专属接口需单独核对。 |

P0–P4 的封装交付、运行验收、P5 实现和目录外扩展分别追踪，当前不能宣称所有 Steam
功能或生产验收已全面完成。

## 复跑与维护

```sh
cargo xtask coverage --json
cargo xtask audit-api
cargo xtask audit-api --json
cargo xtask audit-api --live
cargo xtask audit-api --reference-dir /path/to/captured-pages --json
```

默认读取已提交快照，不访问网络。live 模式以四个有界 worker 读取索引及每个接口页，
刷新匿名发现并核对上游 SDK 树；除无 key 发现读取外，不调用清单中的方法。契约或
SDK 引用变化时非零退出等待复核，不改写绑定或快照；`--output` 可保存带法律元数据
的 JSON。快照模式读取 index.html 和按接口命名的 HTML，其 SDK/发现事实仍取已提交基线。

维护工具专用 scraper 0.29.0 采用 HTML5 DOM 语义并关闭默认 feature，不属于运行库
依赖。审计快照、本文、P5 设计、测试及 xtask 均不进入发布归档。CI 将离线审计 JSON
与覆盖报告一起存入 `steam-api-web-coverage`。
