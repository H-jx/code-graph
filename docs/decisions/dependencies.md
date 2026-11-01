# 依赖与工具链决策（计划 T00/T01 产出）

核对日期：2026-10-25 → 2026-10-26。来源：crates.io / 包元数据、npmmirror、napi.rs、SQLite 源码、
`plan/NEW-PROJECT-LONG-TASK.md` 第 2.2 节与 `docs/NEW-PROJECT-STRUCTURE.md`。

工作区位置说明：本仓库直接把 workspace 建在 `code-graph/` 根目录（`docs/` 与 `plan/` 与之同级），
未按计划 §1.1 的 `new-project/` 子目录落地；这是本仓库的显式选择，后续引用相对路径时以根目录为准。

## 1. 工具链基线（本机已实测）

| 工具 | 版本 | 状态 |
| --- | --- | --- |
| Rust | 1.97.1（`rust-toolchain.toml` 固定，含 rustfmt / clippy） | 已 `rustup toolchain install 1.97.1` 并实测 `cargo check` |
| Node.js | 24.14.0 | 满足 SDK/MCP/native 的 `engines: >=24` |
| pnpm | 10.34.8（`packageManager` 固定，corepack 解析） | `pnpm install` 实测通过，`pnpm-lock.yaml` lockfileVersion 9.0 |
| C 工具链 | VS 2022 BuildTools（MSVC） | `rusqlite` bundled、Tree-sitter grammar、`aws-lc-sys` 均实测编译通过 |
| Git | 2.33.0 | 当前 HEAD `2d1da7c`，实施成果尚有工作区改动；无远程仓库配置 |
| Python | 本机 uv 3.12.13；CI 3.12.10 | T00/T07 已真实执行；通过配置的绝对路径选择运行时 |

crates.io 在本机需经代理访问：临时用 `HTTPS_PROXY`，或写入用户级 `~/.cargo/config.toml`；
仓库 `.cargo/config.toml` 只放 alias，不含机器相关配置。npmmirror 直连可用。

## 2. Rust 依赖矩阵

版本集中在根 `Cargo.toml` 的 `[workspace.dependencies]`，子 crate 只写 `workspace = true`。

