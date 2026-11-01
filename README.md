# Project Knowledge

Rust Core 负责索引与查询；基于 Rig 的简单 Agent 负责按 Prompt 调查项目、生成采集插件并推动验证与激活。

设计与职责划分以 [docs/NEW-PROJECT-DESIGN.md](docs/NEW-PROJECT-DESIGN.md)、
[docs/NEW-PROJECT-STRUCTURE.md](docs/NEW-PROJECT-STRUCTURE.md) 为准；
实施顺序、验收与依赖约束见 [plan/NEW-PROJECT-LONG-TASK.md](plan/NEW-PROJECT-LONG-TASK.md)；
依赖选型、版本依据与实测记录见 [docs/decisions/dependencies.md](docs/decisions/dependencies.md)。

当前已能运行基础索引与查询、Node 24 / Python 3.12 插件、锁定依赖准备、扩展类型登记、暂存 extract/link、候选验证、原子激活和按依赖重跑的无模型增量更新。
页面、流程和台账查询、业务主题/Skill、native/SDK 与 stdio MCP 已接入同一 Core。
首版按用户确认交付 Windows x64 可安装候选；其他平台后续，当前未公开发布。安装方式见 [docs/installation.md](docs/installation.md)，验收范围见 [docs/validation/acceptance.md](docs/validation/acceptance.md)。

## 目录

```text
crates/core         扫描、采集宿主、索引、存储、检索、图查询、证据、知识输出
crates/agent        Rig Agent：调查 → 生成 → 验证 → 修复 → 激活
crates/cli          project-knowledge 命令行（默认含 agent feature）
bindings/node       napi-rs 原生绑定（Node-API 8，cdylib + build.rs）
packages/sdk        TypeScript SDK
packages/mcp        本地 stdio MCP（MCP v2 server/client）
scripts             契约类型生成与验收驱动入口（Node .mjs，跨平台）
prompts/            通用 Prompt 模板（shared / investigate / collector / link / repair / knowledge）
skills/             外部 LLM 可复用的工作指南
collectors/         提取脚本 manifest 形态与两种项目布局样例
fixtures/           场景验收数据、协议向量与人工真值
xtask/              导出契约、组织 native 与 CLI 制品
docs/schema         xtask 导出的 JSON Schema（入库，供审查与校验）
docs/decisions      依赖与实现决策记录
docs/evidence       任务执行证据（按任务 ID 分目录）
docs/validation     验收与真值审阅记录
.github/workflows   CI（fmt/clippy/check/test/Core-only/依赖方向）与 Windows 候选构建
```

## 调用关系

```text
Rust CLI ───────────────────────→ Rust Core
Rust CLI → Rust Agent（Rig）─────→ Rust Core 工具
TypeScript SDK → napi-rs binding → Rust Core
MCP → TypeScript SDK ───────────→ Rust Core
```

## 环境

| 项 | 要求 | 备注 |
| --- | --- | --- |
| Rust | 1.97.1（`rust-toolchain.toml` 固定） | rig `=0.45.0` 的发布元数据要求；CI 与本机同版本 |
| C 编译器 | Windows MSVC Build Tools / Linux gcc / macOS CLT | rusqlite bundled、Tree-sitter grammar、aws-lc-sys |
| Node.js | 24+ | SDK / MCP / native 的 `engines` 下限 |
| pnpm | 10.34.8（`packageManager` 固定） | workspace + catalog |

crates.io 需要代理的机器：设 `HTTPS_PROXY` 或写 `~/.cargo/config.toml`，不要提交进仓库。

## 命令

```sh
# Rust
cargo check --workspace --all-targets --locked
cargo test --workspace --locked
cargo build -p project-knowledge-cli --no-default-features --locked   # 不含 Rig 的 Core-only 构建
cargo run -p xtask -- export-contracts                                # Rust 同源 Schema

# Node
pnpm install
pnpm run typecheck
pnpm run build:native            # napi 生成 .node + 加载器 + index.d.ts
pnpm run build:ts                # SDK → MCP
pnpm run build:contracts         # 契约 Schema + TS 类型
pnpm run verify:docs -- --plan plan/NEW-PROJECT-LONG-TASK.md
```

工作区在仓库根目录，因此计划文件路径是 `plan/...`，不是计划原文里的 `../plan/...`。

已实现的基础 CLI：

```sh
project-knowledge init --root .
project-knowledge index --config .project-knowledge/project.json --base
project-knowledge index --config .project-knowledge/project.json --incremental
project-knowledge status --config .project-knowledge/project.json --check-freshness
project-knowledge query --config .project-knowledge/project.json --operation search --text "purchaseLedger"
```

候选插件可使用 `plugin submit / validate / activate / rollback`；正式 `collect` 只运行已激活且哈希匹配的制品。
候选目录、验收 case 与命令示例见 [docs/collectors.md](docs/collectors.md)。

内置模型接入、`.env`、调查生成、有限修复和恢复命令见 [docs/agent-usage.md](docs/agent-usage.md)。

Node native 与 TypeScript SDK 提供十类只读查询和显式维护、请求取消及幂等关闭；见 [docs/sdk-usage.md](docs/sdk-usage.md)。stdio MCP 默认只读，启动时显式选择维护模式；见 [docs/mcp-usage.md](docs/mcp-usage.md)。

## 平台范围

首版只验收 Windows x64。以下 Linux/macOS 条目是后续目标，不表示已经支持；这是 2026-10-29 用户确认的收尾范围（计划 D31）。

| OS/架构 | Rust target | native 后缀 |
| --- | --- | --- |
| Windows x64 | `x86_64-pc-windows-msvc` | `win32-x64-msvc` |
| Linux x64 glibc | `x86_64-unknown-linux-gnu` | `linux-x64-gnu` |
| macOS x64 | `x86_64-apple-darwin` | `darwin-x64` |
| macOS arm64 | `aarch64-apple-darwin` | `darwin-arm64` |

首版不发布 Windows arm64、Linux arm64/musl、浏览器或 WASM；glibc 下限由干净安装实测后写入发布清单。
