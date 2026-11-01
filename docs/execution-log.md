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

---

### 2026-10-26 — T08 续实施 → T09 — Codex / 无模型

- 状态：T08 doing（补齐本轮 Node 子闭环，保留未满足门禁）；T09 todo → doing。
- 前置与执行范围：先核对计划、最近记录、T07/T08 证据和源码。承接用户明确指定的 T08 剩余与随后 T09，复用已有事务 B、扫描和存储；T00/T01/T02/T07 未完成门禁没有被改写为 done。T10 未启动验收。
- 工作目录/版本：`E:/Project/my-project/code-graph`；Git 尚无提交版本，源码哈希见 `docs/evidence/T08/continuation-2026-10-26/source-hashes.json`。
- 已实现的可观察闭环：
  - Core backup staging → 移除目标旧指针 → 稳定 DAG extract → 再导出完整候选/断言 → link → 重放 acceptance case → 短事务 B；坏阶段不提交任何本组事实，空输出确实替换。
  - 来源集合按仓库合并，摘要绑定内容与选择状态；候选集合包括完整断言与零匹配；自动补齐并复核 config/artifact/runtime/source/sourceSet/candidateSet/upstreamBatch 依赖。
  - 校验补齐证据/关系 ID 重算、UTF-8 字符边界、scope 一致性、必需数组、已定义业务载荷 Schema；非法 fixture 的 Serde/jsonschema 拒绝及精确 code/JSON Pointer 一致。
  - CLI validate-only 不改正式 generation；正式 collect 只接受已激活且哈希相同的制品，可整组或显式分阶段提交。
  - 候选 generated/prepared/validated/published/active/failed/superseded 持久化；多个新候选共享验证、整组成功或整组失败；自动发现去重与 autoLoad/autoActivate 分离。
  - 制品在同父目录临时写入、sync、rename 到 hash 目录；单项目 OS 锁、二次来源/配置/运行时/活动映射核对、C 事务 CAS 一次切版本和事实；纳入受影响活动阶段闭包。
  - 回退按当前输入重跑并生成新批次；确认输入变化先持久失效，内容改回、失败候选、重启均不复活旧批次。活动制品篡改使相关批次和下游 stale。
  - 五处错误注入及实际发布 I/O 错误验证回滚；Windows 实际终止验证宿主，独立断言 Node 插件与孙进程退出、OS 锁释放、正式 generation 不变和维护恢复。
- 发现并纠正的既有问题：旧 CLI 吞掉校验错误仍返回 validated；输入没有 candidateSets；来源集合摘要只含 ID；skipped 文件未进集合；read_stable 没核对修改时间和二次内容；Windows verbatim 路径导致 Node EISDIR。
- 契约与迁移决策：
  - CollectorOutput 的六个数组按计划 §3.5 改为必需字段，这是修复现有 DTO 与既定 v1 契约不一致；已从 Rust 重新生成 Schema/TS，无平行真相模型。
  - DB Schema 升到 v2，新增 plugin_candidates；维护迁移保留 indexId/generation；只读不迁移，版本不符明确失败。
  - case、inputFixture、truthRef 参与制品哈希；候选自带 case 永远标 candidate，不自动成为独立业务真值。runtime versionRange 当前支持数字比较条件，未支持语法明确拒绝。
