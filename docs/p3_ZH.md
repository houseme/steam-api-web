<!--
Copyright 2026 steam-api-web contributors.
SPDX-License-Identifier: Apache-2.0
Licensed under the Apache License, Version 2.0.
See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.
-->

# 0.4 微交易、反作弊与游戏会话

[English](p3.md) | 简体中文

0.4.0 新增六个接口的 38 条方法路径，总计覆盖 30 个接口、161 条路径，
其中 151 条为显式文档路径，10 条为继承的 Sandbox 路径。
[机器清单](api-inventory.json)记录各封装及版本。这些实现已通过本地协议测试；
Publisher 凭据集成与生产运行仍是独立验证步骤。

| 接口 | 路径数 | 编码与权限 |
| --- | ---: | --- |
| [ISteamMicroTxn](https://partner.steamgames.com/doc/webapi/ISteamMicroTxn) | 10 | 普通及索引表单；具备微交易权限的 Publisher key |
| [ISteamMicroTxnSandbox](https://partner.steamgames.com/doc/webapi/ISteamMicroTxnSandbox) | 10（继承） | 相同签名，独立测试数据库 |
| [ICheatReportingService](https://partner.steamgames.com/doc/webapi/ICheatReportingService) | 7 | `input_json`；具备反作弊访问权限的 Publisher key |
| [ISteamCommunity](https://partner.steamgames.com/doc/webapi/ISteamCommunity) | 1 | 普通表单；Publisher key |
| [IGameNotificationsService](https://partner.steamgames.com/doc/webapi/IGameNotificationsService) | 7 | `input_json`；Publisher key |
| [ILobbyMatchmakingService](https://partner.steamgames.com/doc/webapi/ILobbyMatchmakingService) | 3 | `input_json`；CreateLobby 使用用户 key，其余方法要求 Publisher key |

Publisher key 也满足客户端的用户 key 种类检查。Steam 仍会检查所有权、授权、
功能启用及 app 访问权限；本地凭据种类检查无法证明这些权限。

## 支付与核对

显式选择 `client.micro_txn_sandbox()` 或 `client.micro_txn_production()`。
两个前缀共用实现，绝不在数据库之间自动切换。
[只读示例](../examples/query_sandbox_transaction.rs)查询已有 Sandbox 交易，
不会执行购买、扣款或退款。

所有支付响应都要求字符串 `response.result` 等于 `OK`。缺失或格式错误的确认
会解码失败，明确的业务失败仍作为 API 错误返回。确认仅代表请求被接受，不代表
结算成功或可以发货。Opaque 响应保留未知交易状态、拒付、欺诈标记及部分退款，
交由应用作出决定。

[实施指南](https://partner.steamgames.com/doc/features/microtransactions/implementation)
将初始化、用户授权与最终扣款分开。先持久化商户订单号，再提交请求；获得授权后
才能 Finalize，浏览器返回本身不能证明批准。初始化结果不确定时遵循 Valve 的
新建交易指引；最终扣款结果不确定时通过 QueryTxn/GetReport 核对。
所有写入都仅发送一次，即使请求已有订单号。

金额保留文档规定宽度的有符号整数。`amount[i]` 是整行总价，crate 不会再乘数量。
Steam 对所有币种均使用百分之一单位，包括 JPY 和 KWD；应用换算应遵循
[当前币种规则](https://partner.steamgames.com/doc/store/pricing/currencies)。
crate 检查币种代码形式，不固化市场列表及各币种金额步长。购物车数量由输入切片
推导，描述和类别使用 Unicode 字符数限制；语言接受 `zh-CN`、`pt-BR`、
`es-419` 等受支持形式。

续费字段被组合，避免发送不完整配置。ProcessAgreement 被接受仅代表开始计费，
不能证明扣款完成；协议取消状态本身也无法确定已付费权益何时到期。参照
[续费指南](https://partner.steamgames.com/doc/features/microtransactions/recurring_billing)
核对付款事件。文档允许的 Steam 商店订阅场景保留零订单号。

GetReport 是权益核对数据源，不是财务报表。时间戳游标保留小数精度；即使返回
短批次，也应继续使用最新返回记录的更新时间，空批次则保留原游标。
调用方负责轮询、去重、持久化和发货；客户端不会访问返回的授权 URL。

## 举报、封禁与 VAC 状态

ReportPlayerCheating 记录指控。后端应验证举报双方身份、审查证据，再在确有依据时
使用原始举报 ID 显式调用 RequestPlayerGameBan，不存在自动举报到封禁的流程。
永久封禁要求 `GameBanDuration::Permanent`，临时封禁不能使用零秒。
正时长保留完整 uint32 范围。举报查询游标与可选过滤器原样传递，不虚构分页或默认值。

RequestVacStatusForUser 使用专属响应策略：`success=false` 可以代表 VAC 尚未就绪。
此例外仅适用于规范 `response` 封装内的该布尔值；明确错误及其他响应封装仍严格检查。
`success=true` 且 `session_verified=false` 仍未验证，验证字段缺失也保留缺失状态。
后续 VAC 超时事件可能使先前结果失效，封装不会据此自动允许匹配。

VAC 参考文档仅描述标量语义，没有完整 JSON 封装示例。VAC 和
EndSecureMultiplayerSession 采用规范 Service `response` 封装，此选择仍待
凭据环境验证；其他不完整 schema 保持 opaque。Web API 安全会话方法需要 Valve
手动启用，仅适用于独立于 Steam 完成身份验证的游戏。使用 Steam Authentication
的游戏应遵循原生 P2P 或游戏服务器流程。

社区举报保留准确字段大小写：`steamidActor`、`steamidTarget`、`abuseType`、
`contentType`。文档未枚举的类型保持数值；提交举报不代表审核决定已经形成。

## 通知会话与大厅

UpdateSession 的正文描述替换语义，包括移除遗漏用户。可选字段保留省略与显式空值
的区别；调用方应按替换语义构造更新。GetSessionDetails 保留嵌套会话选择器。
未公开的本地化、用户及消息对象保留字段，Debug 输出经过脱敏。

DeleteSessionBatch 虽然名称含 Batch，文档只规定单个标量 `sessionid`，封装不虚构
ID 数组。通知请求、更新和删除都由调用方显式发起，没有后台自动轮询或清理。

CreateLobby 采用文档中的 `lobby_type` 字段，未照搬示例中不一致的 `lobbytype`。
类型 4 会复用具名唯一大厅并替换允许成员列表，显式空列表可移除所有预留位置。
元数据键值对及完整宽度 Steam ID 原样保留。大厅 Web API 不代替原生 SDK 的加入、
回调或在线状态跟踪。

## 验证与分发

本地测试覆盖全部路径、HTTP 方法、凭据种类及编码，并验证写入不重放、读取重试、
整数精度、响应错误、VAC 未就绪状态和核对边界。本次未执行真实支付、封禁、
社区举报、通知或大厅写入；生产权限及实际结果仍需凭据环境验证。

维护命令使用 Rust xtask，仓库没有 Python 源码。Cargo include 白名单保留库源码、
可运行示例、中英文 README/CHANGELOG 和许可证；测试、基准、xtask、workflow、
这些文档与参考清单仅留在 GitHub。归档策略检查及独立解压构建验证分发边界。

2026-10-21 在 stable Rust 1.99.1 下，本地已通过 168 项 workspace 测试、
格式检查、严格 Clippy、无警告 rustdoc、包含预发布版本的依赖审计、归档策略
及 48 文件发布包的独立解压构建。Rustls 预览版兼容性例外见
[dependencies_ZH.md](dependencies_ZH.md)。跨平台 CI 结果单独确认。
