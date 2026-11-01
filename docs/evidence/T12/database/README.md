# T12 数据库采集闭环 — 2026-10-28

T12 已完成；前置 T11 保留已验收证据，下一可执行任务为 T13。工作树基准 `2d1da7c`，沿用此前未提交成果。所有源码、制品、独立审阅补充包和日志哈希见 `verification-report.json` 与 `final-checks.json`。

## 实际能力和范围

- A/B 两独立 Python 制品，Core prepare → run → case → validate → staging → commit；模型请求和业务数据库连接均为 0。
- SQLGlot 30.24.0（严格 ErrorLevel.RAISE、显式 SQLite 方言）、Python stdlib AST、py-tree-sitter 0.25.2 / TypeScript grammar 0.23.2。完整 wheel hash-lock 与官方元数据保存在制品和 dependency-metadata.json。
- 每布局独立审阅的 90 个 DDL expected：2 个数据源、10 张表、62 列、16 个有序主外键。正确查询的 4 组 LEFT JOIN 保存完整有序键，CTE 的直接投影按实际源列绑定，基数不作推断。
- 静态 SQL/ORM queryImplementation；明确词法/相对 import 的实例和模型映射；source/statement 与转义 SQL 都引用原始 UTF-8 范围。findAll 记录现有 ORM 实现，不生成 SQL。
- 不同数据源的同名表隔离；动态 SQL/实例、缺 import、参数/局部变量遮蔽、未调查 ORM、复合键缺列、坏 SQL、无数据源的 SQL 均有反例。旧业务断言被新批次正确替换。
- T14 的业务粒度、有效状态、时间、NULL 策略、单位、聚合口径仍 unknown+gap。原始样例中无 import 的 erp 全局调用保持 UNBOUND_INSTANCE；没有用目录或同名变量猜绑定。

T06 冻结包未改动，truthHash 仍为 `aaf1fd10b75902583be1108b083e6d3a2f71cb515f633b116ca71967f780b1ed`。补充样本与逐行审阅/文件 hash 位于 fixtures/truth-supplements/T12-database；reviewHash 为 `c59bfa400fe1e49f2713e80374d1d97e9c5aa85e1744484c238578564d4ef6c6`。这是独立源码审阅的合成 smoke，不声称人类签认、生产准确率或四平台验收。

## 验证

工作目录均为 `E:/Project/my-project/code-graph`。运行 `python docs/evidence/T12/database/verify.py` 重放主要验证；`final-checks.json` 保存另行执行的 typecheck、版本、打包与最终状态核对。

| 命令 | 退出码 | 结果/日志 |
| --- | --- | --- |
| node scripts/verify-plugins.mjs --capabilities api,database | 0 | 两语言 API + 两布局数据库综合验收；plugins.log |
| cargo test --workspace --locked -j 1 -- --test-threads=1 | 0 | 129 passed / 0 failed / 0 ignored；workspace-tests.log |
| cargo clippy --workspace --all-targets --locked -j 1 -- -D warnings | 0 | clippy.log |
| cargo fmt --all -- --check | 0 | fmt.log |
| cargo run -p xtask --locked -- export-contracts --check | 0 | 5 文件一致；contracts.log |
| node scripts/generate-types.mjs --check | 0 | types.log |
| node scripts/verify-contract-types.mjs | 0 | 7 正例 / 8 负例；type-boundaries.log |
| pnpm run typecheck | 0 | SDK/MCP；typecheck.log |
| python scripts/verify-truth.py | 0 | 50 冻结文件与独立整数 oracle；truth.log |
| node scripts/verify-plan.mjs --plan ../plan/NEW-PROJECT-LONG-TASK.md | 0 | 24 项任务依赖与状态；plan.log |
| node scripts/audit-source.mjs --output docs/evidence/T12/database/source-audit.json | 0 | 源码哈希和未实现位置；source-audit.json |

Windows x64：Rust/Cargo 1.97.1、Node 24.14.0、pnpm 10.34.8、插件 Python 3.12.13。没有运行原项目服务、SQL 或修改原项目；原项目工作树只读复核为干净。未进行本轮 Linux/macOS 运行或远程 CI，平台交付仍由 T23 验收。

## 本轮纠错

Core 原来对 maps_to/transitions_to/relates_document 使用 camelCase Serde 输出，与计划、身份计算和登记表不一致。现在显式按既定 snake_case 序列化，并在既有 contract 测试逐项验证所有关系的 wire/as_str/往返一致；Schema/TS 已从 Rust 重生，没有另建协议。

首次数据库 Core 验证因上述契约错误失败（backend-schema-failed.log）。随后坏 SQL 反例发现 SQLGlot error_level 传字符串没有启用严格错误，已改用 ErrorLevel.RAISE（backend-parse-failed.log），最终两综合用例与全量回归通过。直接探针初次还遇到宿主 Python 默认 GBK 与可选身份槽 undefined；已修正 UTF-8 探针及省略可选槽位，不放宽 Core 校验。正式业务库未提交失败输出。

## 空文件与未实现源码核对

扫描 151 个源码文件，排除依赖/构建产物和 `.gitkeep`：零字节源码 **0**；只有注释 **9**；只有空导出模块 **5**。

| 位置 | 数量 | 对应任务 |
| --- | --- | --- |
| core graph/{neighbors,paths} 与 codemap/{rank,render} | 4 | T15 |
| core knowledge/{context,documents,skill} | 3 | T17 |
| bindings/node/src/lib.rs 与 SDK index/client | 3 | T20 |
| MCP index/tools/resources | 3 | T21 |
| core syntax/scip.rs | 1 | 本版范围外的可选能力 |

query 的 feature/page/workflow/report 仍使用场景骨架，CLI generate 和发布验证也有明确未实现入口；这些属于未完成任务。没有通过补空返回值或修改完成状态掩盖缺失逻辑。完整行号、标记和哈希在 source-audit.json；自动检测结果不能代替语义验收。