| crate | 版本 | 归属 | 用途 |
| --- | --- | --- | --- |
| serde / serde_json / schemars | 1.0.229 / 1.0.154 / 1.2.3 | core、agent、xtask | 契约、JSON stdin/stdout、Schema 导出 |
| indexmap / strum | 2.14.2 / 0.28.0 | core | 稳定顺序、枚举与字符串互转 |
| thiserror / anyhow | 2.0.21 / 1.0.104 | core / 入口层 | 结构化错误码与入口聚合 |
| async-trait | 0.1.92 | core、agent | 运行时与工具的对象安全接口 |
| tokio | 1.54.1 | core、agent、cli、binding | 异步子进程、超时、取消、并发扫描 |
| tokio-util | 0.7.20 | core | `CancellationToken`（关闭 default features） |
| futures / rayon / parking_lot / arc-swap | 0.3.34 / 1.12.0 / 0.13.0 / 1.9.3 | core | 流式进度、并行哈希、快照与热切换 |
| ignore / globset / regex | 0.4.33 / 0.4.20（serde1）/ 1.13.1 | core | 文件发现、`.gitignore`、选择器与拆词 |
| dirs / home / pathdiff / url | 7.0.0 / 0.5.12 / 0.2.3 / 2.5.8 | core | 配置定位、`~` 展开、仓库相对路径、URL 归一化 |
| rusqlite | 0.41.0，`default-features = false`，`bundled + backup + cache + chrono + serde_json + limits` | core | SQLite；`bundled` 确认打开 `SQLITE_ENABLE_FTS5`（libsqlite3-sys build.rs 实测） |
| jsonschema | 0.60.1，`default-features = false`，`resolve-file` | core | 只允许本地/已登记 Schema，禁止远程 `$ref`；避开 `aws-lc`/网络栈 |
| sha2 / hex | 0.10.9 / 0.4.3 | core、xtask | 内容哈希与制品身份（计划 §2.2 要求 0.10.x；0.11.1 已发布但不跟进） |
| uuid / tempfile / chrono | 1.29.0（v4+serde）/ 3.27.0 / 0.4.45（clock+std+serde） | core、agent | run-id、临时目录、时间戳 |
| petgraph / rustc-hash | 0.8.3 / 2.1.3 | core | 代码地图与关联遍历 |
| handlebars | 6.4.4 | core、agent | Skill/主题模板渲染 |
| tracing / tracing-subscriber | 0.1.44 / 0.3.23（env-filter+json） | core→cli | 结构化日志，订阅器只在入口层初始化 |
| tree-sitter + language crate | 0.27.1 + 0.1.9；typescript 0.23.2 / javascript 0.25.0 / python 0.25.0 | core（feature `syntax`） | 首版只内置 TS/TSX/JS/Python（计划 D18） |
| rig | **`=0.45.0`** + `agent + derive + reqwest + rustls` | agent | 内置 Agent；MSRV 1.97.1（见 §3.1 实测） |
| include_dir | 0.7.4 | agent | `prompts/` 在 crate 外，发布收集由 xtask 负责（见 §5 偏差） |
| clap / clap_complete | 4.6.7（derive+env+cargo+wrap_help）/ 4.6.11 | cli、xtask | 命令与补全 |
| dialoguer / comfy-table / indicatif | 0.12.0（无 default）/ 8.0.1 / 0.18.6 | cli | 交互配置、状态表、进度 |
| napi / napi-derive / napi-build | 3.15.0（`napi8`）/ 3.6.13 / 2.6.0 | bindings/node | Node-API 8；`napi-build::setup()` 在 `build.rs` |
| cargo_metadata / walkdir | 0.23.1 / 2.5.0 | xtask | 制品定位与目录整理 |
| insta / proptest / assert_cmd / predicates | 1.49.0 / 1.12.0 / 2.2.2 / 3.1.4 | dev | 快照、属性与 CLI 集成测试 |

**本版不依赖**：SCIP（`scip` crate）、jieba-rs、tree-sitter-go / -html、tree-sitter-vue（0.0.3，2018 edition，成熟度不足）、
`serde_yaml`（已 deprecated）、ts-rs（Schema 单一来源走 schemars）。Vue 页面结构由项目专属提取脚本处理。

注意区分「依赖图」与「锁文件」：`cargo tree -e normal`（本机 feature 组合）为 403 个节点，不含
`ort / fastembed / lancedb / milvus / helixdb / arrow`；而 `Cargo.lock` 是特性无关的并集，会出现
`jieba-rs`、`lancedb` 这类 rig facade 可选生态的条目（本版本不会被编译）。`cargo deny check` 仍会覆盖它们。

Feature 平面：

```toml
# core：default = ["syntax"]，syntax = [tree-sitter 及三个 grammar]
# cli ：default = ["agent", "syntax"]；agent = ["dep:project-knowledge-agent"]
#       --no-default-features 得到不含 Rig 的 Core-only 构建（计划 T01 要求）
```

## 3. 关键实测结论

### 3.1 Rig 0.45.0 与 MSRV

- `rig` 0.45.0 发布元数据 `rust_version = 1.97.1`；`rig-core` 0.44.0 为 1.95.0。
- 本机原 stable 为 1.95.0。用 `default-features = false + [agent, derive, reqwest, rustls]` 与
  `native-tls` 两套各建探针：在 rustc 1.95 下均报
  `error: rustc 1.95.0 is not supported by the following packages: rig@0.45.0 ...`（退出码 101）；
  切到 `cargo +1.97.1 check` 后依赖图（rustls+aws-lc-sys 与 native-tls+schannel 两条）全部编译通过，
  只剩探针自身调用签名错误 —— 说明依赖可构建，Agent API 细节留给 T00/T18 的最小例确认（计划 U01）。
- 结论：按计划 D12 采用 `rig = "=0.45.0"`，并安装 1.97.1、以 `rust-toolchain.toml` 固定；
  未采用网页示例里的 0.44.0/1.95 组合（那是较低 MSRV 的旧版，与计划要求不符）。

