<!--
Copyright 2026 steam-api-web contributors.
SPDX-License-Identifier: Apache-2.0
Licensed under the Apache License, Version 2.0.
See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.
-->

# CI 与发布流程

[English](releases.md) | 简体中文

工作流参考 [rustfs-mimalloc CI](https://github.com/houseme/rustfs-mimalloc/blob/main/.github/workflows/ci.yml) 和其[发布流程](https://github.com/houseme/rustfs-mimalloc/blob/main/.github/workflows/release.yml)，适配为单个 HTTP 客户端 crate。分配器专用 feature、原生 profiling 和多 crate 发布顺序改为本项目实际需要的验证。

## CI

CI 支持 main push、PR、手动执行和可复用 workflow 调用。发布流程传入固定源码 revision，每个 checkout 使用该 revision，并关闭凭据保留。

- Linux、macOS、Windows stable 编译所有 target，并执行 Rust 测试、文档测试和独立 provider 测试进程。
- lint job 运行 fmt、严格 Clippy、双语/链接/版权/清单检查、AWS-LC 依赖图检查、离线 Python 发布保护测试和 actionlint。
- 文档 job 拒绝 rustdoc 警告。
- 打包等待全部门禁，运行包含独立构建验证的 cargo package，将 crate 和 SHA256SUMS 上传为 steam-api-web-package artifact。

已有安全审计与 SARIF 工作流独立保留。开发 CI 会取消被新提交取代的任务；标签和手动发布验证避免自动取消。提交这些定义不代表 crates.io 已有包或远端 CI 已通过。

## 准备发布

1. 在 Cargo.toml 设置 SemVer；依赖变更需要时刷新 Cargo.lock。
2. CHANGELOG.md 与 CHANGELOG_ZH.md 都添加相同版本的非空条目。发布日期明确后，将准备条目的 Unreleased 日期更新为正式日期。
3. 更新两份 README 和成对文档；代码注释使用英文。刷新生成元数据时保留版权与 SPDX 头。
4. 运行本地检查并查看包内容，真正发布前确认固定 revision 的 CI 已通过。

```sh
python3 scripts/release.py check --version 0.1.0
python3 scripts/check_repository.py
python3 -m unittest discover -s scripts/tests -v
cargo fmt --all --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --all-features --locked
cargo package --locked
```

本地工作区未提交时可以用 --allow-dirty 查看打包结果；CI 发布必须干净检出，不使用该参数。

## 手动 dry-run

打开 Release 工作流，选择源码分支，输入 manifest 版本，保留 dry_run 为 true（默认值）。dry-run 检查双语 changelog、完整可复用 CI 和归档，不需要 registry token，也不会发布或创建 GitHub release。可下载 artifact 审阅 crate 和校验和。

## 真正发布

仓库需配置名为 CARGO_REGISTRY_TOKEN、具备 steam-api-web 发布权限的 secret。GitHub release 使用该 job 的 GITHUB_TOKEN，仅该 job 具备 contents: write；其他 job 默认为 contents: read。本次配置不会创建凭据或触发发布。

审阅固定提交后推送不可变 v<VERSION> 标签，或手动关闭 dry_run 并指定已存在的匹配标签。以下命令只应在确实决定发布时执行：

```sh
git tag v0.1.0
git push origin v0.1.0
```

标签 push 从标签解析版本；手动真正发布则检出已存在的目标标签。准备阶段校验严格 SemVer、Cargo 包版本/许可/edition、双语 changelog 和 HEAD 与标签提交完全一致，拒绝前导零及命令式输入。支持 rc/beta，并标记为 prerelease。

可复用 CI 通过后，发布 job 下载归档并核对保存的 SHA-256，再从同一 revision 重新 cargo package，要求与已审阅 artifact 逐字节相同。随后从干净 revision 执行 cargo publish。上传返回失败时，不能仅凭版本存在就判成功：脚本轮询 crates.io 并比对确切版本和归档 SHA-256，拒绝不同内容或 yanked 版本。只有 registry 验证通过，才从英文 changelog 对应版本生成 GitHub release，并附加已验证 crate 和校验和。

发布任务串行执行且不自动取消。恢复中断发布时，已存在包必须与新验证归档相同。crates.io 发布不可逆，工作流实现及离线测试没有执行真实上传。

## 文件版权头与机器数据

Rust、Python、TOML、YAML、Markdown、ignore/lock 文件及 LICENSE 均包含英文版权和 Apache-2.0 声明。Markdown 用 HTML 注释，Python 保留解释器 shebang；Cargo.lock 可以在版权声明前保留 Cargo 自动生成的说明。JSON 不支持注释，因此第一个 _metadata 对象保存版权及许可字段。api-inventory.json 与 dependency-versions.json 是共享的语言无关证据；所有说明文档均有中英文版本。
