<!--
Copyright 2026 steam-api-web contributors.
SPDX-License-Identifier: Apache-2.0
Licensed under the Apache License, Version 2.0.
See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.
-->

# 0.4 分支审查与请求构造

[English](review-0.4.md) | 简体中文

审查日期为 2026-10-21，范围从 v0.3.0（`852064d`）到 `ee0e71b`，包含
P3 调用方、共享响应处理、请求编码、URL 缓存及测试。另一任务正在重组 main，
因此本次工作隔离在 `houseme/review-0-4`，保留其文件和提交历史。

## 发现与重构

接口模块共享传输与编码器，微交易环境也明确，无需整体重写。局部问题在于
响应验证器为 VAC 单独展开了一套遍历，而请求构造为不可重试调用保留了多余 URL 副本。

**Major / P2：明确的 VAC 错误丢失 API 错误码。** 原验证器先要求
`response.success`，再检查该对象的错误。因此
`{"response":{"error":{"errorcode":15}}}` 被误报为 `Error::Decode`，
而非 `Error::Api { code: Some(15), .. }`。新增回归测试已先复现失败，再验证修复。
集成测试也改为对全部八个反作弊/社区方法断言错误类型和错误码，避免只检查失败及脱敏。

现在所有响应策略共用一条递归错误检查路径，通过具名作用域，仅对直接规范
`response` 内的 VAC success 推迟确认检查。明确错误检查之后，再要求 VAC 布尔就绪
字段或支付 `OK` 确认。根、兄弟及嵌套错误仍严格处理；类型化和 opaque 调用继续
共用验证，并在验证完成后才累计成功指标。

**Informational：单次发送有多余 URL 副本。** 原执行循环为所有尝试克隆 URL，
包括写入和重放预算为零的 GET。现在单次发送直接将 URL 与 POST 缓冲移交 reqwest。
仅观察性读取进入重试循环，查询串只编码一次，前面的尝试克隆 URL，最后一次直接
消费原 URL。实际发送函数统一持有并发许可、节流及尝试指标，所有路径都受其控制，
且在重试退避前释放许可。

公开签名、依赖版本及端点/重放分类保持兼容。客户端缓存仍以固定内部路径为上界，
不存查询参数。短临界区的同步锁没有可确认的正确性缺陷，也无测量证据支持替换整个同步框架。

## 测量与被放弃的设计

```sh
cargo bench --bench request --locked --offline -- --preparation-only
```

参考算法从 `ee0e71b` 提取，保存在 `benches/support/preparation_v040.rs`；
候选直接编译生产 `request::Endpoint::request`。两边都克隆相同输入和 URL fixture，
计时包含构造及输出析构，之前必须通过 HTTP 方法、URL、头和请求体等价断言。
九轮 A/B/B/A 取中位数，基线漂移超过 10% 即拒绝解释结果。不发送网络请求，
也不将 URL 缓存查询计入此项测量。

stable Rust 1.99.1、优化 profile、一次通过漂移门槛的本地结果：

| 请求构造 | 原实现 ns/op | 当前 ns/op | CPU 耗时减少 | 基线漂移 |
| --- | ---: | ---: | ---: | ---: |
| 单次 GET | 148 | 122 | 17.2% | -0.7% |
| 4 KiB 输入的单次 GET | 3,753 | 3,735 | 0.5% | -0.9% |
| POST | 131 | 105 | 19.6% | -1.9% |

4 KiB 场景没有显著收益。以上是请求构造 CPU 测量，不能换算为 Steam 时延、
传输吞吐或并发缓存竞争结果。

首次全量编码基准因 35% 基线漂移而中止，未用于归因。临时的每次重试重新编码方案
使三次发送、4 KiB 读取探针耗时约增至 2.8 倍，已放弃；通用 RequestBuilder 克隆
迭代器也使小型首次读取构造回退约 38%，同样放弃。这两种设计都未留在生产代码或
基准中。最终读取循环复用已编码查询串，不增加该迭代器。

## 验证与分发

本地通过 170 项 workspace 测试、格式检查、全部目标严格 Clippy、无警告 rustdoc、
仓库及发布元数据检查。新增回归验证 POST 请求体继承原 String 分配，缓存 URL 在
不同读取之间仍无 query；既有 fixture 覆盖重试耗尽、最后尝试恢复、冷却、期限、并发与取消。

48 文件归档继续排除测试、基准、xtask、workflow 和文档。为保留另一任务的生成物，
打包使用独立 target 目录及 `--no-verify`，随后通过归档策略检查和独立解压后的
`cargo check --all-targets --locked --offline` 验证。仓库没有 Python 源码。
跨平台 CI 和真实 Publisher/支付/封禁操作仍分别验证。本分支不发布 crate 或修改版本标签。
