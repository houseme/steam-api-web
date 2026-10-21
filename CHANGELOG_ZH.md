<!--
Copyright 2026 steam-api-web contributors.
SPDX-License-Identifier: Apache-2.0
Licensed under the Apache License, Version 2.0.
See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.
-->

# 变更记录

[English](CHANGELOG.md) | 简体中文

记录面向用户的重要变更，版本遵循语义化版本规则。标记为 Unreleased 的准备条目不代表已经发布。

## [Unreleased]

### 规划

- 按完整 Web API 与原生 SDK 互操作路线图扩展覆盖范围。
- 使用有权限的凭据验证 Publisher 接口、删除游标、写入结果和所有权 v5。

## [0.4.0] - Unreleased

### 新增

- P3 全部 38 条路径：微交易生产及 Sandbox 各 10 条，反作弊/社区 8 条，通知/大厅 10 条；累计 30 个接口、161 条路径，包含 10 条继承 Sandbox 路径。
- 显式微交易环境选择、精确整数金额和 ID、周期协议及对账 cursor；生产环境不作为 Sandbox 失败后的后备目标，全部写入保持单次发送。
- 支付响应必须显式 result=OK；仅 VAC 专用响应允许 success=false 表示尚未就绪，错误和未验证状态不被隐藏。
- 通知会话、显式封禁/举报、大厅请求、Sandbox 只读查询示例，以及完整本地协议测试和双语 P3 契约。

### 调整

- 覆盖率检查分别核对显式和继承路径，避免混淆 Sandbox 镜像与独立公开签名。
- 索引表单键直接写入最终缓冲，每个客户端缓存固定端点 URL，不保留查询参数或凭据。
- 维护工具继续使用 Rust，发布包继续排除测试、基准、工具、工作流、文档及参考 JSON，均保留在 GitHub。

### 验证边界

- 未使用真实 Microtransaction/反作弊授权，没有执行真实支付、退款、封禁、举报、通知或大厅写入；真实权限、回调和持久化结果仍需验证。
- 0.4.0 尚未发布，既有版本标签不变。

## [0.3.0] - 2026-10-21

### 新增

- P2 全部 66 个方法：Cloud、库存、资产经济、交易、市场、远程 UGC、Workshop 搜索/投票/审核及付款配置；累计 24 个接口、123 个方法。
- Cloud OAuth、严格 x-eresult 校验、带签名地址的脱敏结果，以及显式上传批次、文件提交和结束阶段；文件字节传输由调用方管理。
- 库存 requestid、动态属性类型及并发时间戳、交易翻页、Workshop cursor/revision 和原样 v0001 物品定义路径。
- Cloud 只读示例、完整协议/错误/重放测试及双语 P2 契约说明。

### 调整

- 共享响应支持经过内部错误校验的对象型 result，保留严格标量 EResult；写入仍禁止自动重放，包括有副作用的 GET。
- 将响应契约集中到 ResponsePolicy，借用文本并在栈缓冲区格式化整数，消除表单标量临时字符串；新增对照原 0.3 编码器的离线基准。
- 仓库检查核对覆盖率/版本、公开模块及 P2 方法；维护代码继续使用 Rust，测试、基准、工具、工作流和资料继续排除于发布包。

### 验证边界

- 未使用真实 OAuth/Publisher 凭据，也没有写入 Cloud 文件、库存、交易、挂牌或 Workshop；实际权限、去重和持久化结果仍需验证。
- 创建标签和 registry 发布是独立步骤，实际发布状态以 registry 及发布工作流为准；已有 0.1.0、0.2.0 标签不变。

## [0.2.0] - 2026-10-21

### 新增

- P1 全部 39 个方法：API 发现、应用管理、玩家、商店、评论、新闻、游戏服务器与排行榜；累计 12 个接口、57 个方法。
- Service input_json、评论筛选及分页、排行榜原始字节、服务器 token 脱敏、按凭据和方法区分的重试策略。
- SetAppBuildLive 的 HTTP 200/201 状态区分，以及仅对等待确认的 201 接受空响应；共享错误检测新增 EResult 校验。
- 使用 Rust xtask 取代全部 Python 维护/发布脚本及 workflow 内嵌 Python，保留版本/标签/changelog/归档/registry 发布保护。
- 发布包显式白名单，将仓库工具、工作流、集成测试、规划资料和参考 JSON 保留在 GitHub；校验归档及独立构建。

### 调整

- 统一端点策略及表单/Service JSON 直接编码，类型化响应转移数据所有权，不透明 JSON 直接传递，保留公开 API 与可用性行为。
- 增加对照重构前算法的离线优化编解码基准，以及双语 Review 结论、可复跑测量和所有权回归测试。

### 验证边界

- 新接口通过本地协议测试；未执行 Steam Publisher 管理写入，未获得实际凭据验证或生产负载证据。
- 创建标签和 registry 发布是独立步骤，实际发布状态以 registry 及发布工作流为准；0.1.0 历史标签和已发布归档保持不变。

## [0.1.0] - 2026-10-21

### 新增

- 独立异步封装 ISteamUserAuth、ISteamUserStats、ISteamUser 的全部 18 个文档方法。
- 显式 AWS-LC TLS provider、连接复用、响应/并发上限、请求节奏、超时、有限重试和共享 Retry-After 冷却。
- 保守的认证/写入重放策略、结构化脱敏错误、自有凭据清零以及无损 SteamID/统计值解码。
- 对齐 steamworks-rs 的票据有效长度及原始 SteamID 转换，无需依赖原生 SDK。
- 中英文 README、设计和依赖记录、完整接口/模块路线图、发布说明及 Apache-2.0 文件版权头。
- 跨平台可复用 CI、默认手动 dry-run、标签/版本/changelog 校验、归档校验和比对、registry 验证与 GitHub 发布说明。
- 本地协议/故障测试及离线发布保护测试。

### 验证边界

- 打标签前，Linux、macOS、Windows stable CI、文档、工作流检查和分发归档验证均已通过。
- 已实测匿名 Steam HTTPS 读取；Publisher 认证、写入、v5 所有权和生产负载尚未验证。
- 三个官方响应 schema 不完整的方法保留原始结果；旧式登录要求调用方提供准备好的密文。
- Rustls 使用 reqwest 兼容的 0.23.46，而非类型不兼容的 0.24.0-dev.2 预览版。
