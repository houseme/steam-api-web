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
