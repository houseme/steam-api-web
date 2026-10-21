<!--
Copyright 2026 steam-api-web contributors.
SPDX-License-Identifier: Apache-2.0
Licensed under the Apache License, Version 2.0.
See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.
-->

# 0.3 Cloud、库存、经济与 Workshop 契约

[English](p2.md) | 简体中文

0.3.0 处于开发阶段，新增 P2 的全部 12 个接口、66 个文档方法，累计 24 个接口、123 个方法。[机器清单](api-inventory.json)记录 P2 对应的 Rust 方法及物品定义接口原样保留的 `v0001` 路径。当前交付是有本地测试的协议封装，真实 Steam 凭据验证和生产运行仍需独立完成。

## Cloud 与远程存储

[ICloudService](https://partner.steamgames.com/doc/webapi/ICloudService)需要用户授权的 `read_cloud` 或 `write_cloud` OAuth scope。使用 Public host 客户端并传入 `cloud::OAuthAccessToken`；API key 不能替代这项权限。Token 位于 `input_json` 外的表单字段，自有秘密以及返回的签名 URL、存储服务 header 的 Debug 均脱敏。

每个 Cloud 方法都要求 `x-eresult: 1`。缺失、格式错误或重复 header 会导致 Decode，非成功结果返回业务错误。仅 Delete、CompleteAppUploadBatch 在检查 header 成功后接受空 HTTP 200；其他方法必须返回规定的类型化结果。这与 SetAppBuildLive 允许空 HTTP 201 的策略不同。

上传由调用方管理以下阶段：

1. BeginAppUploadBatch 声明计划上传和删除的文件，并取得 batch ID。
2. BeginHTTPUpload 返回存储服务地址和 header 指令。
3. 调用方使用独立 HTTP 客户端及指定 header 执行文件 PUT。
4. CommitHTTPUpload 上报真实传输结果；`file_committed=false` 仍表示该阶段失败。
5. 所有操作尝试完成后，包括存在失败时，都必须尝试 CompleteAppUploadBatch。

Steam 客户端不会跟随返回的传输 URL，也不会向其转发 API key 或 OAuth token。库不在 Drop 中异步清理、不承诺最终完成，也不把取得上传计划视为文件已持久化。取消和结果不确定时，需要应用负责对账及结束批次。可选 `is_public` 来自 Valve 示例，其参数表没有列出该字段。

[Cloud 示例](../examples/cloud_files.rs)只读取一页元数据，不打印私有文件名或签名 URL。枚举使用 `start_index`、`total_files`，由调用方控制翻页。[ISteamRemoteStorage](https://partner.steamgames.com/doc/webapi/ISteamRemoteStorage)的七个方法处理 UGC 元数据、合集及订阅，使用索引 ID 与派生 count；未公开 schema 的结果保留原始 JSON。实际文件字节仍由独立传输流程处理。

## 库存与定义管理

[IInventoryService](https://partner.steamgames.com/doc/webapi/IInventoryService)需要具有 Steam Economy 权限的 Publisher key。请求使用 `input_json`，ModifyItems 按明确示例将 appid 放在 JSON 外。整数 ID 不丢精度，ConsumeItem 数量编码为十进制字符串，响应内的 `item_json`、`itemdef_json` 原始文本保持不变。

发放、促销及消耗请求的 request ID 由调用方生成并持久化，库不会代为生成或替换。Steam 对适用操作去重，但携带 ID 不代表所有失败都可以安全重试。任何写入都不自动重放。合成材料按 ID 与正数量配对，实际配方、原子性和所有权仍由 Steam 校验。

ModifyItems 携带调用方的并发时间戳，每个 item/property 只能指定一种类型值或删除。限制为 100 个不同 item，而不是 100 行更新。保留有符号整数及有限 f32 的文档整数/字符串编码。库不知道已有状态，因此过期时间戳和最终动态属性大小必须由 Steam 检查。

AddItem 的可选字段按说明文字处理参数表中不一致的必填标记。GetItemDefs 保留文档声明的标量 uint64 过滤器，不根据复数名称猜测数组。默认值和可省略字段见源码类型。

[IGameInventory](https://partner.steamgames.com/doc/webapi/IGameInventory)使用普通表单参数。UpdateItemDefs 原样保留 `/v0001`，物品定义以 JSON 对象数组传输。其示例使用对象型 `result` envelope，共享校验会检查内部错误，同时保留对标量 EResult 的严格处理。部分 `updated`、`failed` 结果不被隐藏，调用方需要检查；缓存刷新仍是独立显式操作。

## 经济、交易与市场

[ISteamEconomy](https://partner.steamgames.com/doc/webapi/ISteamEconomy)使用 `classid0`、`instanceid0`、`assetid0`、`assetquantity0` 等普通表单字段。不透明 asset/transaction ID 保留原始拼写，货币字符串和数值不经过浮点换算。GET StartTrade 有副作用，只发送一次；发起和完成资产交易的 POST 也不重放。

[IEconService](https://partner.steamgames.com/doc/webapi/IEconService)、[IEconMarketService](https://partner.steamgames.com/doc/webapi/IEconMarketService)使用 `input_json`。交易历史翻页保留上一页时间和完整 trade ID；报价过滤器不增加文档未规定的互斥条件，active/cutoff 与 historical 选择可以重叠。文档未声明 GetTradeOffers cursor，因此不虚构该参数。缓存变更和取消挂牌要求 Publisher 凭据且不重放。

## Workshop 查询、订阅与审核

[发布文件查询](https://partner.steamgames.com/doc/webapi/IPublishedFileService)显式选择页码或 cursor；cursor 不透明，页码范围为 1..=1000。GET Delete 是写操作，不重放。元数据、标签、封禁及不兼容状态更新各自保留文档契约。

[搜索](https://partner.steamgames.com/doc/webapi/ISteamPublishedItemSearch)与[投票](https://partner.steamgames.com/doc/webapi/ISteamPublishedItemVoting)使用旧式索引表单，ResultSetSummary 保留文档声明的 64 位 appid。[审核](https://partner.steamgames.com/doc/webapi/IPublishedFileModerationService)固定决策 revision 为 2，并携带原始 change number，陈旧内容由 Steam 拒绝。

[Workshop 付款及本地化](https://partner.steamgames.com/doc/webapi/IWorkshopService)的部分嵌套 message 未公开字段，接受借用的 JSON 对象或对象数组保留内容，不虚构内部 schema。收入区间保留服务定义的左闭右开范围，文档没有依据将其强制解释为 UTC。接收不透明响应不代表付款规则、描述或审核决定已持久化。

## 验证与分发

每个 P2 方法都有本地 endpoint、HTTP 方法、凭据、编码及错误 fixture，并覆盖重放、完整整数和相关阶段/翻页边界。没有修改真实 Cloud 文件、库存、交易、挂牌或 Workshop 内容；scope 权限和实际结果仍需应用验证。

维护工具继续全部使用 Rust xtask，无需 Python。发布白名单保留库源码、示例、许可及双语 README/changelog；集成测试、基准、工具、工作流、本文及参考 JSON 仅留在 GitHub。归档策略及独立解压构建验证该边界，仓库检查同时对照声明覆盖率、版本、清单和公开 Rust 模块。

2026-10-21 本地验证使用 stable Rust 1.99.1：128 项 workspace 测试、格式化、严格 Clippy、无警告 rustdoc，以及 42 文件归档的独立解压构建通过。远端各平台 CI 结果单独核对。
