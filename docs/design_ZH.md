<!--
Copyright 2026 steam-api-web contributors.
SPDX-License-Identifier: Apache-2.0
Licensed under the Apache License, Version 2.0.
See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.
-->

# Steam Web API 设计分析

[English](design.md) | 简体中文

核对日期：2026-10-21。以 Valve Web API 文档为协议依据，对照 [steamworks-rs](https://github.com/Noxime/steamworks-rs) 的 SDK 类型、票据回调和生命周期。

## 服务边界

`SteamClient` 持有共享 `Arc<Inner>`；`UserAuth`、`UserStats`、`User` 借用客户端，分别封装 2、7、9 个方法。接口对象不单独创建连接池。SDK 回调与 HTTP 后端请求的运行模型不同，因此不引入 steamworks/steamworks-sys 依赖或隐式初始化原生 SDK。

官方[概览](https://partner.steamgames.com/doc/webapi_overview)要求 HTTP/1.1、UTF-8 表单以及数组 `count`/`name[i]` 编码。实现固定方法路径与版本，读取使用 query，POST 使用 `application/x-www-form-urlencoded` body。rawbinary 参数逐字节转义，保留非 UTF-8、NUL 和表单保留字符。

Publisher 凭据默认选择 Partner。该域名的所有请求都要 Publisher key；403 可能造成进一步 IP 限流，因此不自动重试或切换域名。[官方 key 文档](https://partner.steamgames.com/doc/webapi_overview/auth)支持 `x-webapi-key` header，可避免把 key 写入 GET URL。Header 标记 sensitive，关闭重定向及 reqwest 内置重试。

本地检查凭据声明种类及参数；真实 key 权限、AppID 归属和 IP 白名单仍由 Steam 校验。Publisher 专用方法应部署到 Partner。

## 请求生命周期

私有 `request` 和 `response` 模块负责编码、端点策略及解码。类型化响应接收选中 JSON 的所有权，不透明响应保留原有容器；Service JSON 从借用字段的请求结构体直接序列化到表单编码器。[传输层 Review 与编解码测量](performance_ZH.md)记录重构依据、兼容性检查和可复跑的本地基准。

1. 验证 ID、非空文本、批次数量、日期范围、stat 名称与凭据类型。
2. 在总截止时间内等待并发许可、clone 共享请求节奏和限流冷却。
3. 复用连接发送请求，连接与单次请求分别有超时。
4. 非成功 HTTP 转换为结构化错误。`Retry-After` 支持秒数及 HTTP 日期，在可重试状态下建立共享冷却。存储 Instant 时超大秒数限为一年，每次调用仍受更短的总截止时间限制。
5. 成功响应同时检查声明长度和逐块累计长度，解码 JSON 后检查业务失败及预期 envelope。
6. 仅 replay-safe GET 可以有限重试传输错误和 408/429/500/502/503/504。重试释放并发许可，指数退避加 AWS-LC 随机抖动；再次发送仍须通过冷却。
7. 参数、JSON、业务错误不重试。认证和写入不自动重放；取消 Future 会释放许可。同步 JSON 解码受响应大小及解析深度限制，但 Tokio timeout 不能抢占同步 CPU 计算。

这是客户端故障隔离及资源控制，不是 Steam 远端 SLA。跨实例配额、部署冗余、熔断策略及报警由应用/平台层负责。身份、授权和封禁结果不会隐式缓存。持续故障返回明确错误，应用决定拒绝登录、排队或提示稍后重试。

## TLS 与凭据

关闭 reqwest 默认 features 和 native TLS，使用 Rustls 1.2/1.3。显式 `ClientConfig::builder_with_provider(aws_lc_rs::default_provider())` 配置每个客户端及平台证书验证器；不修改或采用宿主进程的全局 provider。独立测试进程安装不可用的全局 provider 后，客户端仍能构建。匿名 Steam HTTPS 读取另作网络验证。

`ApiKey`、`WebApiTicket` Debug 脱敏，自有字符串使用 zeroize；网络库、响应 JSON 及 OS 缓冲副本不保证清零。错误只保留固定 endpoint、HTTP 状态、数值业务码和解析坐标，不保留 URL、body、自由文本或原始 reqwest/serde 错误源。自定义 base URL 属于可信配置，要求 HTTPS，测试允许 loopback HTTP。

## steamworks-rs 对照

对照的[用户接口源码](https://github.com/Noxime/steamworks-rs/blob/master/src/user.rs)与[共享类型](https://github.com/Noxime/steamworks-rs/blob/master/src/lib.rs)：

| SDK 概念 | 本 crate 的映射与处理 |
| --- | --- |
| SteamId::raw/from_raw | 64 位整数桥接，JSON 数字或字符串均不转浮点 |
| AppId / GameId | u32 / u64 独立处理，禁止窄化混用 |
| authentication_session_ticket_for_webapi(identity) | 后端 AuthenticateUserTicket 使用同一非空 identity |
| TicketForWebApiResponse.result | 客户端必须先检查成功，再发送准备好的票据 |
| TicketForWebApiResponse.ticket_len / ticket | 参考 wrapper 把整个固定数组转为 Vec；from_callback_bytes 检查有效前缀 |
| AuthTicket | 原生取消句柄，不是票据内容 |
| GetAuthSessionTicket / BeginAuthSession | P2P/游戏服务器协议，不能替代 Web API ticket |
| cancel_authentication_ticket / run_callbacks | 调用方管理 SDK 初始化、回调与取消 |
| 用户统计整数/浮点操作 | Web 读取保留数值；Web 写入按其文档只接受 uint32，不照搬 SDK 浮点 setter |

网络层自身无 unsafe，不复制 SDK 的 FFI、回调循环或原生状态机。对照源码链接使用 master，记录的是本次观察，之后可能改变。全部源码模块及固定源码 revision 的映射见[路线图](roadmap_ZH.md)。

## 响应建模与未公开协议

[响应说明](https://partner.steamgames.com/doc/webapi_overview/responses)指出 uint64 常为字符串，本库同时接受数字或字符串。所有权 `ownsapp` 是必需 bool，缺失不授权；可见性受限字段和 ban 标记为 Option，缺失不等于否定。私密好友列表 401 保持错误；vanity 无匹配按该方法语义返回 None。

`AuthenticateUser`、`GetDeletedSteamIDs`、`SetUserStatsForGame` 的链接没有完整响应 schema。它们通过 OpaqueResponse 保留原 JSON 并检查标准错误 envelope，不虚构字段。应用需结合真实 Publisher 返回核对写入结果、删除游标和分页终止条件。

旧式 AuthenticateUser 的 RSA 公钥、padding、AES 模式/IV 未在链接中定义，只接受已准备密文。32 字节是加密前随机 session key 的长度，不能用它检查 RSA 密文长度；这也不是 Encrypted Application Ticket 的本地解密流程。

[ISteamUser](https://partner.steamgames.com/doc/webapi/ISteamUser)的 GetPublisherAppOwnership 路径为 v4，但版本历史提到 v5 的 expiry。默认使用明确路径，另提供显式 v5，不通过自动失败回退掩盖版本差异；v4 的 timeexpires 为 Option。无 key 的公开 GetSupportedAPIList 不列出该 Publisher 方法，不能据此确认 v5 部署。

## 验证层次

- 原有 P0 协议测试：18 个方法的路径/版本、GET/POST、header、envelope、indexed forms 及所有 256 种原始字节；整数精度、可选字段、错误处理、认证/写入不重放、重试预算、共享冷却、总/单次超时、取消、批处理、响应限额、重定向及 provider 隔离。
- 本地静态验证：stable 的 fmt、严格 Clippy、rustdoc 和示例编译。
- 实际网络验证：2026-10-21 通过 current_players 示例读取 Public host 的 AppID 480，返回 23944。它只证明该时刻匿名 HTTPS/响应路径可用，不代表持续可用性或 Publisher 接口验证。
- Publisher 实测需有权限的 key、AppID 和票据，继续验证 identity 不匹配/过期/封禁、AppID 授权、写入后读回、删除游标、v5 expiry 和实际配额。未搜寻本地凭据，未执行真实统计写入。

已增加 Linux/macOS/Windows 测试 CI，保留已有审计和 SARIF 工作流；远端执行状态不能由本地验证替代。当前锁定 ICU 依赖声明最低 Rust 1.88，本次实际使用 stable 1.99.1，未安装或验证旧工具链。

## 0.2 P1 扩展与仓库边界

当前开发版新增九个 P1 接口、39 个方法：API 发现、应用、玩家、商店、评论、新闻、游戏服务器、服务器统计及排行榜。与 P0 合计 12 个接口、57 个方法。Service 方法使用 input_json；管理 POST 禁止自动重放；服务器登录 token 的输入和响应 Debug 脱敏；排行榜 details 保留原始字节且限制为 256 字节。

SetAppBuildLive 通过保留 HTTP 状态区分请求接受与 201 等待手机确认，只有明确允许的 201 可以没有响应体；空 200 仍报 Decode。共享错误检查识别 result/eresult，并拒绝小数结果码。评论及 Partner 应用列表按官方公开字段建模，未公开响应保留 OpaqueResponse，不把缺失结构解释成空列表或成功。

维护和发布保护迁移到独立、不可发布的 Rust xtask；工作流不再依赖 Python。Cargo include 白名单把仓库工具、集成测试和参考资料留在 GitHub，公开库不依赖 xtask。完整 workspace 测试与精简归档的独立构建分别验证，避免工具带来的 feature 合并掩盖公开库的依赖错误。P1 的本地测试不是实际 Steam Publisher 写入证据。
