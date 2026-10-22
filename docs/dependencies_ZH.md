<!--
Copyright 2026 steam-api-web contributors.
SPDX-License-Identifier: Apache-2.0
Licensed under the Apache License, Version 2.0.
See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.
-->

# 依赖版本记录

[English](dependencies.md) | 简体中文

核对日期：2026-10-22。来源为各 crate 的 crates.io API，排除 yanked，按语义版本排序，包含 rc、beta、alpha 和 dev 预发布。可复核快照见 [dependency-versions.json](dependency-versions.json)。

全部直接依赖的版本与公共配置统一定义在根目录 `Cargo.toml` 的 `[workspace.dependencies]` 中，运行库和 `xtask` 通过 `workspace = true` 继承。成员专用 feature 按需启用，例如 `xtask` 的 blocking HTTP 客户端及 Tokio 的测试运行时。Cargo 会将继承配置展开到发布 manifest，确保 crate 脱离仓库工作区后仍可独立编译。

| 直接依赖 | 使用版本 | 用途 |
| --- | --- | --- |
| aws-lc-rs | 1.18.2 | 密码后端与随机抖动 |
| reqwest | 0.13.5 | HTTP/1.1、连接池、超时 |
| rustls | 0.23.46 | 显式逐客户端 AWS-LC provider |
| rustls-platform-verifier | 0.7.1 | 平台证书验证 |
| serde | 1.0.229 | 类型化响应 |
| serde_json | 1.0.154 | JSON 及精确整数 |
| tokio | 1.54.1 | async、并发、节流与超时 |
| thiserror | 2.0.21 | 可匹配错误类型 |
| url | 2.5.8 | URL 与 base URL 校验 |
| httpdate | 1.0.3 | Retry-After 日期 |
| itoa | 1.0.18 | 表单整数的栈缓冲区格式化 |
| zeroize | 1.9.1 | 自有凭据清零 |
| wiremock（测试） | 0.6.5 | HTTP 协议与故障测试 |
| scraper（仅 xtask） | 0.29.0 | 维护工具 HTML5 契约审计，关闭默认 feature |

Rustls 的最高版本为 **0.24.0-dev.2**，但 reqwest 0.13.5 使用 Rustls 0.23；`tls_backend_preconfigured` 根据类型识别 ClientConfig。将两个版本同时编入不能让其互操作，会在构造 client 时失败。采用同一条 0.23 的最新版本 0.23.46 是最高版本要求的兼容性例外；后续 reqwest 支持时同步升级。其余直接依赖均为查询时的最高语义版本，当前没有更高 rc/beta。

正常依赖图使用 AWS-LC，不含 ring 或 native-tls；按客户端显式绑定 AWS-LC，不依赖全局 provider。macOS 的 platform verifier 使用 Security.framework 进行证书验证，它不是 native-tls 传输后端。

Cargo.lock 已在线 cargo update，传递依赖取当前约束及工具链允许的最新兼容版本。Cargo 不会自动越过类型不兼容的大版本，也不自动选择未显式要求的传递预发布包；强行替换所有传递依赖会破坏上游约束。最新 ICU 2.3 要求 Rust 1.88；本次验证使用 stable 1.99.1。

```sh
cargo xtask check-dependencies
cargo update
cargo tree --package steam-api-web --edges normal -i aws-lc-rs
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
```

检查脚本只读，使用 Rust xtask、cargo metadata、有限工作线程及显式 AWS-LC HTTP 客户端，也检查新预发布；已记录的 Rustls 不兼容预览版单独标注。如果出现其他新版本或更高预览版会退出非零。脚本不自动修改 manifest、lockfile 或快照。生成新快照可使用 `cargo xtask check-dependencies --json > docs/dependency-versions.json`，随后审阅差异并验证。

JSON 清单作为共享的语言无关证据保留一份。cargo update 若重写 Cargo.lock，需要保留仓库版权/许可头。工作流采用已核对的 checkout v7、upload-artifact v7、download-artifact v8 和 action-gh-release v3；CI 用最新 actionlint，本地验证为 1.7.12。这些构建工具不改变 crate 运行时依赖图。

`xtask` 是 publish=false 的独立 workspace 成员，其 blocking HTTP 与审计依赖不进入公开 crate 的依赖图。项目源码和 workflow 不再依赖 Python。

本轮全部 14 个直接依赖已重新核对，包含预发布版本；运行库依赖不变。scraper 不进入发布库依赖图或归档。
