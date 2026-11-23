# Project Knowledge

Project Knowledge 在本地为一个或多个代码仓库建立索引。可以搜索源码、SQL 和文档，定位代码符号，沿已采集的关系追踪业务链路，并回到原始文件验证结论。CLI、TypeScript SDK 和 stdio MCP 共用同一个 Rust Core 和 SQLite 索引。

基础索引和查询可以离线运行。页面到 API、审批流程、报表字段血缘等业务查询依赖目标项目的采集插件。可以手动编写插件，也可以配置内置 Agent 协助生成、验证和激活。已激活插件的日常增量更新无需模型。

## 安装

当前已有 **Windows x64** 的隔离安装验收。CLI 可以直接运行解压后的二进制。SDK 和 MCP 需要 Node.js 24+，由同一候选目录的四个 npm tarball 安装。候选文件、校验、安装和升级步骤见 [安装指南](docs/installation.md)。需要从源码生成候选时，见 [构建指南](docs/releasing.md)。

## 上手

解压 CLI 后，在 PowerShell 中把下列工具路径和项目路径替换为实际路径。示例将配置和索引写入目标项目的 `.project-knowledge/`。

```powershell
$cli = 'C:/tools/project-knowledge/project-knowledge.exe'
$project = 'C:/workspace/my-repo'
$config = Join-Path $project '.project-knowledge/project.json'

& $cli init --root $project
& $cli index --config $config --base
& $cli query --config $config --operation search --text 'purchaseLedger'
& $cli status --config $config --check-freshness
```

把 `purchaseLedger` 换成项目里的类名、业务词或路由。初始化后可调整 `project.json` 的仓库和文件范围。代码发生变化时运行：

```powershell
& $cli index --config $config --incremental
```

业务查询返回 `status=partial` 时，查看 `completeness` 和 `gaps`，确认采集覆盖、索引时效或查询预算。配置采集器后，用项目内有代表性的路由、流程和查询核对结果。

## 使用文档

| 要做什么                               | 文档                                                            |
| -------------------------------------- | --------------------------------------------------------------- |
| 在 Codex 等客户端使用 MCP 查询         | [MCP 配置与工具](docs/mcp-usage.md)                             |
| 在 Node.js 应用中查询或维护索引        | [TypeScript SDK](docs/sdk-usage.md)                             |
| 调查目标项目并生成采集插件或项目 Skill | [内置 Agent](docs/agent-usage.md)                               |
| 编写、验证、激活或回退采集插件         | [采集插件](docs/collectors.md)                                  |
| 核对查询、采集、证据及配置格式         | [协议](docs/protocol.md) · [JSON Schema](docs/schema/README.md) |
| 确认已验收场景与支持范围               | [验收范围](docs/validation/acceptance.md)                       |

## 源码开发

使用仓库锁定的 Rust 1.97.1、Node.js 24、pnpm 10.34.8 和平台 C 工具链。Windows 需要 MSVC。运行完整插件验收还需要 Python 3.12。工具链和依赖依据见 [依赖决策](docs/decisions/dependencies.md)。

```powershell
pnpm install --frozen-lockfile --ignore-scripts
cargo build -p project-knowledge-cli --locked
cargo test --workspace --locked -j 1 -- --test-threads=1
pnpm run check:contracts
pnpm run build:native
pnpm run build:ts
pnpm run test:sdk
pnpm run test:mcp
```

`crates/core` 负责扫描、采集、索引、关系查询和证据。CLI 直接使用 Core，并可选接入 `crates/agent`。SDK 通过 `bindings/node` 调用 Core，MCP 通过 SDK 提供查询和显式维护。插件示例在 `collectors/`，场景与测试数据在 `fixtures/`。开发背景和历史验收见 [功能设计](docs/NEW-PROJECT-DESIGN.md) 与 [实施记录](plan/NEW-PROJECT-LONG-TASK.md)。

## 许可证

[MIT](LICENSE-MIT) 或 [Apache-2.0](LICENSE-APACHE)。
