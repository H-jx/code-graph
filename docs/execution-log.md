# 执行记录

本文件按 `plan/NEW-PROJECT-LONG-TASK.md` 第 7 节的模板记录每次实施。状态变更同步写回
计划文档第 4 节的对应任务。证据默认保存在 `docs/evidence/<任务ID>/`。

---

### 2026-10-26 — T00/T01/T02/T03/T04/T05 — opencode / 无模型

- 状态：todo → doing（T00、T01、T02）→ done（T03、T04、T05）
- 前置依赖：无（T00 起）
- 工作目录/源码版本：`E:\Project\my-project\code-graph`（Git 仓库尚无提交，全部文件为未跟踪状态；
  `git status --short` 全部显示 `??`）
- 输入：计划 §3 的契约、§4 的任务、§5 的场景矩阵；仓库内已有的 `docs/NEW-PROJECT-DESIGN.md`
  与 `docs/NEW-PROJECT-STRUCTURE.md`（两份必读设计已完整阅读，659 行结构资料按 §2 决策具体化）
- 本次具体能力：
  - 统一契约层（`crates/core/src/contract/`）：实体、关系、证据、采集、查询、错误与稳定身份；
    全部 JSON 字段 camelCase、未知字段拒绝、协议版本校验、稳定 ID 纯函数与类型注册表。
  - 项目接入（`crates/core/src/project/`）：多仓库配置解析与校验、根重叠拒绝、身份增量 CAS、
    显式 ignore 行为、符号链接/超限/二进制/非 UTF-8/大小写冲突缺口、来源快照与集合摘要。
  - 存储层（`crates/core/src/storage/` + `crates/core/migrations/0001_initial.sql`）：
    §3.4 的逻辑表、外键、CHECK、触发器与索引；只读/维护连接、事务 A/B/C 基础 API、
    批次替换与单向失效。
  - 基础索引（`crates/core/src/index/`）：扫描 → 快照 → Tree-sitter 符号与保守引用 →
    实体/关系/证据 → FTS5 搜索投影 → 原子提交；未采集能力登记 `not_configured`。
  - 查询层（`crates/core/src/query/`）：统一 `QueryRequest` tagged union、status/search/
    readSource/trace/codeMap 与三个场景组合、统一响应外壳与预算。
  - CLI（`crates/cli/src/`）：`init` / `index` / `status` / `query` 已实现；`collect` / `plugin` /
    `agent` / `generate` 明确返回 `NOT_IMPLEMENTED`，不返回假成功。
  - 契约导出与类型生成：`xtask export-contracts`（含 `--check`）、`scripts/generate-types.mjs`
    （含 `--check`）、`scripts/verify-vectors.mjs`、`scripts/verify-plan.mjs`。
- 修改文件：
  - `crates/core/src/contract/{entity,relation,evidence,collector,query,error,identity,mod}.rs`（新建/重写）
  - `crates/core/src/project/{config,scan,snapshot,mod}.rs`（新建/重写）
  - `crates/core/src/storage/{database,sources,entities,relations,runs,plugins,search,mod}.rs`（新建）
  - `crates/core/src/index/{build,dependencies,incremental,mod}.rs`（新建）
  - `crates/core/src/query/{mod,feature,page,workflow,report}.rs`（新建）
  - `crates/core/src/search/{documents,rank,tokenize,mod}.rs`（新建）
  - `crates/core/src/syntax/{parser,symbols,mod}.rs`（新建）
  - `crates/core/src/evidence/{read,position,mod}.rs`（新建）
  - `crates/core/migrations/0001_initial.sql`（新建）
  - `crates/core/tests/indexing.rs`（39 条用例）
  - `crates/cli/src/main.rs`、`crates/cli/src/commands/*.rs`（重写）
  - `xtask/src/main.rs`（重写）
  - `scripts/{identity,verify-vectors,generate-types,verify-plan}.mjs`（新建）
  - `fixtures/protocol/identity-vectors.json`（新建）
  - `Cargo.toml`（新增 `result_large_err = "allow"` 与理由）
  - `plan/NEW-PROJECT-LONG-TASK.md`（同步 T00—T05 状态）
