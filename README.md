# Project Knowledge（骨架）

Rust Core 负责索引与查询；基于 Rig 的简单 Agent 负责按 Prompt 调查项目、生成采集插件并推动验证与激活。

设计与职责划分以 [docs/NEW-PROJECT-DESIGN.md](docs/NEW-PROJECT-DESIGN.md)、
[docs/NEW-PROJECT-STRUCTURE.md](docs/NEW-PROJECT-STRUCTURE.md) 为准；
实施顺序、验收与依赖约束见 [plan/NEW-PROJECT-LONG-TASK.md](plan/NEW-PROJECT-LONG-TASK.md)；
依赖选型、版本依据与实测记录见 [docs/decisions/dependencies.md](docs/decisions/dependencies.md)。

当前状态：**workspace、依赖与锁文件就绪**；Core 的部分模块正在实现中（`cargo check` 会看到实现层报错，
依赖层已验证完整），未实现的产品命令一律返回非 0 并指明所属任务，不报绿。

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
.github/workflows   CI（fmt/clippy/check/test/Core-only/依赖方向）与四平台发布
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
cargo run -p xtask -- export-contracts                                # 待实现（T02）

# Node
pnpm install
pnpm run typecheck
pnpm run build:native            # napi 生成 .node + 加载器 + index.d.ts
pnpm run build:ts                # SDK → MCP
pnpm run build:contracts         # 契约 Schema + TS 类型（待实现）
pnpm run verify:docs -- --plan plan/NEW-PROJECT-LONG-TASK.md
```

工作区在仓库根目录，因此计划文件路径是 `plan/...`，不是计划原文里的 `../plan/...`。

目标 CLI 形态（尚未实现）：

```sh
project-knowledge init --root .
project-knowledge index --config .project-knowledge/project.json --base
project-knowledge status --config .project-knowledge/project.json --check-freshness
project-knowledge query --config .project-knowledge/project.json --operation search --text "purchaseLedger"
```

## 发布平台矩阵（首版）

| OS/架构 | Rust target | native 后缀 |
| --- | --- | --- |
| Windows x64 | `x86_64-pc-windows-msvc` | `win32-x64-msvc` |
| Linux x64 glibc | `x86_64-unknown-linux-gnu` | `linux-x64-gnu` |
| macOS x64 | `x86_64-apple-darwin` | `darwin-x64` |
| macOS arm64 | `aarch64-apple-darwin` | `darwin-arm64` |

首版不发布 Windows arm64、Linux arm64/musl、浏览器或 WASM；glibc 下限由干净安装实测后写入发布清单。