- 命令与结果（完整日志见证据索引）：

  | 命令 | 退出码 | 结果 |
  | --- | --- | --- |
  | `cargo test --workspace --locked` | 0 | 87 passed / 0 failed / 0 ignored（CLI 3、collectors 29、indexing 44、lifecycle 11） |
  | `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 | 无 lint 告警 |
  | `cargo fmt --all -- --check` | 0 | 无差异 |
  | `cargo build -p project-knowledge-cli --no-default-features --locked` | 0 | Core-only 构建通过 |
  | `cargo run -p xtask --locked -- export-contracts --check` | 0 | 5/5 一致 |
  | `node scripts/generate-types.mjs --check` / `pnpm run typecheck` | 0 | 同源类型与 SDK/MCP 编译通过 |
  | `node scripts/verify-vectors.mjs` | 0 | Node 独立实现 18/18 |
  | `node scripts/verify-plan.mjs --plan ../plan/NEW-PROJECT-LONG-TASK.md` | 0 | 24 项状态、DAG、需求映射通过 |
  | `node scripts/verify-collection.mjs` | 0 | 真实 CLI 合成 8 步；坏 link 步按预期 exit 1，其余 exit 0 |
  | `cargo test -p project-knowledge-cli --no-default-features --test collect collector_host_termination --locked` | 0 | Windows 真实宿主终止与两级后代清理独立断言通过 |

- 失败语义：坏输出、伪造依赖、失败 case、CAS 冲突不切正式事实；源码/配置/运行时已确认变化的旧业务批次会先持久 stale；C 内错误整组回滚；C 后未答复可按候选/激活记录幂等确认。
- 新鲜度/覆盖：测试插件为 Node 无依赖合成 smoke；独立业务准确率未评估。模型请求 0，未调用模型客户端。
- 完成证据：`docs/evidence/T08/continuation-2026-10-26/README.md`、`docs/evidence/T09/README.md`；使用说明 `docs/collectors.md`。
- 未完成与下一可执行项：T07 的 Python 3.12、真实 npm/pip hash-lock 安装及运行缓存接线/文件完整性；T00 探针与 T01 CI 等旧门禁保留。T09 剩余是 C 内真实宿主强杀、磁盘满/断电与跨平台恢复；不要把错误注入当这些实测。先补仍可独立验证的 T07/T09 缺口，再满足门禁后进入 T10；T06 独立真值仍未冻结。
- 本轮最后恢复检查补充：暂存库改放本项目 `.project-knowledge/work/staging/`，独立 owner.lock + owner.marker 保证目录所有权；恢复删除前复核最终绝对目标是该根的直接子目录，拒绝链接/重解析点，跳过仍被活动 validate-only 持锁的目录。Windows 宿主终止综合用例覆盖保留活动目录与退出后回收；87 项总数不变（同一用例包含两种宿主模式）。

---

### 2026-10-26 — T00 余项闭环 — Codex / 无真实模型调用

- 状态：T00 doing → done；下一任务 T01（前置 T00 已满足）。
- 现场证据复核：HEAD `2d1da7c` + 既有未提交成果。uv 中已存在 Python 3.12.13，并非此前记录的不可用状态；CI 文件也已存在，T01 将核对修正而不是重复新建。此前 T08/T09 的 87 项有效证据保留。
- 新实现：`docs/evidence/T00/probe/` 独立 Rust workspace 与 Cargo.lock，native 探针直接调用产品 Core；锁定 TypeScript 6.0.3 的 npm lock；Rig 官方 MockCompletionModel 驱动真实工具循环；`scripts/verify-integration-probes.mjs` 保存实际命令、退出码及哈希；`scripts/verify-vectors.py` 是独立 Python 编码实现，仅共享导出槽位规格。
- 实测：Rust 4 passed；Node 5 passed（native、TS UTF-16→UTF-8、legacy MCP、2026 MCP、Python AST/模板）；Python identity 18/18。
- MCP API 结论：2.4.0 Client 默认 legacy，显式 `versionNegotiation.mode={pin:"2026-07-28"}` 才固定现代协议；serveStdio factory 服务两个时代，Standard Schema adapter 的 validate 调 Core，jsonSchema.input/output 回同源 Schema。两条连接均列出精确 Schema、成功调用且拒绝缺 gaps 的参数。
- Rig API 结论：0.45.0 facade 用 `AgentBuilder::new(model)`、Tool::parameters/call(ToolContext, args)、ToolExecutionError::with_model_output、AgentHook::on_outcome；provider 用 OpenAIConfig::new(key).with_base_url(...).client().chat(...)。只验证构造及模拟工具循环，不宣称真实 provider 兼容性或业务准确率。
- 新增锁文件真实执行：npm package-lock-only 与 npm ci --ignore-scripts，registry.npmjs.org，TypeScript 6.0.3 完整性与官方元数据一致；Rust 探针从根 Cargo.lock 保留已选依赖，仅增探针/测试 feature 后锁定，后续使用 --locked。
- 输入设计 SHA-256：DESIGN=`79423864c43d0f9967e2a494bfe41f6a903d014d04b96985e3582ac3edf3483b`；STRUCTURE=`41fce9e90b7d52d421b381b4f7585e9a98f93bd4ad2501edf99064309d504fd4`。历史 `docs/ARCHITECTURE-REVIEW.md` 仍 missing；保留事实，不补造审查。
- 证据：`docs/evidence/T00/{native-build.log,rig-tests.log,node-tests.log,npm-ci.log,python-vectors.json,probe-report.json}`；Python 结果同时保存在 T02 证据目录。
- 初次探针的两次失败来自 Python 测试脚本的转义和缺少 sources 输入字段，已修复测试输入后全部通过；未修改产品校验以放宽验收。

---

### 2026-10-26 — T01 CI 与验证入口闭环 — Codex / 无模型

- 状态：T01 doing → done（前置 T00 已 done）；自动进入 T02 的扩展类型与契约余项。
- 保留有效成果：未新建重复 workflow；修正已有 `.github/workflows/ci.yml`。Rust 测试需要 Node 24，Node/native job 需要指定 Rust 与 Python；已补齐。Rust toolchain action 固定到官方 API 当次返回 commit `683b588c487a69d01f6c6ddf40f036ff682cc2c8`，未使用浮动 master。
- 关键纠错：旧 cargo tree→grep 管道在 cargo 非零时可能仍打印 OK，改为 Node 显式检查 spawn 状态及 Core 依赖节点；实际正例 406 节点、无 Rig/Agent，缺 Cargo 负例拒绝且子退出 1。
- CI 原 binding 为空，require 返回对象不能证明 ABI/业务入口有效，改跑调用真实 Core 的 napi/MCP/Rig/Python 探针。仓库全局 Prettier 属既有格式整治，不冒充已通过；CI 保留计划要求的 Rust fmt，并验证可执行契约/TS/探针。
- 工具固定：Node 24.14.0、Rust 1.97.1、pnpm 10.34.8。CI Python 用 3.12.10（官方 actions/python-versions manifest 确认 darwin/linux/win32 均有制品）；本机 uv Python 3.12.13 仅 Windows 可执行，不假设 GitHub setup-python 能提供该 Windows patch。
- 实际命令：根 pnpm frozen/ignore-scripts 安装、fmt、clippy -D warnings、cargo check、cargo test（87 passed）、Core-only、dependency check、TS typecheck/build、check:contracts、verify:docs、verify:probes 均 exit 0。负例验证入口自身 exit 0，断言其缺 Cargo 子进程 exit 1。
- 证据：`docs/evidence/T01/rust-checks.json` 与同目录日志；探针源/锁/报告在 T00。未触发远程 Actions，四平台发布/安装仍由 T23 验收。

---

### 2026-10-26 — T02 契约与扩展闭环，推进 T06/T07 — Codex / 无模型

- 状态：T02 doing → done；T06 todo → blocked（独立资料缺失，按任务显式规则），自动进入 T07。
- 实现：manifest typed 扩展声明、命名空间/重复/公共覆盖/本地引用限制；候选组+活动 manifest 登记，完整 Core ID/位置/证据/端点校验；动态 kind 持久化、搜索与候选再导出。扩展身份规范化方案写回计划 §3.3 与 docs/protocol.md，内置编码不变。
- IdentitySlot 显式 null 不再被误当缺省；Schema/Serde/TS 一致。生成器修正 union 根遗漏不可达定义，全部公共 DTO、RelationRef 与业务载荷同源导出。
- 实测：cargo test --workspace --locked 92 passed；Clippy -D warnings、check:contracts（5 文件+TS+7 正例/8 负例）、typecheck 通过；Rust/Node/Python 21 向量一致；native/MCP/Rig/Python 探针 4+5+21 通过。生成物哈希见 docs/evidence/T02/verification-report.json。
- 初次失败已修复：动态 kind 所有权适配误触 SQL 字符串，完整回归前已还原；搜索测试读取 entityRef.entityId；Clippy clone 顺序修正。未放宽校验。
- T06：无独立真值、来源审阅或签认，现有 fixture 仅合成 smoke；docs/validation/truth-review.md 保存事实，不编造准确率。T07 已确认 Python 阻塞解除；下一项为真实 npm/pip 锁安装、缓存文件校验及宿主接线。

---

### 2026-10-27 — T07/T08 收尾，自动进入 T09 — Codex / 无模型

- 状态：T07 doing → done；T08 doing → done；T09 doing 继续。T06 独立真值仍 blocked。
- 显式 plugin prepare 贯通真实 npm/pip 锁安装与 Core 采集；缺缓存、坏锁与完整文件树篡改拒绝。ESM 从隔离缓存解析，Python 3.12 从 venv 导入；模型 key 和宿主语言路径不继承，任务 cwd 独立。验证绑定/自动 runtime 依赖加入 cache digest/integrity，运行前后和 C 前复核。
- 安装总时限 120 秒、合并日志上限、跨进程缓存 OS 锁和原子发布；子进程 I/O 同时排空并完整限时，取消/超限/宿主退出清理 owned 后代。
- 真实 CLI 27 步、2 个缓存；全量 Rust 95 passed，Clippy -D warnings、fmt、同源检查与 Core-only 构建通过。命令/输出/hash 在 docs/evidence/T07/dependencies；后续未运行安装器由日志/hash/mtime 不变核验，未声称 OS 断网。
- 实际失败纠正：Windows AppData 跨卷虚拟化 rename；Python 长路径导入；I/O 重构后 stdin 需要显式 drop；Node 探针中文/长路径复制 API。均有最终回归，无校验放宽。
- T08 的扩展登记由 T02 补齐，Python/cache 由 T07 补齐；既有 staging/DAG/事务 B 成果保留。关系单独生成知识反例确认仍从 evidence 根因拒绝。
- 下一项 T09：用真实 owned 子宿主在 C 内被杀与 SQLite FULL 条件补证；物理磁盘满/断电与跨平台不得用可控错误代替。

---

### 2026-10-27 — T09 证据复核闭环，自动推进 T10 — Codex / 无模型

- T09 doing → done，T10 todo → doing；前置依赖已满足。发现最近源码/日志中已有未登记的 C 内真实强杀、SQLite FULL、Linux ENOSPC 和跨平台依赖验证，先核对并复用，没有重写实现。
- 当前复核 Windows activation 13 passed；Linux 14 passed + 隔离满额 tmpfs 1 passed。完整命令、退出码、源码/日志 SHA-256 在 docs/evidence/T09/recheck-report.json。
- 实际断电与 macOS/ARM64 未验证，继续由 U03/T23 约束发布；进程强杀和 SQLite max_page_count 不冒充断电或物理磁盘满。T06 独立真值仍 blocked。
- 首次手工 WSL PATH 转义使 cargo not found；已改用可重放 recheck.sh，后续命令 exit 0。
- T10 现状：run_incremental/run_base_only 仍调用同一全业务失效占位实现，没有按依赖选择阶段，也不重跑活动插件；开始补齐既有 Core 边界内的无模型执行与综合验收。

---

### 2026-10-27 — T10 无模型增量与失败失效闭环 — Codex / 无模型

- 状态：T10 doing → done；沿已满足依赖自动推进后，下一项 T11 的 T06 前置仍 blocked。复核全部 24 项后，未发现依赖全 done 的 todo；没有跳过真值门禁执行业务插件。
- 实现：增量入口比较来源/选择集合、配置、运行时与缓存、完整候选断言集合；当前批次反向闭包及显式 DAG 下游先持久失效，再按稳定 DAG 执行受影响阶段，业务结果在一次 B 事务提交。来源最终复核和 generation CAS 阻止跨输入/跨代拼接。
- 基础索引按仓库摘要复用未变批次，删除/替换仍保存单向失效历史；无变化更新保持阶段 batchId 和正式 generation。阶段输入不回流自身与显式下游旧指针，case 只重放对应阶段。
- CLI 接入异步增量及 executed/invalidated，collect 在失败前也登记输入变化；base-only 不执行或要求插件运行时。运行中断保留 A，维护恢复仅把 running 标 interrupted；内容还原、候选失败和回退都不复活旧 batch。
- 综合验收：Windows cargo test --workspace --locked -j 1 -- --test-threads=1 exit 0，103 passed（CLI 5、collectors 36、indexing 45、lifecycle 17）；实际全工作区 Clippy -D warnings、fmt、Core-only build、Rust 5 个契约文件/TS 同源检查、Core 406 节点依赖方向、24 项计划校验全部 exit 0。
- Linux Core 99 passed / 1 ignored（专用满文件系统已由 T09 单独实测），Core-only CLI 3 passed；最终源码 incremental_ 4 passed。Windows CLI 实际终止增量宿主，验证 owned 插件/孙进程退出、持久 stale、只读不恢复、维护 interrupted 与新批次重跑。
- 五组规范化 incremental/clean 结果完全相同，保存实体、关系、来源、证据、搜索、覆盖、执行阶段及失效闭包；覆盖 0/1/2 候选、新增未引用候选、删除/空输出、属性、改名/移动、ignore、未变仓库、制品/运行时、失败/取消/失败回退、分阶段与 base-only。模型调用为 0（Core 无模型客户端/Agent/Rig 依赖）；合成 fixture 不作为独立业务准确率。
- 真实失败与修正：Windows 并行平台验证触发 OS 1455 提交内存不足（workspace-final.log、clippy.log），改串行 -j 1 后通过，未降低断言；Linux 输出洪泛脚本强制 process.exit 会截断 stdout，改为等待 drain，保留 OUTPUT_LIMIT 断言，最终两平台 collectors 通过。
- Linux 独立 Clippy 组件产生零字节可执行文件，发现后明确作废其退出 0 记录；安装修复遇遗留组件路径冲突，未冒充 Linux lint 通过，有效 lint 为 Windows 全工作区 Clippy。只对本任务 WSL 缓存建议释放干净文件页，未更改系统缓存策略或终止用户服务。
- 完整命令/退出码/源码与输入及日志 SHA-256：docs/evidence/T10/verification-report.json；用法：docs/collectors.md；规范化结果：docs/evidence/T10/comparisons。
- T06 事实纠正：truth 和三个场景目录存在，但仅 .gitkeep；仍无独立资料/审阅记录/truthHash。已在异步问题中请求已有资料的本地路径与确认来源，未改写真值或验收条件；到位后才可解除 T06 并进入 T11。

---

### 2026-10-27 — 真实项目来源接入与 AI 插件边界确认 — Codex / 无产品模型调用

- 用户提供 E:/Project/tbm/synes，并明确真实项目应由 AI 调查后生成插件，不能把示例或真实项目规则直接写死到 SDK。
- 只读确认该原始项目工作树干净，HEAD 6f47bdbf901f66e59909f8e24edd2e41a1b4f379，作者 hu-bo，提交时间 2026-10-20T21:58:42+08:00；发现前端/后端源码、业务文档和历史评审材料。未修改原始项目，未运行其业务服务或数据库命令。
- T06 blocked → doing：原始资料缺失解除，继续建立独立真值包；没有把用户提供路径或 Git 作者信息当作真值签认，T11 前置仍未完成。来源记录：docs/evidence/T06/source-intake.json。
- 决策 D20：AI 调查 → 生成项目候选插件 → Core 准备/验证/激活/运行 → SDK 通用调用；项目路径、框架注册、ORM 映射及业务状态只在插件和带引用的项目调查材料中。接入另一项目无需修改或重新发布 SDK/native/Core。
- 核对当前 SDK index/client 仍为骨架；SDK/native/Core 没有 synes 或采购台账专属解析代码。本轮未添加此类实现，也未提前把 T18/T19 自动生成入口标为完成。
- 更新 shared/investigate/collector/knowledge Prompt：从用户指定目标调查，不预置采购台账；生成可对当前输入快照重新提取事实的脚本，不把示例事实或 expected 写死为输出。T11/T19/T20 增加相应生成/换项目验收约束；docs/collectors.md 同步说明当前真实能力。
- 验证：node scripts/verify-plan.mjs --plan ../plan/NEW-PROJECT-LONG-TASK.md exit 0，24 项状态/DAG/映射通过；git diff --check exit 0。仅调整指令与文档，没有重跑不相关的 Rust/TS 功能测试。

---

### 2026-10-28 — T06 真值冻结闭环，自动进入 T11 — Codex / 无产品模型调用

- 状态：T06 doing → done；T11 todo → doing。前置 T05/T10 的有效证据保留。工作目录 E:/Project/my-project/code-graph，基准提交 2d1da7c，沿用未提交成果；本轮源码 hash 见 docs/evidence/T06/verification-report.json。
- 输入：用户已提供的 synes 原始源码，HEAD 6f47bdbf901f66e59909f8e24edd2e41a1b4f379，复核工作树干净。仅只读原项目，不执行其业务服务/数据库。11 份原文冻结、逐文件最后 Git 作者/日期/hash 在 fixtures/original/provenance.json。
- 审阅：从原始注册、i18n、页面、GraphQL、ORM、工作流/权限资料和既有测试建立事实/禁止误连依据，未使用待测插件输出。当前 /Other/purchaseLedger 的菜单是“生产采购申请”，历史 e2e 权限注释与当前路由不同，保存冲突原文，不照搬通用示例名称或自动裁决。
- 输出：fixtures/truth/{manifest,scenario1,scenario2,scenario3,negative,frontend}.json 与 review.md，50 个冻结文件，truthHash aaf1fd10b75902583be1108b083e6d3a2f71cb515f633b116ca71967f780b1ed。A/B 完整可读 TS 前后分仓和 JS/TSX+Python 注册/请求/后端绑定样本、流程条件/复合键、数值 SQL/DDL/数据均已创建。
- 分类：原始声明事实为源码独立审阅；A/B 为合成 smoke；数值为按计划设计样本的独立算术审计。不声称人类签认、生产口径正确或业务准确率。SDK/Core 未增加项目解析规则。决策变更：无。
- 实测：python docs/evidence/T06/verify.py exit 0。Python 不使用 SQL/采集器，按原始行整数累计；Rust 内存 SQLite 返回相同三行与合计 [6,5,12,15100,2000]，金额单位分。fanout 反例首行收货 10/发票 4000，WHERE 子表过滤只剩 2 行，均被识别。
- Rust truth_ 4 passed：50 内容/hash 与原始作者来源、每条证据行回读、26 expected entity/20 expected relation 身份及 forbidden 隔离、SQLite 数值与反例、候选越界写入拒绝且同名相对目录仅写入 owned work。命令、退出码、日志和 hash 在 docs/evidence/T06/verification-report.json；原始输出同目录。
- 首次测试构建纠正 RelationKind 参数类型；随后候选边界断言修正 Windows canonical UNC 路径前缀比较。未降低断言或修改产品写入边界。失败只影响测试，无正式库提交。
- 预算：产品模型请求 0；普通采集模型调用 0。新鲜度/覆盖：本任务仅冻结验收输入，不产生业务活动批次。
- 下一可执行任务：T11，两布局 AST 提取与 Core 驱动 frontend_ 验收；复用锁定 TypeScript 6.0.3 和宿主依赖隔离，项目约定仅进入插件制品。

---

### 2026-10-28 08:47 Asia/Shanghai — T11 前端采集闭环，自动进入 T12 — Codex / 无产品模型调用

- 状态：T11 doing → done；T12 todo → doing。前置 T10/T06 已 done，核对其实现与已存证据后执行；未重建既有生命周期/存储实现。
- 工作目录/源码：E:/Project/my-project/code-graph，基准 Git 2d1da7c，保留此前未提交成果。修改源码/独立制品 SHA-256 在 docs/evidence/T11/verification-report.json，输入 truthHash aaf1fd10b75902583be1108b083e6d3a2f71cb515f633b116ca71967f780b1ed。
- 实现：两布局独立 frontend manifest/约定/case/锁；共同 AST 机械逻辑在 collectors/templates/node/lib/frontend-ast.mjs，经 scripts/package-example-collectors.mjs 打成自包含制品。TypeScript 6.0.3、GraphQL 16.11.0，依赖仅宿主 prepare，采集无网络安装/无模型/不继承业务 node_modules。
- 可观察结果：静态 route/hash/history/alias 数组/直接与 lazy import、inline/i18n 文案、JSX→handler→imported wrapper→HTTP/GraphQL、JSDoc/模块/定时/消息/命令入口有原始引用。UTF-16 offset 经原文映射 UTF-8 字节；内嵌 GraphQL 引用完整原始调用范围。无任意包 re-export、框架模板、运行时实例推测。
- 异常：动态 URL/client/proxy/title/组件、参数和局部变量遮蔽、嵌套入口、标题零/多匹配及语法失败返回 partial/gap；未知注册模式为 notConfigured。同名 decoy 零误连；移动目录/组件与业务改名后按新身份输出；动态 proxy 不标 bindingComplete。
- Core 修复：staging case 重放原来只保留 projectId，丢失 scopes/services/bindings；现用与 live export 相同的宿主配置，仍不信任 case 配置。不变更公共 DTO/Schema 或放宽关系端点；GraphQL request 使用合法的 owner requests operation 与 request references operation。
- 实测命令/退出码/工具版本/输出：docs/evidence/T11/{verification-report.json,tool-versions.json}。node scripts/verify-plugins.mjs --capabilities routes,frontend,comments,modules,entrypoints exit 0；两布局独立真值 26 expected entity / 20 expected relation 全命中、forbidden 0，Core 重放两 case 全通过。原始快照和引用在 frontend-{a,b}-snapshot.json；候选 case 仍标 candidate，不冒称独立生产准确率。
- 全工作区 cargo test --workspace --locked -j 1 -- --test-threads=1 exit 0，125 passed（Agent 13、CLI 6、business 8、collectors 36、indexing 45、lifecycle 17）。Clippy 全工作区 -D warnings、fmt、Rust Schema 5 文件/TS 同源、计划、50 文件 truth 与 Python oracle 均 exit 0。新功能的查询与来源均 fresh 于临时 fixture；未创建真实项目活动业务批次。数值和真实来源分类沿 T06。
- 实际失败：测试曾错误使用 rusqlite 不支持的 usize/u64 读取，改 i64；coverage 负例误把 base notConfigured 行计入插件，改按 replacement_key 隔离，保留原失败日志。全工作区的 serde_json preserve_order feature 暴露测试用普通序列化重算 truthHash 的不稳定性，改用既有规范 JSON，原冻结真值不变，失败日志 workspace-map-order-failed.log。一次执行中更新制品触发 ARTIFACT_CHANGED，未作为通过证据；最终固定制品后全回归。
- 决策变更：仅登记示例 GraphQL 精确依赖及解析来源（docs/decisions/dependencies.md §9），公共契约未变化。预算：产品模型请求 0；普通采集模型调用 0。原始 synes 工作树再次只读确认干净；未测试其运行环境或生产业务准确率。
- 下一可执行任务：T12，先接入两布局 HTTP/GraphQL 后端注册与真实 import/handler 绑定，再补数据源/SQL AST；T13 前置仍未满足。

---

### 2026-10-28 08:58 Asia/Shanghai — T12 API 子闭环 — Codex / 无产品模型调用

- 状态：T12 doing 继续；T13 未启动。前置 T11 已 done，未以 API 子能力替代本任务的数据库门禁。
- 工作目录：E:/Project/my-project/code-graph，基准提交 2d1da7c，沿用既有未提交成果；本轮实际源码/独立制品及 npm 锁/hash 见 docs/evidence/T12/api-verification-report.json。
- 实现文件：Node api-ast.mjs 与 Python api_ast.py、两布局独立 api manifest/约定/case、scripts/package-api-examples.mjs；scripts/verify-plugins.mjs 增加 api 能力入口，database 仍显式拒绝。TypeScript/GraphQL 复用已锁依赖；Python 3.12 仅 stdlib AST，身份编码/槽位规格使用既有 T02 验证实现与同源 schema，业务源码不被 import 或执行。
- 输入：T06 冻结 A/B 原始注册与业务方法源码；fixtures/truth-supplements/T12-api.json 单独记录逐行出处、合成审阅边界与 baselineTruthHash。补充文件 SHA-256 0656a541167d8195b7a7286c1dc660fcb730fd826778e50f69051f129f5374cf，不改原 truthHash，不从插件输出反推 expected，不声称维护者签认。
- 可观察闭环：HTTP registration → handles → 明确 local/imported operation；GraphQL schema type+field → resolves → resolver；service 来自唯一显式配置 scope。两布局各 4 expected entity/3 expected relation，case 与正式来源均由 Core 校验/提交，forbidden decoy 0。动态路径不建 endpoint，动态 handler 不确认 handles，旧批次被替换，源码 UTF-8 证据可回读。
- 跨生产者校正：frontend 采集的服务内 GraphQL operation/field 原来使用前端文件 scope；现按已有 serviceId 登记的 service.scopeId 存放。两插件一组 staging/commit 后同一字段保留两份原始断言，无 scope 覆写；身份与公共 DTO 不变，frontend 回归同时通过。Python 参数/局部变量遮蔽与嵌套函数不会被当作外部函数绑定。
- 实测：python docs/evidence/T12/verify-api.py exit 0，完整命令/退出码/日志 hash 在 api-verification-report.json。两个 API 综合用例通过，前端回归通过；cargo test --workspace --locked -j 1 -- --test-threads=1 共 127 passed（Agent 13、CLI 6、business 10、collectors 36、indexing 45、lifecycle 17）。Clippy 全工作区 -D warnings、fmt、50 冻结文件与独立 Python oracle、计划均 exit 0。普通采集产品模型调用 0；Agent 测试只使用本地 mock，不产生外部模型请求。
- 证据保存：api-{a,b}.json 和 api-{a,b}-snapshot.json；frontend 当前回归快照同目录。Python 测试读取本机 uv 中已存在 3.12.13（不安装、不修改宿主 PATH）；运行时路径仅写临时测试配置，产品仍要求显式配置/满足版本门禁。
- 复现性补充：.gitattributes 对 fixtures/original 与 T06/T11/T12 证据设置 -text，防止 Git 行尾归一化破坏逐字节原始来源和记录的日志/JSON hash。没有重写原始来源或补造通过；SDK/native/Core 仍无项目 API/ORM 适配规则。
- 新鲜度/覆盖：临时 smoke API 批次为当前输入 fresh，动态目标 partial；knowledge 与 database 未实现，不宣称统一完整。失败/中断/回滚：本子闭环无失败提交，无原始项目写入。决策变更：无公共契约变更。
- 未完成：数据源/ORM 实例绑定、DDL 与 SQLGlot 解析、queryImplementation、复合键/joins 及对应独立反例。T12 保持 doing，T13 前置未满足。下一可执行项仍为 T12 数据库部分，首条实际核对命令：Get-Content crates/core/src/contract/relation.rs；复用已冻结 purchase-ledger/{schema,correct,fanout,where-child}.sql 和现有宿主 prepare，禁止连接业务数据库。

---

### 2026-10-28 13:00 Asia/Shanghai — T12 数据库采集闭环与空实现核对 — Codex / 无产品模型调用

- 状态：T12 doing → done；前置 T11 已 done，先读计划、最近执行记录、API 证据与源码后补齐数据库部分，未重复规划或覆盖既有未提交成果。下一依赖已满足的最早任务为 T13（todo）。
- 工作目录/版本：E:/Project/my-project/code-graph，Git 基准 2d1da7c；实际源码/制品、真值与日志 SHA-256 在 docs/evidence/T12/database/{verification-report,final-checks}.json。原始 synes 项目只读复核为干净，未运行服务或 SQL。
- 空实现专项：scripts/audit-source.mjs 扫描 151 个源码，零字节源码 0；只有注释 9、空导出模块 5。graph/codemap 对应 T15、knowledge 对应 T17、native/SDK 对应 T20、MCP 对应 T21，另有范围外 SCIP；四场景 execute 仍调用 skeleton。保存完整行号与哈希，没有用空返回补齐这些后续能力或改成 done。Python 生成字节码加入 gitignore，制品清单不包含运行缓存。
- 实现：collectors/templates/python/lib/{source_ast,database_ast}.py、两布局独立 database manifest/约定/hash-lock/case、scripts/package-database-examples.mjs；verify-plugins 增加 database 能力。A 用 TS Tree-sitter AST，B 用 stdlib AST，SQLGlot 30.24.0 用显式 SQLite 方言解析 DDL/SELECT/CTE，不使用 optimizer/执行器、不导入业务代码。所有准备/运行/验证/提交由既有 Core 负责，Core/SDK 无项目专属 ORM 规则。
- 可观察结果：2 数据源同名表隔离；每布局 90 DDL expected（2 dataSource、10 dataTable、62 dataColumn、16 dataKey），正确查询 4 个有序 LEFT JOIN 全命中，基数 unknown。明确相对 import/词法绑定的 connect/model/query/findAll 生成映射、queryImplementation 和 reads/calls；原始字符串/调用范围有 UTF-8 证据。业务粒度/状态/NULL 策略/单位等 T14 口径保留 unknown+gap。
- 正常/异常：Core 真实 prepare→run→case→validate→staging→commit；错误实例 forbidden 跨源 reads=0，缺 import/动态 SQL/动态实例/参数与局部遮蔽/未知 ORM、模型与连接键缺列、未绑定 SQL、解析失败和旧批次替换均验证。未改 T06 的 50 个冻结文件或 truthHash；新增带明确导入样本及独立逐行审阅/hash 在 fixtures/truth-supplements/T12-database，reviewHash c59bfa400fe1e49f2713e80374d1d97e9c5aa85e1744484c238578564d4ef6c6。不声称人类签认或生产准确率。
- 契约纠错：发现 RelationKind 的 maps_to/transitions_to/relates_document 在登记/身份中为 snake_case、Serde 却为 camelCase；按既定计划契约修正显式 Serde rename，Rust 重生 Schema/TS，并在既有 contract 测试对全部关系核对 wire/as_str/往返一致。不是 SDK/插件私改协议。新 schemaHash 见 docs/schema/protocol.json，既有证据作为历史保留。
- 命令与结果（完整命令、工具版本、退出码与日志 hash 见上述两份报告）：
  - node scripts/verify-plugins.mjs --capabilities api,database exit 0；两 API、两数据库综合用例通过。
  - cargo test --workspace --locked -j 1 -- --test-threads=1 exit 0，129 passed / 0 failed / 0 ignored（Agent 13、CLI 6、business 12、collectors 36、indexing 45、lifecycle 17）。
  - 全工作区 Clippy -D warnings、fmt、Rust 5 个契约/TS 生成检查、7 正例/8 负例 TS 边界、pnpm run typecheck 均 exit 0；50 文件 truth 与 Python 独立整数 oracle、24 项计划、最终打包一致性与 git diff --check 均 exit 0。
  - Rust/Cargo 1.97.1、Node 24.14.0、pnpm 10.34.8、插件 Python 3.12.13；工具输出 tools.log。Windows x64 本轮验收；其他平台未重新运行，由 T23 继续。
- 真实失败/修复：backend-schema-failed.log 记录蛇形关系 wire 被旧 Schema 拒绝；backend-parse-failed.log 记录 SQLGlot error_level 字符串未触发严格错误，改 ErrorLevel.RAISE 后坏 SQL 正确 gap。初次直接探针的默认 GBK/可选 identity undefined 已修正。最终未放宽任何 Core 校验、未提交失败业务批次。
- 新鲜度/覆盖：临时 fixture 的当前数据库批次 fresh；动态/未知语义 partial。knowledge 尚未实现。本任务不创建真实项目活动业务批次。预算：产品模型请求 0、普通采集模型调用 0、业务数据库连接 0；依赖只在 prepare 联网，采集只用已准备缓存。
- 完成证据：docs/evidence/T12/database/README.md、verification-report.json、final-checks.json、database-{a,b}{,-snapshot}.json、source-audit.json 与日志；原始 API 子闭环证据继续保留。
- 未验证边界：任意 ORM/运行时构造、未知方言、生产业务准确率和四平台交付不在本轮验收范围；T14—T23 状态不变。下一可执行项：T13，首条核对命令 Get-Content fixtures/truth/negative.json；从现有 staging 导出与显式绑定继续，不按名称猜跨层关系。

---

### 2026-10-28 — T13 实施与阶段验收 — Codex / 无产品模型调用

- 状态：T13 todo → doing；依赖 T12 已 done。先核对当前状态、最近记录、T12 证据与原始绑定源码；沿用既有未提交成果，未重复规划。
- 实现：Core 导出所选当前来源版本中被断言引用的 evidence；candidateSets 摘要增加相关关系断言；validator 拒绝缺失/重复/实体集合不一致的 selector。新增 A/B 独立 link 制品及 package-link-examples，复用身份编码，不创建相似实体，不把项目规则写进 SDK/Core。
- 决策 D23：CollectorInput.evidence 为可省略数组（旧输入默认空），从 Rust 重生 Schema/TS。前端请求补充实际 clientName，关联按显式 client/service + 实际 HTTP/GraphQL 注册或原始 query reads 决定；缺绑定和多 resolver 保持候选，人工原始断言保留 declared/human。
- 已通过：两布局登记/整组验证/激活的跨仓请求、GraphQL resolver 与数据源关联；缺调用证据、缺绑定、人工来源、候选截断；四组新增 resolver/消除歧义/代理与数据源变化的增量和干净重建实体/关系/覆盖逐项相等。比较保留全部重复记录，规范化结果在 docs/evidence/T13/normalized-*.json。
- 真实失败：导出最初用 rusqlite 不支持的 u64 读取，改沿库约束的 i64；归一化测试先取错 producer_key 所在表，随后同作用域覆盖记录排序不稳定。原始失败日志/完整左右值保留在 docs/evidence/T13，后续稳定排序保持记录数和字段不变，增量综合用例最终 exit 0。
- 当前门禁：docs/evidence/T13/verify.py 正在执行最终工作区回归、Clippy/fmt、契约/类型、冻结真值与扫描。完成报告未产生前不改 done；T14 尚未执行。产品模型调用 0，未连接业务数据库，原始 T06 truthHash 不变。

### 2026-10-28 — T13 完成，继续 T14 — Codex / 无产品模型调用

- 状态：T13 doing → done，T14 todo → doing；依赖均满足。最终门禁脚本 exit 0，全工作区 131 passed / 0 failed / 0 ignored（Agent 13、CLI 6、business 14、collectors 36、indexing 45、lifecycle 17），Clippy -D warnings、fmt、Rust Schema 5 文件/TS 同源与类型边界、typecheck、50 冻结文件/独立 oracle、计划均通过。
- 复核：verification-report.json 中全部源码/制品 hash 再核对相符；四组规范化比较逐份 JSON 相等，包含全部重复记录，comparison-report.json 保存文件 hash 与行数。业务记录没有因运行顺序不稳定而删除或合并。
- 完成证据：docs/evidence/T13/{README.md,verification-report.json,comparison-report.json} 与日志/原始快照；D23 已同步计划与 docs/protocol.md，历史 T12 证据保留为历史版本。产品模型请求、业务数据库连接、普通采集模型调用均为 0。
- 下一项：T14，先复用已冻结 purchase-workflow/{flow.ts,rules.md}、purchase-ledger/{policy.md,correct.sql,fanout.sql,where-child.sql}，实现支持语法的原始流程/字段语义采集，不将 calls 转成业务 transitions，不重写 T06 真值。

### 2026-10-28 — T14 完成，继续 T15 — Codex / 无产品模型调用

- 状态：T14 doing → done；T15 todo → doing，前置 T13 已 done。新增 workflow/report Python AST 采集器与 A/B 独立 semantics 制品，复用锁定运行时和 Core 验证/激活，不修改冻结真值。
- 每布局验收：6 状态、14 条文档/代码流转断言、2 页面先后、2 单据关系（3 有序键对），9 字段完整 lineage；文档/代码 gross/net 冲突保留两个断言。缺文档只有 calls/状态写入，无流程边；错状态对象、缺 policy、错粒度/tenant/状态/删除/NULL 均返回 gap。数量单位、时间字段和未知基数不猜测。
- 同源修复：TS 参数按 AST token 归一化，保留字面量、跳过格式/注释，API 与 workflow operation ID 一致；guard 引用覆盖原始函数和文档。
- 最终 docs/evidence/T14/verify.py exit 0：14 命令通过，semantics 2 passed、frontend 回归 6 passed，Clippy all-targets -D warnings、fmt、Rust/TS 契约、50 冻结文件与独立整数 oracle、计划与源码扫描通过。109 源码/制品 hash 和全部日志 SHA-256 再核对一致。T13 全工作区 131 passed 属历史门禁，T14 未重复无关测试。
- 证据 docs/evidence/T14/{README.md,verification-report.json}、两布局快照及完整日志；独立补充 reviewHash 4dea260f6c096557bf40f0bacf10ee762f21b02cfedd6d6a5d7024ca7aea698f。产品模型请求 0、业务数据库连接 0，本轮 Windows x64，不声称生产口径或四平台交付。
- 下一项 T15：补齐 graph/codemap 的注释占位与通用查询边界；T16 场景组合及后续任务仍按依赖继续，不提前标记完成。

### 2026-10-28 — T15 完成，继续 T16 — Codex / 无真实远程模型调用

- 状态：T15 doing → done，T16 todo → doing；前置 T14/T15 均满足。graph/neighbors、graph/paths、codemap/rank、codemap/render 四处注释占位替换为实际有界邻接、BFS、入口/依赖选择与声明轮廓，并接入统一入口。
- D24：单根 neighbors keyset 与不可恢复的 trace BFS 分开；Rust 重生 Schema/TS，类型边界 8 正例/9 负例。游标绑定请求/快照/稳定键，字符裁剪后只推进到最后保留项。visited 为实际检查候选/边/扫描项，实体另行去重。
- 查询固定只读事务和 query_only、SQLite VM/Core 共用取消、预算约束来源扫描；默认 stale 不返回旧事实，indexed/allowStale 保持 unknown/partial。原始引用必须匹配存储的快照/字节范围；只有显式 allowStale 返回历史片段。Agent 改为调用同一校验和取消入口，调查校验的字符预算包含信封。
- 最终验证 docs/evidence/T15/verify.py exit 0：索引 49、Agent/CLI 19、采集扩展搜索回归 1 passed，0 failed/ignored；全目标 Clippy -D warnings、fmt、同源契约/类型、冻结真值/oracle、源码/计划/diff 检查通过。30 符号/58 邻接常规与小字符分页无重无漏；高出度/回环/预算/frontier、跨代/跨库、inflight 取消、伪造/失效来源通过。
- 只读证据：持久表全逻辑摘要与主文件前后相同，SQL trace 无 DML/DDL，实际搜索和双向邻接 SQL 的 EXPLAIN 使用预期索引。源代码、制品、日志和四份响应快照 hash 已再次核对；完整证据 docs/evidence/T15/{README.md,verification-report.json,search.json,graph.json,boundaries.json,evidence-reads.json}。
- 空实现：180 源码、零字节 0；注释-only 5、空导出 5。剩余 knowledge(T17)、native/SDK(T20)、MCP(T21) 和范围外 SCIP，不改为空成功。四个场景查询仍显式 skeleton，下一 T16 继续补齐。
- 早期编译、测试输入与旧字节范围用例失败保留日志，未放宽断言唯一性/证据校验。Rust 1.97.1/Node 24.14.0/pnpm 10.34.8，Windows x64；业务 DB 连接/真实远程模型请求 0，未改 50 冻结文件，不声称四平台交付。

### 2026-10-28 — T16 完成与下一任务交接 — Codex / 无真实远程模型调用

- 状态：T16 doing → done；依赖 T14/T15 已 done。替换 feature/page/workflow/report 四个查询骨架及 verify-scenarios 的失败占位，删除 scenario_skeleton。统一只读事务、快照/取消/预算、原始引用校验；Node 驱动不重写图算法。
- 实际 CLI：两布局各 page/workflow/report 共 6 主响应与 Core 规范化相等；HTTP/GraphQL/反向页面路径、alias、中文词项召回、跨入口 search 游标、动态路由候选与无流程资料反例均执行。108 原始引用回读 current，所有明确结论沿原始断言，不以路径长短挑选业务结果。
- 输入核对：冻结 service 源码的 erp 缺 import，原样测试确有 PAGE_DATA_SOURCE_MISSING；单独审阅 fixtures/truth-supplements/T16-bindings 后在自有临时目录补显式 import，才确认两个布局 ds.erp 的 7 个 purchase_detail 列。50 冻结文件/truthHash 未改。通用 SQL AST 补完整 DDL 范围的 wildcard reads、重复输出名称不冒充唯一 CTE 列；嵌套 statement.text 纳入搜索。
- 流程/报表：每布局 6 状态、14 个独立代码/原文流转、两个 3 键单据关系、9 字段完整 lineage，冲突与未知口径保留。移除文档只见 codeCalls，不生成业务流转；已有 fanout/WHERE 子表过滤明确 gap。numeric-sql.json 的实际行/合计与独立 Python 整数 oracle 完全相等，合计 [6,5,12,15100,2000]；生产 SQL 未执行，业务数据库连接 0。
- D25 与真实失败：CLI 对照先混用过编译产物，冻结后仍有一个路径裁剪差异。cargo feature 图确认 Rig 启用 preserve_order，改变 JSON Map 并列遍历及请求 hash。改显式字段名并列规则、Core canonical_json、固定元数据预留，不放宽相等断言；完整失败 JSON 和日志保留。readSource 预留信封并标片段截断，来源 current/expired 与片段预算分别表达；无参数 CodeMap 也返回真实有界地图。
- 最终回归：business_scenarios 18、indexing 49、Agent/CLI 19，共 86 passed / 0 failed / 0 ignored；18 条最终命令和 4 条 CLI 驱动命令 exit 0，含全目标 Clippy -D warnings、fmt、Schema/TS/类型边界、五组制品一致性、真值/oracle、源码/计划/diff。完整证据 docs/evidence/T16/{README.md,verification-report.json,cli-report.json,numeric-sql.json}。
- 制品：Cargo 测试会改写 debug CLI；最终将 CLI 复制到 target/verified-cli/<SHA-256>/ 再执行整套 CLI 矩阵，记录该不可变副本路径/hash。最终产品源码与整套回归时一致；仅验收驱动调整副本路径，未将未测二进制替换进报告。
- 空实现复核：182 源码，零字节 0；4 个 graph/codemap 注释占位、4 个场景骨架和验收脚本占位已解决。仍有 knowledge 3、native/SDK 3、MCP 3 个必做占位和范围外 SCIP 1；未用空成功掩盖，后续状态保持未完成。
- 下一依赖满足任务：T17，先读 crates/core/src/knowledge/{context,documents,skill}.rs、templates 与 generate CLI，实现主题契约/版本依赖/导入导出后验收；不重复规划。T18/T19 的既有模型证据为历史，知识/SDK/MCP/四平台没有提前关闭。


### 2026-10-28 — T17 续做与最终验收启动 — Codex / 无模型调用

- 状态：T17 保持 doing；依赖 T16 已 done。先复核 T16 最后记录、185 份当前源码及历史证据，保留所有既有未提交成果，不重复规划。knowledge 三个注释占位、generate CLI 失败骨架和三个模板占位已替换。
- 实现：Core context 收据/原始字节引用、逐 claim/topic 校验与版本保存、派生约定原始 provenance 和 identityDelta CAS、主题/导出依赖复核与持久 stale、strict Handlebars 全文件输出、只读 knowledge status/哈希核验。config/sourceSet/facts/plugins/batch/selectors/conventions/prompt/template 均绑定；生成控制输出扫描回流为零。
- D26：Schema migration v3 新增上下文/结构化主题/约定/导出尝试，Rust 同源导出知识类型及 TS。正文 [[claim:id]]、Mermaid %% [[claim:id]] 映射；完整版本目录 versions/<exportId> 加 CURRENT.json 原子选择，返回可安装 directory。没有外部依赖/API 版本更改；CLI 新增直接使用 workspace schemars 的输入校验。
- 正常/异常：6 个 knowledge 综合用例与全目标 Clippy 预检通过；真实 CLI context→docs→skill、原始 SQL 回读/完整文件 hash、fresh→stale→重渲染 stale→新 context/import version 2→fresh 通过。3 个取消点及发布前源文件改变保留旧完整输出；未将旧 topic generation 改成新生成。
- 真实失败：保留早期编译接线和测试日志。Windows 完成标记文件句柄未关闭导致目录 rename 拒绝，已关闭再发布；原输出定位按 exportId，避免同一时间戳中的随机 ID 排序误判。原始依据范围扩展到完整 SQL/方法正文，不把文件声明的单字节范围当业务内容证据。
- 最终门禁：python docs/evidence/T17/verify.py 已启动，实际 CLI 已通过，工作区回归进行中。完整报告成功生成并 hash 复核前不关闭 T17；T18/T19 的 Agent 知识接线、SDK/MCP 与四平台交付仍未关闭。产品模型请求/业务数据库连接为 0，50 冻结文件与独立真值不改。


### 2026-10-28 — T17 完成与 T18 交接 — Codex / 无真实远程模型调用

- 状态：T17 doing → done；T16 依赖已满足。CLI/core 的 context→外部 topic→Skill 闭环真实运行，knowledge 三个纯注释文件、generate 失败骨架及三个模板占位已解决。保留既有工作区成果，未提交 Git，HEAD 仍为 2d1da7c 加保存的源码哈希清单。
- D26 已同步计划和 docs/protocol.md；migration v3、新增知识类型与 Rust 同源 Schema/TS、Core receipt、原始完整 UTF-8 范围、逐 claim/basis/origin/gap、派生约定 provenance/CAS、topic version 和依赖复核落地。引用可追溯只证明来源，未外推生产业务准确率。
- 依赖/新鲜度：按选中仓库保守绑定 sourceSet/facts/plugins/batch/selectors/config/conventions/prompt/template。无关仓库 generation 改动保留原 topic snapshot 并复核；相关 SQL、候选/批次/插件或约定改变使旧解释持久 stale。只重渲染及恢复旧文件不复活解释，重新导入当前 context 才生成下一版。partial 原因强制传播为不可删除的主题 gap。
- 发布/异常：strict Handlebars 输出完整 Skill 和全部 references，manifest 绑定逐文件哈希、原始快照和依赖；COMPLETE 绑定 manifest，CURRENT 原子选择 versions/<exportId>。3 个取消点、发布前来源变更保留旧完整输出；status 只读复核来源/文件完整性，预算不足 unknown。仓库内输出硬排除，扫描回流 0；不连接业务数据库、不运行模型。
- 最终命令与证据：python docs/evidence/T17/verify.py 的工作区 145 passed/0 failed/0 ignored（Agent 13、CLI 6、business 18、collectors 36、indexing 55、lifecycle 17）。随后发现并修正 partial 缺口传播，只变更 context 和知识测试两份已记录源码；python docs/evidence/T17/verify.py --reuse-workspace 校验原 inventory/广泛回归日志哈希后，重跑完整 indexing 55、12 条实际 CLI、Clippy -D warnings/fmt、Schema/TS/类型边界/typecheck、50 冻结文件和独立算术 oracle、源码/计划/diff。13 项最终命令记录均 exit 0，广泛回归标记 reused，未把 200 次测试执行当作 200 个独立用例。
- 正常与反例：6 个 knowledge 综合测试，14 组伪造/循环/版本/缺口删除拒绝，加生成知识/未展开派生引用拒绝、推断 gap/Mermaid 映射、真实 Node 插件/零匹配集合/失效、身份 CAS 和输出完整性。实际 CLI 原始 SQL 回读、所有文件 SHA-256、fresh→stale→重渲染 stale→导入 version 2→fresh 均通过，过期命令 exit 2；不可变 CLI 副本 hash 记录于 cli-report.json。
- 哈希复核：verification-report.json 全部命令/日志、源码、快照、不可变 CLI 以及 source-audit.json 的 185 份源码哈希再次比较一致。早期失败日志保留；最终证据 docs/evidence/T17/{README.md,verification-report.json,cli-report.json,source-audit.json,unfinished-review.json}，cli-skill/ 是可检查的完整输出样本。此前 T16 证据作为历史保留，未改 T06 truthHash。
- 空逻辑扫描：185 份源码无零字节；注释-only 剩 native 1 与范围外 SCIP 1，空导出剩 SDK 2/MCP 3；发布验收与 xtask package 仍明确失败（T23）。小型 mod.rs 为有效模块导出，空回调为故障/反例 fixture，不当成产品实现。Agent 当前仍保存 investigation draft，knowledgeAvailable/identityImportAvailable=false，已记录到 T18/T19，不伪报完成。
- 下一依赖满足任务：T18（T00/T09 已 done）。先读 crates/agent/src/{config,prompts,tools}.rs、runner.rs 及 docs/evidence/T18/T19 的既有模型记录，接入已完成 T17 的原始约定/主题工具；首先 cargo test -p project-knowledge-agent --test agent_lifecycle config_ --locked，再按 tools_/知识接线验收。T19/SDK/MCP/四平台继续按现有依赖门禁，当前轮仅 Windows x64。


### 2026-10-28 — T18 续做完成，T19 交接 — Codex / 两次真实查询 smoke

- 状态：T18 doing → done；T19 todo → doing。开始时核对 T17 完成报告全部源码哈希一致，读取 T18/T19 的既有真实 smoke、构建/失败摘要和当前 Agent 源码；保留既有未提交工作，HEAD 仍为 2d1da7c 加已保存文件哈希，不重复规划。
- 实现与 D27：import-conventions 从调查草稿改为 Core 登记 derivedConvention/原始依据，identityDelta 必须 expectedConfigHash；Core 完成 CAS 与验证，Agent 更新配置元数据并导航 update-base。按 withSkill 注册 export-topic-context/import-topic/export-skill，主题收据限当前 run，逐 claim/引用/缺口使用 Core 校验；缺产物改为 KNOWLEDGE_INCOMPLETE，不能仅因模型停止就报 Skill 完成。
- Prompt/Schema：shared→investigate→collector→link→repair→knowledge 及有界项目补充按顺序组合；真实工具名、实际 allowlist、manifest schemaHash、configHash/scopeIds 导航同源。Tool 参数保持最小形状，公共 TopicArtifact/IdentityDelta 从 Rust Schema 取得，完整错误 JSON Pointer 保留。无框架/业务项目规则进入 Core/SDK，无外部依赖版本或协议 wire 字段更改。
- 取消/时间：KnowledgeControl 共用原有 run 剩余截止时间和取消令牌，不重置任何轮数/工具/修复预算。Core 查询循环、文件发布、SQLite 取得事务后与 commit 前检查，取消则回滚新收据/主题/导出；约定配置在写入前检查并完成短临界提交。SQL trace 同步点验证查询中/BEGIN IMMEDIATE 时取消，Skill transaction 点中断保留旧完整输出。
- 收据修正：摘要加入原始片段、coverage、partial/truncated/gaps，不绑定检查时钟。相同输入换检查时间摘要不变；不能因相同正文而复用缺少 partial 信息的收据。保留已观察到的未知新鲜度缺口。
- 真实验证：新的独立 live-smoke fixture 通过已配置 openai-compatible/qwen3.8-flash 完成 status 工具，最新二进制 2 轮模型/1 工具/4572ms；事务补强前的一次 smoke 存 before-transaction-checks，累计两次/4 请求，未将它们合并成一次。只记录脱敏配置、run/响应 ID、Prompt/源码/二进制 hash 及结果，不记录 key 或完整模型请求。真实查询 smoke 不等于 T19 独立业务全流程。
- 最终门禁：python docs/evidence/T18/continuation-2026-10-28/verify.py 的 16 条命令 exit 0：Agent 16（config 3、tools 8、lifecycle 5），indexing 56（knowledge 7），CLI 6；关闭 Agent feature 的 1 个变体另列。Clippy 全目标 -D warnings、fmt、Schema/TS/类型边界/typecheck、Core 依赖隔离、50 冻结来源及独立整数 oracle、计划/源码/diff 通过。未将相同用例的 feature 变体或重复执行计成新的独立测试。
- Core-only：修复了关闭 Agent feature 后仍引用已删除 not_implemented helper 的分支。无 Agent 构建及实际 CLI 测试通过；最后重建默认 CLI，并用不调用模型的 configure 检查 Agent 可用。真实 smoke 的 immutable CLI hash 与默认 debug CLI hash 分别记录，后续 Cargo 链接不能替换已验证的 smoke 证据。
- 证据：docs/evidence/T18/continuation-2026-10-28/{README.md,verification-report.json,live-report.json,knowledge-roundtrip.json,knowledge-budget.json,knowledge-invalid.json,identity-import.json,tool-definitions.json,source-audit.json}。全部源码/日志/快照/185 份扫描文件与二进制哈希再次比较一致；此前 T17/T18/T19 文件作为历史保留。零字节源码 0，native/SDK/MCP/T23 与可选 SCIP 的未完成项未伪装为空成功。
- 未验证：本轮仅 Windows x64；没有业务数据库连接，未改冻结真值或宣称生产准确率。当前模拟 provider 的源片段 fixture 只验机制，T19 当前独立业务完整模型流、知识恢复/中断/冲突仍需验收。
- 下一依赖满足任务：T19（T18/T09 done）。先核对 crates/agent/src/runner.rs 的恢复与最终产物判定，以及 tools.rs 的约定变更/知识阶段，再结合既有 docs/evidence/T19 真实构建和失败记录准备当前独立真值 fixture；不得用新 smoke 或模型自评代替业务验收。T20 等任务按原依赖继续。

### 2026-10-29 — T19 独立业务闭环与恢复完成 — Codex / 真实模型与本地故障验收

- 状态：T19 doing → done；T18/T09、知识 T17 和独立真值 T06 均已 done。保留旧工作区成果，HEAD 仍为 2d1da7ca11efe95987449a8e54884a3388bc0ffe，无 Git 提交。下一项 T20 的依赖满足，未把 native/SDK/MCP/发布占位当成已实现。
- D28：恢复候选激活后继续知识；Core 提交与 run.json 之间中断只识别当前候选/hash/validation/committed activation/完整活动版本。重复恢复与完成前复核实际事实、制品及 Skill 文件。resume 的补充 goal 不重置预算；repair --with-skill 用新 run，首次候选继承已验证活动文件，case/input/脚本不经模型重复回传。
- 通用工具：关系 ID、transitions_to/joins 限定 Schema 委托 Core；主题按 kinds 聚焦。实际模型暴露知识地图用 topicId 选择无关文件的问题，改为 Core 事实实体引用，跳过 attributes 等项目元数据；增加含无关主题名/元数据 entityId 的回归。真实 partial/truncated 缺口仍强制传播，没有为通过验收删缺口或改冻结真值。
- 独立业务：冻结 T06 rules.md/flow.ts，原 truthHash 不变。实际源码生成的 Node 插件经原 case 验证/有限修复/激活，独立核对 6 状态、6 个原文 trigger/guard、2 单据关系和 3 个有序键。早期依赖错误、模型预算耗尽、Markdown 无语法轮廓导致 partial、一次验收驱动传错 runId 均保留；没有拿 case expected 或模型自评当独立真值。
- 引用复核：Core 能验证引用结构但不替代业务语义审查。此前两个 guard 声明只引用六字节 finish 名称，citation-review-before.json 明确 failed；最终两个当前主题改用包含实际 netReceived/allReceiptsValid 守卫的原文，independent-guard-support.json 逐项核对，未重写源文件。当前完整 Skill 的生成快照与文件 manifest 可独立检查。
- 最新真实 run agent_924184f7-f2b1-46c3-8f6c-7a271f5ed6c2：既有 openai-compatible/qwen3.8-flash 配置，18 轮/21 工具/0 插件修复，发布 3.0.0，两个主题与 fresh/complete Skill。激活后 update-base 被宿主拒绝并留 issue，最终完成以 Core 复核为准。所有独立 run 按 ID 保留模型轮数累计，含中断请求启动，不冒充计费统计。
- 普通增量：移除凭据变量/关闭 Agent 后，修改 netReceived 守卫、来源删除及还原均只运行已激活插件，模型计数不变；旧主题/Skill 持续 stale。负例后的 stale 是预期，样本 Skill 绑定生成时版本。
- 当前制品：Core-only CLI 执行外部 submit/prepare/validate/activate/重复激活，独立状态与无 Rig 依赖通过；最终不可变 CLI 的真实 owned PID 终止后，知识恢复模型轮数 7→11、工具 6→9、deadline 相同、一次 committed activation，SQLite integrity_check=ok。此处本地 provider 只验恢复机制，不混计真实模型业务。
- 验证：最终 Agent 21、CLI 6、indexing 57（knowledge 8）、collectors 36、lifecycle 17，共 137 passed/0 failed/0 ignored。Clippy all-targets -D warnings、fmt、Rust Schema/TS/类型边界/typecheck、Core 依赖隔离、50 冻结文件与独立数值 oracle、计划、源码、diff 通过；185 源码零字节 0。一次全工作区 900 秒超时保留 workspace-timeout.log，不计通过，不以历史工作区结果冒充本版结果。
- 完成证据：docs/evidence/T19/continuation-2026-10-28/{README.md,verification-report.json,integrity-report.json}，以及 business-04/、external-core-only-final/、subprocess-recovery-final/。源码、日志、快照和不可变二进制哈希交付前再核对；不记录 key 或完整模型请求。只验 Windows x64，无业务 DB 连接，不宣称生产准确率/三端全场景/四平台交付。
- 下一任务 T20：读取 bindings/node/src/lib.rs、packages/sdk/src/{index,client}.ts 与其 Cargo/package 配置，复用 Core JSON DTO/取消令牌和唯一线程 owner 实现 native/SDK；T21/T22/T23 继续原依赖门禁。


### 2026-10-29 — T20 native/SDK 与取消关闭完成 — Codex / Windows x64

- 状态：T20 doing → done；T15/T17/T02 已完成。保留工作区已有未提交成果，未提交 Git。下一项 T21；T22 跨端业务矩阵和 T23 四平台发布未提前关闭。
- D29：Core 统一拥有 MaintenanceRequest/Capabilities 与完整 Schema，xtask 和 native capabilities 共用生成入口；SDK 消费同源类型，查询与显式维护分开。十类查询、候选/依赖/验证/激活/回退/采集及知识维护调用已有 Core 入口，没有 JS SQLite、Rig 或项目解析规则。
- 生命周期：独立 owner 线程创建/使用/释放连接，64 个排队位；AsyncBlock 异步等待不占 libuv。ID 在返回 Promise 前登记，Core 原样使用并计算完整预算信封；SDK 防止在途 ID 重复并转发 AbortSignal。close 立即关闭接单、取消全部令牌并等待资源释放，重复调用同一 Promise；异常清理超过 10 秒返回 BUSY 并继续清理。配置路径固定，后续 chdir 不切换数据库。
- Core 补强：基础扫描/逐来源与提交、collect 来源复核/提交、激活复核/提交传播取消；原 CLI 入口保留行为。真实 SQL 语句开始后取消退出读事务；基础写事务和激活写事务中取消，generation/活动版本保持原值。
- SDK 验收：十类同库 CLI/SDK 规范化实体/关系/原文引用/缺口/分页/完整性完全一致，数据库字节和读 SQL 无写入。验证 Unicode 原文、128 字节 ID、2400 字符原始 JSON 计数，分页代际/非法字段/来源过期、pre-abort/运行中取消、重复 ID/队列满、relative config/chdir、重复 close/关闭后拒绝和 Windows 文件改名。40 个查询期间 event loop 响应；真实插件写出父/后代 PID 后 abort/close，均确认进程消失，排队时异步 fs 同样响应。无模型请求/业务数据库连接。
- 验证：Core 131（business 18/client 3/collectors 36/indexing 57/lifecycle 17），native 1，SDK 7，共 139 passed，0 failed/ignored。Clippy all-targets -D warnings、fmt、Rust/TS 同源 Schema/类型边界/typecheck、Core/native 依赖隔离、50 冻结来源与独立整数 oracle、计划/源码/diff 通过。192 源码零字节 0；只剩 T21 MCP 空导出和范围外 SCIP 等已登记后续工作，不把它们算完成。T19 workspace 超时保留，本轮不声称全工作区已回归。
- 早期失败：修正测试的 migration/result 结构假定与 SQL 前空白处理；napi metadata 改显式 host target，避免拉取无关平台依赖；汇总器改按完整包名判断，避免将 libsqlite3-sys 误认成 JS libsql。未变更外部版本或冻结真值。
- 证据与复现：python docs/evidence/T20/verify.py；最终 verification-report.json 绑定命令/日志、源码、native/loader/SDK dist/CLI 哈希，commands.json 保留每次日志。docs/sdk-usage.md 提供已实现 API/安装边界；CI 已加入真实 native 构建和 SDK 生命周期验收。
- 下一项 T21：先读取 packages/mcp/src/{index,tools,resources}.ts 与 package 配置、已锁定 MCP v2 API 探针，复用 SDK 只读/维护客户端与取消机制；不得引入平行存储/查询或让工具参数切换 mode。

### 2026-10-30 — T21/T22 补记与功能收尾核对 — Codex / Windows x64

- T21 done：stdio MCP 十类查询、14 维护工具、Core 同源 Schema、资源、模式隔离、取消与断开已实现；官方 client 两种协议协商及 3 个综合测试通过，详见 docs/evidence/T21/README.md。
- T22 done：本轮全工作区 Rust 159、SDK 7、MCP 3，共 169 个独立测试；37 个 S/X 条目、14 组三端同库响应、独立整数 oracle 与来源核对通过。实际命令/哈希见 docs/evidence/T22/verification-report.json 与 acceptance-report.json，旧 T19 超时未改。
- T23 doing、现暂缓：用户先确认 Windows x64，随后于 2026-10-30 明确要求先忽略安装。release 双 CLI 与四个 tarball 已生成；npm 安装/离线重装通过，但完整 consumer 退出 0xC0000409，未定位、未产生安装运行通过报告，不标 done。原始记录和候选路径见 docs/evidence/T23/README.md。
- 本次仅核对：202 源码零字节 0、空导出 0；唯一功能占位为首版范围外 SCIP。已记录 Core/Agent/CLI/native/SDK/MCP 功能源码与通过回归快照哈希一致；后续变更限打包/验收辅助脚本，不能把旧报告当成其最终封板。计划检查再次通过。
- 本次不继续安装、重现故障或调用模型；未提交 Git、未公开发布。功能开发可收口，已知运行异常与交付项单列保留，详见 docs/validation/closure-review-2026-10-30.md。