- 决策变更：
  - **D01（补充）**：计划要求新源码落在 `new-project/`；本次实施直接在仓库根进行，
    根目录已是独立 Rust/Node 工作区（`Cargo.toml` / `pnpm-workspace.yaml` / `rust-toolchain.toml`），
    移动会打乱锁文件与 workspace 引用，因此把仓库根视为 `new-project/`。该偏离已在本文件与
    计划状态行中标注。
  - **D03（落实）**：`dataTable` / `dataColumn` / `dataKey` 的 `catalog`、`schema` 定义为
    **可选**身份槽位，用 `[]` / `[value]` 编码，使“缺省”与“空字符串”不再混淆（MySQL 无 catalog）。
  - **D10（落实）**：中文检索改为「FTS5 负责召回 + Rust 负责精确度」：Rust 生成单字/双字 token
    与标识符拆词写入 FTS5，`search/rank.rs` 计算 `exactRank` 并给出命中理由；不把 trigram
    宣称为通用中文分词器。
  - **D18（落实）**：Tree-sitter 只提供 TS/TSX/JS/Python 的声明与保守引用；跨文件调用不连线，
    同文件调用才生成 `confirmed` 关系。
  - **§3.2 澄清**：仓库根允许 `..`（设计 §6.1 的 `root: ".."` 指向仓库根），仓库内部的相对路径
    仍禁止 `..`、绝对路径与 NUL。
- 命令与结果：

  | 命令（含工作目录） | 工具版本 | 退出码 | 匹配/通过断言数 | 日志路径 |
  | --- | --- | --- | --- | --- |
  | `cargo test -p project-knowledge-core --test indexing`（仓库根） | cargo 1.97.1 | 0 | 39 passed / 0 failed | `docs/evidence/T05/base-tests.log`、`docs/evidence/T04/storage-tests.log`、`docs/evidence/T02/contract-tests.log` |
  | `cargo clippy --workspace --all-targets` | cargo 1.97.1 | 0 | 0 warning | 本文件「质量门禁」小节 |
  | `cargo fmt --all -- --check` | cargo 1.97.1 | 0 | 0 diff | 本文件「质量门禁」小节 |
  | `cargo run -p xtask -- export-contracts` | cargo 1.97.1 | 0 | 5 个契约文件 | `docs/evidence/T02/` |
  | `cargo run -p xtask -- export-contracts --check` | cargo 1.97.1 | 0 | 5/5 一致 | 同上 |
  | `node scripts/generate-types.mjs` / `--check` | Node 24.14.0 | 0 | 生成 `packages/sdk/src/generated/contract.ts` | 同上 |
  | `node scripts/verify-vectors.mjs` | Node 24.14.0 | 0 | 18/18 向量一致 | `docs/evidence/T02/identity-vectors-node-report.json` |
  | `node scripts/verify-plan.mjs --plan ../plan/NEW-PROJECT-LONG-TASK.md` | Node 24.14.0 | 0 | 24 个任务、依赖无环、需求映射与脚本登记一致 | 本文件「质量门禁」小节 |
  | `pnpm run typecheck` | pnpm 10.34.8 / Node 24.14.0 | 0 | SDK 与 MCP 均无类型错误 | 本文件「质量门禁」小节 |
  | CLI 端到端 `init → index --base → status → search → query --request → collect → index --incremental` | project-knowledge 0.1.0 | 见下 | 7 步 | `docs/evidence/T05/e2e/commands.log` |

- 输出：
  - 契约 Schema：`docs/schema/contract.schema.json`（2020-12），SHA-256 见
    `docs/evidence/T02/schema-hash.txt`。
  - 共享身份向量：`fixtures/protocol/identity-vectors.json`（18 条）。
  - CLI 端到端产物：临时工作区 `.project-knowledge/{project.json,index.sqlite}`，
    制品哈希见 `docs/evidence/T05/e2e/artifact-hashes.txt`。
- 正常与异常验证：
  - 正常：CLI 七步全部按预期退出（见 `commands.log`）；`status --check-freshness` 返回
    `fresh`；`search "采购台账"` 命中 3 条（title 符号为 substring 命中）。
  - 异常：`collect` 明确返回 `NOT_IMPLEMENTED` 且退出码 1（不返回假成功）；来源被修改后
    `status` 从 `fresh` 变为 `stale`；缺失数据库只读打开返回 `INDEX_MISSING`；配置根重叠、
    身份增量 CAS 失败分别返回 `CONFIG_INVALID` 与 `ACTIVATION_CONFLICT`。