### 3.2 TLS 后端

`reqwest + rustls` 会带入 `aws-lc-rs / aws-lc-sys`（实测在 MSVC 下编译通过，需要可用的 CMake 路径）；
`reqwest + native-tls` 走系统 `schannel`，构建链最轻。当前按计划与 Rig 默认选择 rustls；
若 T23 的干净构建环境缺 CMake，可切换到 `native-tls`，只需改 `[workspace.dependencies]` 的 rig features。
`jsonschema` 已显式关掉 `resolve-http + tls-aws-lc-rs`，避免不必要的网络与原生依赖。

### 3.3 SQLite

`rusqlite/bundled` 触发 `libsqlite3-sys` 编译 sqlite3.c，构建参数含
`SQLITE_ENABLE_FTS5`、`JSON1`、`RTREE`、`STAT4`、`DEFAULT_FOREIGN_KEYS=1`、`THREADSAFE=1`；
`backup` feature 提供 Online Backup API（候选验证用一致快照）。
rusqlite 0.41 的 `default` 是 `cache + ffi-sqlite-wasm-rs`（会拉进 wasm 相关依赖），因此显式关闭 default。

## 4. Node 侧依赖

`pnpm-workspace.yaml` 的 catalog 统一版本；成员：`packages/*`、`bindings/node`、`scripts`。

| 包 | 版本 | 归属 | 用途 |
| --- | --- | --- | --- |
| typescript | ^5.9.3 | 全部 | tsc 与 `--noEmit` 校验 |
| @types/node | ^24.19.2 | 全部 | 与本机 Node 24 对齐（26.x 已发布，本版本不采用） |
| @napi-rs/cli | ^3.10.9 | bindings/node | `build / create-npm-dirs / artifacts / pre-publish`（参数已用 `--help` 核对） |
| publint | ^0.3.25 | sdk、mcp | 发布前 exports 映射检查 |
| json-schema-to-typescript | ^16.0.0 | scripts | `docs/schema/*.json → packages/sdk/src/generated/*.ts` |
| execa | ^10.2.1 | scripts | 调用 cargo / napi |
| prettier | ^3.9.10 | root | 统一格式 |

测试用 **Node 24 内置 test runner**（`node --test test/`，计划 §4.1 的 `test:sdk` / `test:mcp`），不引入 vitest。

MCP 按计划 D13 使用 v2 三包：`@modelcontextprotocol/server` `~2.4.0`（运行时依赖）、
`@modelcontextprotocol/client` `~2.4.0`（T21 验收时启动官方客户端，devDependency）、`zod` `^4.6.7`；
v2 三包传递依赖 `@modelcontextprotocol/core@2.4.0`（Standard Schema 适配层所在）。
**不安装** v1 的 `@modelcontextprotocol/sdk`，两套主版本不混用。

`napi-postinstall` 不在 catalog：平台包的权限/安装细节属于 T23 发布范畴，届时再按需引入并写进 `onlyBuiltDependencies`。

## 5. 与计划的对齐与偏差

