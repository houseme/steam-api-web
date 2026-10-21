<!--
Copyright 2026 steam-api-web contributors.
SPDX-License-Identifier: Apache-2.0
Licensed under the Apache License, Version 2.0.
See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.
-->

# 0.5 财务报表、场所许可与广播

[English](p4.md) | 简体中文

P4 新增六个方法，完成 33 接口文档快照的封装覆盖：167 条路径，包括 157 条显式签名
及 10 条继承 Sandbox 路径。这是 HTTP 协议覆盖；生产权限、私有 API 与原生 SDK 能力
仍分别处理。[路线图](roadmap_ZH.md)及[机器清单](api-inventory.json)保留这些边界。

## 财务权限与报表

[IPartnerFinancialsService](https://partner.steamgames.com/doc/webapi/IPartnerFinancialsService)
需要独立 Financial API Group key，具备伙伴账户范围的可见性，涉及全部应用和有权访问
的 view grant。在独立可信后端客户端中使用 `ApiKey::financial(...)`，User/Publisher
key 不满足本地财务访问规则，Financial key 也不能在本 crate 中访问其他凭据类别的方法。
声明 key 种类不代表已获 Steam 授权；应在 Steamworks 配置组及 IP 白名单。
key 保持清零与脱敏，通过敏感 `x-webapi-key` 头传递，不放入 `input_json`。

三个方法全部为 GET，保留准确 `v001` 拼写，Service JSON 编码由
[概览](https://partner.steamgames.com/doc/webapi_overview)支持。读取重试有界，客户端
不自动切换主机或读取下一页，财务客户端默认使用 Partner。

两个销售游标职责不同：

1. GetChangedDatesForPartner 从 `highwatermark=0` 开始。已处理日期可能因结算或重新
   计算再次出现。
2. 对每个返回日期，GetDetailedSales 从 `highwatermark_id=0` 重新读取并重建完整本地
   快照，再使用该日期返回的 max_id 继续。
3. 返回 max_id 等于本次输入游标才代表该日期完成；空结果或短批次不能独立证明结束。
4. 全部日期成功后才能保存外层 result_highwatermark。crate 不保存游标、替换记录或
   合并不完整快照。

外层游标保留精确字符串，日期内游标接受 int64 或十进制文本，负数不能强转 uint64。
请求游标仍为 uint64，由调用方显式执行检查转换。销售日期采用 Pacific Time，愿望单
日期采用 GMT；接受的日历格式原样发送，不转换到 UTC 或虚构报表区间。

本地币种价格保留 uint64 分单位，USD 金额保留原始带符号十进制字符串；数量和分类
遵循文档类型。package、microtransaction、key 的条件字段保持可选，未知元数据及
lookup 数组留在 `extra`。超出可用范围的愿望单响应可仅含 app_min_date，不虚构零活动。
Debug 隐藏日期、游标、金额、名称及未知字段。销售聚合不代表支付授权，也不代替正式付款报表。

[示例](../examples/financial_changed_dates.rs)只读取一批变更并输出日期数量，不记录
财报值、不推进游标；必须由调用方显式提供 Financial key 才能访问账户。

## 站点活动与报表区间

[ISiteLicenseService](https://partner.steamgames.com/doc/webapi/ISiteLicenseService)
的两个 GET 要求 Publisher key。方法页 URL 使用 Public，可用 `host(Host::Public)`
显式对齐；概览推荐可信 Publisher 后端使用 Partner，默认保持该选择。主机固定，
不会自动跨域切换，参数通过 `input_json` 发送。

siteid 省略或显式零都表示全部有权站点，非零过滤器不会被扩大。`game_names.id` 与
游戏记录的 id 是响应内字典键，不是 appid。站点 ID 保留 uint64 数值/字符串，时长秒数
保留 uint32。许可类型识别 user/commercial/free，并保留未来字符串；各层未知字段仍可
访问，Debug 对站点名称、IP、hostname 和扩展内容脱敏。

GetTotalPlaytime 选择开始时间在 `[start_time, end_time)` 内的会话，不擅自裁剪相交
会话时长。共享 UTC 校验和比较支持等价后缀及任意小数精度，不使用浮点或原始字符串
直接排序；相等边界表示空区间，时间戳原文不变，不自动拆分或轮询。

Site 文档只列响应字段，没有完整 JSON 外层示例。模型采用规范 `response` 封装及
sites/game_names 数组，外层与省略约定仍待凭据实测；缺失数组作为 schema 错误处理，
不虚构空站点报表。

## 广播帧

[IBroadcastService](https://partner.steamgames.com/doc/webapi/IBroadcastService)
新增 PostGameDataFrame v1，使用 Publisher key 与 `input_json`。frame 是 opaque
UTF-8 字符串，允许空内容或嵌入 NUL；crate 不将其解析为 JSON、不假设 base64、
不施加未公开大小上限、不启动广播或推断交付。POST 遇到 429、503 或超时也仅发送
一次；未公开 schema 的响应保留 opaque 并脱敏。

## 覆盖、验证与分发

```sh
cargo xtask coverage
cargo xtask coverage --json
cargo xtask check-repository
```

报告从清单推导总数及剩余路径，检查公开模块、方法声明及准确路径字面量，并将
Sandbox 继承签名与生产原签名匹配。静态源码映射不是 AST 分析、线上探测或操作成功
证明，协议 fixture 是独立验证层。CI 将 JSON 存为 `steam-api-web-coverage`。

本地验证包含 210 项 workspace 测试，覆盖 P4 路径/编码/key、错误及重放、日期边界、
整数极值、隐私、覆盖检查及既有传输回归。独立复核未发现其他具体契约缺陷。本次未
读取真实财务、站点或广播数据，真实授权及生产行为仍未验证。

Cargo 显式 include 白名单仅保留源码、示例、中英文 README/CHANGELOG、许可证及
Cargo 元数据；测试、基准、xtask、workflow、这些文档及参考 JSON 留在 GitHub。
仓库保持零 Python，沿用每客户端 AWS-LC 后端，没有新增依赖。0.5.0 尚未发布，
创建标签与发布是独立操作。

[0.5 分支 Review](review-0.5_ZH.md)记录共享整数解码重构、回归边界与标量基准。