- 新鲜度/覆盖：base = fresh（端到端工作区）；业务能力 routes/frontend/api/database/workflow/report
  全部 `not_configured`；知识层未采集。
- 独立真值：本轮**没有**独立业务真值（T06 未开始）。`fixtures/protocol/identity-vectors.json`
  是协议层向量，由 Rust 与 Node 两种独立实现互相校验，不属于业务准确率证据。
- 失败/中断/回滚：无中断。实施中发生两次编码事故（PowerShell 往返把 UTF-8 转成 GBK 破坏
  `identity.rs` 与 `config.rs`），已用脚本还原并逐字重写该两文件，随后对全仓库 130 个源码/文档
  文件做 U+FFFD 扫描，结果为 0。
- 预算：模型请求 0；工具调用 0；修复次数 0；普通采集的模型调用 0。
- 未验证项或阻塞：
  - 本机没有可用的 Python 3.12（`python` 是 WindowsApps 占位），Python 侧向量与 Python 插件
    运行时无法执行；T02 因此保持 `doing`，T07 的 Python 模板只能先写后验。
  - napi v3 实际加载与 MCP 2.4.0 stdio 握手尚未做最小例（T00 未完成部分）。
  - `collect` / `plugin` / `agent` / `generate` 尚未实现，四个命令明确失败。
  - 新项目独立 CI workflow 尚未创建（T01 未完成部分）。
- 下一可执行任务：T07（manifest、运行时、锁定依赖与有界子进程），
  第一条命令：`cargo test -p project-knowledge-core --test collectors runner_`。
- 完成证据：
  - `docs/evidence/T00/tools.txt`
  - `docs/evidence/T01/cli-help.txt`、`docs/evidence/T01/cli-version.txt`
  - `docs/evidence/T02/{contract.schema.json,entity-kinds.json,relation-kinds.json,error-codes.json,protocol.json,identity-vectors.json,identity-vectors-node-report.json,schema-hash.txt,contract-tests.log}`
  - `docs/evidence/T03/`（扫描与配置用例见 `docs/evidence/T05/base-tests.log` 的 `project_` 组）
  - `docs/evidence/T04/{storage-tests.log,schema-ddl.txt}`
  - `docs/evidence/T05/{base-tests.log,e2e/*}`

---

### 2026-10-26 — T07 — opencode / 无模型

- 状态：todo → doing
- 前置依赖：T05（证据 `docs/evidence/T05/base-tests.log`）
- 工作目录/源码版本：`E:\Project\my-project\code-graph`（Git 仓库仍无提交）
- 输入：计划 §3.5 的 manifest/协议/依赖/限额契约；`collectors/templates/` 的 Node 与 Python 模板骨架
- 本次具体能力：
  - `crates/core/src/collect/manifest.rs`：manifest 校验、阶段 DAG 校验、制品哈希。
  - `crates/core/src/collect/runner.rs`：有界子进程宿主（stdin/stdout 协议、输出与 stderr 上限、
    并发 2、超时、取消、env 白名单、任务 cwd）。
  - `crates/core/src/collect/process.rs`：Windows Job Object / Unix 进程组的进程树终止。
  - `crates/core/src/collect/deps.rs`：依赖准备（`npm ci --ignore-scripts`、
    `pip install --require-hashes --only-binary=:all:`、缓存摘要与完整性校验）。
  - `crates/core/src/collect/import.rs`：阶段输入导出（来源 + 内容 + 选择集合摘要 + 已导出实体断言）。
  - `crates/core/src/collect/validate.rs`：采集输出校验（协议、ID 重算、端点约束、证据区间与行号、
    依赖与缺口声明）。
  - `crates/cli/src/commands/collect.rs`：`collect --phase extract|link [--plugin] [--validate-only]`
    发现候选、运行阶段并校验输出（提交事实留给 T08/T09）。
- 修改文件：
  - `crates/core/src/collect/{manifest,runner,process,deps,import,validate,mod}.rs`（新建/重写）
  - `crates/core/tests/collectors.rs`（14 条用例，新建）
  - `crates/cli/src/commands/collect.rs`（重写）
  - `collectors/templates/node/{manifest.json,src/collect.mjs,faults/*}`、
    `collectors/templates/python/{manifest.json,src/collect.py,faults/*}`（新建）
  - `Cargo.toml`（新增 `libc`、`windows-sys`；`unsafe_code` 由 `forbid` 改为 `deny`，
    仅 Windows Job Object 的 FFI 封装显式放行）
  - `crates/core/Cargo.toml`、`crates/cli/Cargo.toml`（新增目标平台与 CLI 依赖）