| 项 | 计划要求 | 本次落地 | 说明 |
| --- | --- | --- | --- |
| 工作区位置 | §1.1 `new-project/` | 仓库根目录 | 你已确认；`plan/` 与 `docs/` 保持同级 |
| Rig | D12 `rig = "=0.45.0"` + 1.97.1 | 一致（含工具链固定） | §3.1 有实测记录 |
| MCP | D13 server/client 2.4.0 | 一致 | 已移除 v1 sdk |
| 测试执行 | §4.1 Node 内置 test runner | 一致 | 无 vitest |
| 脚本入口 | §4.1 全部目标命令 | `build:contracts`、`check:contracts`、`build:native/sdk/mcp`、`typecheck`、`test:sdk/mcp`、`test:plugins`、`verify:scenarios`、`verify:docs`、`verify:packages`、`check:core-only` 已登记；未实现项一律退出码 2 并打印归属任务 | 计划禁止空脚本报绿 |
| 契约位置 | T02 `docs/schema/` | `docs/schema/` + `scripts/generate-types.mjs` | 根目录不再有 `schema/` |
| 平台矩阵 | §6.2 四平台 | `napi.targets` 与 `release.yml` 均为四平台 | 去掉 arm64 Windows、musl、Linux arm64 |
| sha2 | §2.2 0.10.x | 0.10.9 | 一致 |
| Tree-sitter | D18 TS/JS/Python | 三个 grammar | 一致 |
| Node-API | §2.2 Node-API 8 | `napi/napi8` | 一致 |
| 包名 | §6.1 待核验 | `project-knowledge-*` / `@project-knowledge/*` 均为私有占位 | 公开发布权限在 T23 核验 |
| 偏差 | — | `typescript` 用 5.x（TS 7.0.2 为最新，生态适配中）；`agent` 保留 `include_dir` 但需 xtask 在发布时收集 crate 外 `prompts/` | 记录在案，不声称已解决 |

计划 §2.2 中属于「目标项目插件」而非本工作区的依赖，本文件只登记不安装：
TypeScript Compiler API 6.0.x（明确 `<7`，AST 采集脚本）、Python 3.12.x + SQLGlot 30.24.0（SQL 采集脚本）。

## 6. 实际执行与结果

| 命令 | 结果 |
| --- | --- |
| `rustup toolchain install 1.97.1 -c rustfmt -c clippy` | 安装成功；`rustc 1.97.1` / `cargo 1.97.1` |
| `cargo generate-lockfile`（根 workspace） | 退出码 0；锁定 779 个包；`rig-core 0.44 → rig 0.45.0`、`sha2 0.10.9`、`rusqlite 0.41` 生效 |
| `cargo metadata --locked` | 退出码 0（manifest 与 feature 组合合法） |
| `cargo check --workspace --all-targets`（纯骨架，rustc 1.95 + rig-core 0.44 时期） | 退出码 0；该次运行在依赖矩阵切换到 rig 0.45 / 1.97.1 之前，未在纯骨架状态下复跑 |
| rig TLS 探针（独立最小 crate，`cargo +1.97.1 check`） | rustls/aws-lc 与 native-tls/schannel 两条依赖链的全部依赖编译通过（含 `aws-lc-sys`），只剩探针自身的调用签名错误 |
| `pnpm install`（pnpm 10.34.8） | 116 个包，退出码 0，生成 `pnpm-lock.yaml` |
| `pnpm run typecheck` | 退出码 0 |
| `pnpm run build:contracts` / `verify:*` / `test:sdk` / `test:mcp` | 退出码 2，输出归属任务（预期，不允许报绿） |
| `cargo tree -p project-knowledge-cli --no-default-features -e normal` | 493 个节点，`rig` 出现 0 次 —— Core-only 构建确实不含模型框架（计划 T01 门禁） |
| `cargo fmt --all -- --check` | 骨架文件（本次创建的 32 个 `.rs` 占位）已全部通过；当前树另有 136 处 diff，集中在实现进行中的 `crates/cli/src/commands/*`、`crates/core/src/contract/identity.rs` 等文件，属实现层收尾，不在本次改动范围内 |
| `cargo check --workspace --all-targets`（当前树，1.97.1 + rig 0.45） | 退出码 101，15 条错误全部位于 `index/build.rs`、`index/incremental.rs`、`search/*`、`storage/search.rs` 的实现代码（缺方法/类型不符），**无一条是依赖缺失、feature 非法或版本冲突**；core 失败使 agent 与 binding 两个 crate 本轮未被检查 |

仍未验证（不可当作通过，见计划 §2.3）：U01 真实 Rig Agent API 与工具循环、U02 napi v3 加载与 MCP stdio 实测探针、
U03 Windows 子进程树/文件发布恢复、U04 真实项目约定与独立真值、历史审查资料 `docs/ARCHITECTURE-REVIEW.md` 仍缺失。

## 7. 更新依赖的流程

