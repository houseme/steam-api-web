<!--
Copyright 2026 steam-api-web contributors.
SPDX-License-Identifier: Apache-2.0
Licensed under the Apache License, Version 2.0.
See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.
-->

# 0.3 传输层 Review 与请求基准

[English](review-0.3.md) | 简体中文

审阅版本为 `2077350d9b8c6de4aad97e97f4f96f3fd07bf2cd`，包括 P2 及共享传输路径，该版本的[平台 CI](https://github.com/houseme/steam-api-web/actions/runs/39606087427)已通过。接口模块仍是共用客户端的薄封装，不需要整体推倒重写；本次确认两项性能/维护层面的信息性问题，没有证据将其定性为阻塞性协议错误。

## 已验证的观察与调整

- **表单值临时分配，Informational。** 原 [request.rs:110](https://github.com/houseme/steam-api-web/blob/2077350d9b8c6de4aad97e97f4f96f3fd07bf2cd/src/request.rs#L110)对每个标量调用 ToString，借用文本、OAuth token、整数和 Steam ID 都先产生堆字符串，再复制到最终表单。现在私有 FormValue 直接借用文本，整数使用 itoa 栈缓冲区；无需转义的 ASCII 前缀直接复制，其余字节仍由原编码器处理，包括非 UTF-8 rawbinary。公开签名和线上编码保持兼容。
- **响应契约职责分散，Informational。** request.rs 的 BodyPolicy、[client.rs:678](https://github.com/houseme/steam-api-web/blob/2077350d9b8c6de4aad97e97f4f96f3fd07bf2cd/src/client.rs#L678)中的 header/JSON 解析、response.rs 的业务解码共同控制响应规则。现在 ResponsePolicy 集中处理 header、允许的空响应及 JSON 根结构；客户端保留有限 I/O、截止时间、并发、限速和重试，header 仍在读取响应体前校验。

策略区分普通 JSON、允许空 HTTP 201 的构建确认、要求成功 result header 的 Cloud JSON，以及空 HTTP 200 的 Cloud acknowledgement。带响应体的 acknowledgement 直接由策略拒绝；此前也会在解码为 unit 时返回 Decode。严格 header、业务错误、脱敏和认证/写入单次发送规则继续受到测试覆盖。

直接依赖 itoa 1.0.18 已经存在于原解析依赖图，本次增加直接依赖边，没有新增解析包。13 个直接依赖已在线核对，包括预发布，Rustls 兼容性例外不变，详见[依赖证据](dependencies_ZH.md)。

## 复跑测量

```sh
cargo bench --bench request --locked --offline
```

基准直接编译生产编码器，将原编码器保留在仅供仓库使用的支持代码中。两边输入相同，计时前校验完整线上编码等价，包括全部 256 个原始字节值。输入在计时外准备，输出析构计入耗时。

每项预热 100 次，交错执行九轮 A/B/B/A，取各实现 18 个样本的中位数；基线漂移超过 10% 会失败。以下 stable Rust 1.99.1 优化构建运行中，各项漂移均低于 2%：

| 场景 | 原实现 ns/op | 当前 ns/op | 实测耗时减少 |
| --- | ---: | ---: | ---: |
| 普通标量表单 | 122 | 55 | 55.2% |
| 约 4 KiB OAuth token 的 Cloud 表单 | 2,863 | 1,790 | 37.5% |
| 100 个索引 ID | 4,867 | 3,047 | 37.4% |
| 100 个 JSON 定义 | 107,610 | 106,274 | 1.2%，基本持平 |

这些是本地编码 CPU 测量，不代表 Steam 端到端延迟，也不证明全局最优。接近噪声的小差异不能证明改善。请求仍需分配最终缓冲区，索引键格式化也可能分配；没有统计分配次数或执行生产压测。

选型前测量了通用 URL 字节迭代器替换，JSON 定义场景反而慢约 38%。因此保留一个字节编码器，仅加速借用标量的 ASCII 前缀；JSON 继续直接写表单，避免该项回归。

## 验证与分发

130 项 workspace 测试通过，包括整数极值、UTF-8/控制字节编码等价、响应策略矩阵及既有 P0/P1/P2 协议测试。格式化、严格 Clippy、无警告 rustdoc、归档策略及独立解压构建分别检查；没有执行真实 OAuth/Publisher 写入。

公开能力仍为 24 个接口、123 个方法，继续使用 Rust xtask 和 AWS-LC，没有引入 Python。新基准及本文通过既有白名单仅留在 GitHub；分发包仍保留 42 个源码、示例、许可及双语 README/changelog 等文件。