- 决策变更：
  - **§3.5 澄清**：依赖准备失败时**不执行插件**；缓存目录与制品目录分离；`npm ci` 禁用生命周期脚本；
    Python 只允许带哈希的 wheel，缺少 wheel 直接报错。
  - **U03（部分）**：Windows 侧改用 Job Object（`KILL_ON_JOB_CLOSE`）而不是 `kill_on_drop`，
    以满足“`kill_on_drop` 不独自证明后代已终止”的要求；Unix 侧使用独立进程组 + `killpg`。
- 命令与结果：

  | 命令（含工作目录） | 工具版本 | 退出码 | 匹配/通过断言数 | 日志路径 |
  | --- | --- | --- | --- | --- |
  | `cargo test -p project-knowledge-core --test collectors`（仓库根） | cargo 1.97.1 | 0 | 14 passed / 0 failed | `docs/evidence/T07/collector-tests.log` |
  | `cargo test --workspace --locked` | cargo 1.97.1 | 0 | 14 + 40 passed / 0 failed | 本文件「质量门禁」小节 |
  | `cargo clippy --workspace --all-targets` | cargo 1.97.1 | 0 | 0 warning | 同上 |
  | CLI `init → index --base → collect --phase extract --validate-only` | project-knowledge 0.1.0 | 0 | 5 步 | `docs/evidence/T07/e2e/commands.log` |
  | CLI `collect --plugin not.exists` | project-knowledge 0.1.0 | 1 | 明确失败 | `docs/evidence/T07/e2e/05-collect-unknown-plugin.json` |

- 输出：候选插件制品哈希 `41b9b59d…`（同一制品重复运行得到同一哈希）；
  阶段输出 272 字节 stdout / 67 字节 stderr，校验 0 issue。
- 正常与异常验证：
  - 正常：Node 模板被真实执行，输出 `CollectorOutput` 且通过全部校验。
  - 异常：非零退出 → `PLUGIN_EXIT`；非法 JSON → `SCHEMA_INVALID`；输出洪泛 → `OUTPUT_LIMIT`；
    超时 → `PLUGIN_TIMEOUT`（1.5 秒限额内返回）；取消 → `CANCELLED`；超限输入 → `OUTPUT_LIMIT`；
    manifest 越界入口/绝对路径/未配置 runtime → `CONFIG_INVALID`；DAG 自依赖与 extract→link → `CONFIG_INVALID`。
- 新鲜度/覆盖：基础层 fresh；业务能力仍未采集。
- 独立真值：不适用（本任务是协议与宿主行为，无业务真值）。
- 失败/中断/回滚：无中断。实施中修复了两个真实缺陷：子进程命令漏传入口脚本（导致 Node 把 stdin 当脚本执行），
  以及 `WalkBuilder::standard_filters(false)` 调用顺序覆盖了显式开启的 `.gitignore` 规则。
- 预算：模型请求 0；工具调用 0；普通采集的模型调用 0。
- 未验证项或阻塞：见计划 T07 状态行。
- 下一可执行任务：T08（事实校验、临时 extract/link 与范围替换），
  第一条命令：`cargo test -p project-knowledge-core --test collectors validation_`。
- 完成证据：`docs/evidence/T07/`

---

### 2026-10-26 — T08（部分） — opencode / 无模型

- 状态：todo → doing
- 前置依赖：T07（证据 `docs/evidence/T07/collector-tests.log`）、T04（`docs/evidence/T04/storage-tests.log`）
- 工作目录/源码版本：`E:\Project\my-project\code-graph`（Git 仓库仍无提交）
- 输入：计划 §3.5 的校验规则与 §3.6 的结果事务 B
- 本次具体能力：
  - `crates/core/src/collect/validate.rs`：采集输出校验（协议、ID 重算、端点约束、证据区间与行号、
    依赖/缺口/覆盖声明），错误信封按第一个问题的类别归类并附精确 `instancePath`。
  - `crates/core/src/collect/commit.rs`：结果事务 B（按替换范围全量替换断言、证据引用、依赖、
    coverage、搜索投影与 `current_batches`；旧批次单向失效并记录失效 generation）。
  - `crates/core/src/collect/import.rs`：阶段输入导出（来源 + 内容 + 选择集合摘要 + 已导出实体断言）。
  - 存储层 UPSERT 修正：`INSERT OR IGNORE` / `INSERT OR REPLACE` 会吞掉外键违规，
    改为带显式冲突目标的 UPSERT。