```sh
cargo update -p rusqlite          # 逐个更新，避免一次性踩 MSRV 与 feature 变更
cargo deny check advisories       # deny.toml 已就绪，需 cargo install cargo-deny
pnpm update --latest --recursive typescript
pnpm install --frozen-lockfile    # CI 与干净构建
```

`Cargo.lock` 与 `pnpm-lock.yaml` 都提交入库；改 `rust-toolchain.toml` 或 rig 精确版本时，
必须同步本文件与相关任务记录。

## 8. 2026-10-26 实测更新

此前探针与 Python 不可用结论已更新：uv 中 Python 3.12.13 可执行；T00 的 napi v3 实际加载、MCP 2.4.0 两协议 stdio、Rig 0.45.0 工具循环/错误/取消/max_turns 均通过。详见 docs/evidence/T00/probe-report.json，真实模型调用 0。TypeScript Compiler API 探针固定 6.0.3，根构建工具 5.9.3 保留；新增独立 npm 锁文件执行真实 npm ci。

T01 已修正独立 CI，所有本地门禁通过；远程 Actions 未触发。CI Python 固定官方三平台可下载的 3.12.10；本地 3.12.13。T02 使用 schemars 1.2.3 → JSON Schema 2020-12 → json-schema-to-typescript 16.0.0，并以编译负例验证完整 DTO 导出、optional/null/union/额外字段；运行时仍由 Core 校验正则与整数等 TS 不能表达的条件。

## 9. T11 示例 AST 依赖（2026-10-28）

示例插件继续使用 T00 实测的 TypeScript 6.0.3，另固定 `graphql=16.11.0`（MIT；只解析文档 AST，不执行查询）。官方 [GraphQL language API](https://www.graphql-js.org/api-v16/language/) 与 npm registry 该精确版本元数据已核对；完整性 `sha512-mS1lbMsxgQj6hge1XZ6p7GPhbrtFwUFYi3wRzXAC/FmYnyXMTvvI3td3rjmQ2u8ewXueaSvRPWaEcgVVOT9Jnw==` 写入两份 npm lock。GraphQL 嵌入文本有转义时只引用可证明的完整源码调用范围。

依赖仅在独立制品 deps/package.json/lock 中，由既有 Core prepare 机制准备到制品哈希缓存；不加到 SDK/Core 的业务依赖。安装与采集分离、忽略生命周期脚本和采集无网络安装规则不变。

## 10. T12 数据库示例 AST 依赖（2026-10-28）

独立 Python 插件固定 SQLGlot 30.24.0（MIT）、tree-sitter 0.25.2（MIT）、tree-sitter-typescript 0.23.2（MIT）。精确版本、官方 PyPI wheel URL/SHA-256 与 requiresDist 保存于 `docs/evidence/T12/database/dependency-metadata.json`，两制品生成完整 hash-lock；只安装 binary wheels，无额外 extras 或浮动版本。SQLGlot 使用 `ErrorLevel.RAISE`，不以字符串代替枚举；本轮反例已检出字符串参数未正确触发严格错误的情况。

核对来源：[SQLGlot 30.24.0 官方元数据](https://pypi.org/pypi/sqlglot/30.24.0/json)、[SQLGlot DDL AST](https://sqlglot.com/sqlglot/expressions/ddl.html)、[Python Tree-sitter 官方仓库](https://github.com/tree-sitter/py-tree-sitter)、[binding 精确版本](https://pypi.org/pypi/tree-sitter/0.25.2/json)、[TypeScript grammar 精确版本](https://pypi.org/pypi/tree-sitter-typescript/0.23.2/json)。实际调用已在 Windows Python 3.12.13 验证；未从 wheel 列表外推其他平台已经验收。

TS/Python 源码 AST 只承担已调查的 connect/model/query/findAll 词法结构；SQLGlot 只解析静态 SQL，不使用 optimizer/scope 或执行目标 SQL，不导入或执行业务源码。Core/SDK/native 不新增项目专属 ORM 逻辑。SQLite 方言支持由插件约定显式登记，未知/未验方言保持 gap。