- 修改文件：
  - `crates/core/src/collect/{validate,commit,import,mod}.rs`
  - `crates/core/src/contract/collector.rs`（`DependencyKind::as_str`）
  - `crates/core/src/storage/{entities,relations,runs,sources}.rs`（UPSERT 修正）
  - `crates/core/tests/collectors.rs`（+8 条 `validation_` 用例）、`crates/core/tests/indexing.rs`（+3 条 `import_` 用例）
- 决策变更：
  - **存储写入语义**：所有 `OR IGNORE` 改为显式冲突目标的 UPSERT。理由：`OR IGNORE` 同时忽略外键违规，
    会让作用域不存在的实体被静默丢弃，与“坏 FK 不允许绕过 Core 直接入库”冲突。
- 命令与结果：

  | 命令（含工作目录） | 工具版本 | 退出码 | 匹配/通过断言数 | 日志路径 |
  | --- | --- | --- | --- | --- |
  | `cargo test -p project-knowledge-core --test collectors validation_` | cargo 1.97.1 | 0 | 8 passed / 0 failed | `docs/evidence/T08/validation-tests.log` |
  | `cargo test -p project-knowledge-core --test indexing import_` | cargo 1.97.1 | 0 | 3 passed / 0 failed | `docs/evidence/T08/import-tests.log` |
  | `cargo test --workspace --locked` | cargo 1.97.1 | 0 | 64 passed / 0 failed | `docs/evidence/verification/03-test.log` |

- 输出：校验错误信封带精确 `instancePath`（如 `/evidence/0`、`/entities/0`）与修复提示。
- 正常与异常验证：
  - 正常：合法采集输出通过全部校验（0 issue）。
  - 异常：越界证据 → `EVIDENCE_INVALID`；行号不符 → `EVIDENCE_INVALID`；缺少证据 → `EVIDENCE_INVALID`；
    ID 与重算不符 → `IDENTITY_CONFLICT`；悬空端点 → `REFERENCE_MISSING`；生成知识当证据 → `EVIDENCE_INVALID`；
    未知协议版本 → `PROTOCOL_UNSUPPORTED`。
  - 批次替换：同替换范围的新批次提交后旧事实撤下、旧批次 stale 并记录失效 generation；
    替换一个生产者不影响另一个；失败事务整体回滚且当前批次指针不变。
- 新鲜度/覆盖：基础层 fresh；业务能力仍未采集。
- 独立真值：不适用。
- 失败/中断/回滚：无中断。
- 预算：模型请求 0；工具调用 0。
- 未验证项或阻塞：见计划 T08 状态行。
- 下一可执行任务：T08 剩余部分（staging extract→link 链路与 CLI 提交接线），随后 T09。
- 完成证据：`docs/evidence/T08/`

---

## 质量门禁（本轮实际执行）

命令与日志保存在 `docs/evidence/verification/`。

| 命令 | 退出码 | 结果 | 日志 |
| --- | --- | --- | --- |
| `cargo fmt --all -- --check` | 0 | 无差异 | `01-fmt.log` |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | 无告警 | `02-clippy.log` |
| `cargo test --workspace --locked` | 0 | 54 passed / 0 failed（collectors 14 + indexing 40） | `03-test.log` |
| `node scripts/verify-vectors.mjs` | 0 | 18/18 向量一致 | `04-vectors.json` |
| `node scripts/verify-plan.mjs --plan ../plan/NEW-PROJECT-LONG-TASK.md` | 0 | 依赖无环、需求映射完整、目标命令已登记 | `05-verify-docs.log` |
| `cargo run -p xtask -- export-contracts --check` | 0 | 5/5 契约文件一致 | `06-contracts.log` |
| `node scripts/generate-types.mjs --check` | 0 | 生成类型一致 | `07-types.log` |
| `pnpm run typecheck` | 0 | SDK / MCP 无类型错误 | `08-typecheck.log` |
| `cargo build -p project-knowledge-cli --no-default-features --locked` | 0 | Core-only 构建通过；`cargo tree -p project-knowledge-core` 中无 rig | 本文件「质量门禁」小节 |
