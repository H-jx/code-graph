# T18 续做验收

从 T17 交接继续，先核对其源码哈希及 T18/T19 历史模型记录。旧记录保留为对应版本证据；本轮完成 Agent 与 Core 原始约定、身份和知识工具的接入。

`import-conventions` 保存经过 Core 校验的 derivedConvention 和原始 provenance。身份增量必须有当前 expectedConfigHash，错误或缺少哈希均不写配置；变更后要求 update-base 和重新调查。Agent 不直接执行 SQL、写业务源码或实现查询算法。

`--with-skill` 才注册 export-topic-context / import-topic / export-skill。工具只使用当前 run 导出的 Core 输入收据，逐条验证主题声明与引用；缺口必须保留。工具共用原来的模型轮数、工具次数、修复预算和截止时间。激活后缺主题或 Skill 的状态为 partial/KNOWLEDGE_INCOMPLETE；只有 Core 返回 fresh/complete 的完整 Skill 才能完成。

KnowledgeControl 把剩余时间和取消传到 Core 查询及提交边界。保存收据、主题和导出记录在取得 SQLite 写事务后与 commit 前复核，避免等待写锁后继续提交已经取消的请求。SQL trace 的精确同步点验证查询中、BEGIN IMMEDIATE 时取消，不依赖任意 sleep。Skill 在 preflight、文件写入、发布及事务边界可中断，旧 CURRENT 和完整输出保留。

上下文摘要绑定事实、原始片段、coverage、partial/truncated 与 gaps，检查时钟不参与摘要；相同输入换检查时间的收据仍一致。Prompt 使用完整 shared/investigate/collector/link/repair/knowledge 顺序，实际工具名、allowlist、Schema hash 和配置/作用域导航均按现有协议提供。Core-only 的 CLI Agent 分支已修复旧辅助函数引用。

`python docs/evidence/T18/continuation-2026-10-28/verify.py` 的最终 16 条命令全部 exit 0：

- Agent 16 tests：config 3、tools 8、生命周期 5；涵盖知识完整流、精确错误 Pointer、错误/遗漏 CAS、统一预算、两次修复上限、取消、来源变化、pending activation 和恢复。
- indexing 56 tests（其中 knowledge 7），CLI 6 tests；另执行关闭 Agent feature 的 CLI 1 个变体用例。变体单列，不把它计作新的独立用例。
- 全目标 Clippy -D warnings、fmt、Rust Schema/TS/类型边界/typecheck、Core 依赖隔离、50 冻结来源与独立算术 oracle、源码清单、计划和 diff 检查。
- 最后重新构建默认 CLI，并用不请求模型的 configure 检查确认 Agent feature 可用。真实 smoke 使用的不可变二进制和当前默认 debug 二进制分别记录哈希，未混用验收制品。

真实模型验收通过 live_smoke.py 执行。读取已授权的根 .env，未保存凭据或完整模型请求。当前配置为 openai-compatible / qwen3.8-flash，事务检查补强后的最新二进制用 2 轮模型请求完成 1 次 Core status 工具调用，约 4.6 秒。此前 smoke 存在 before-transaction-checks/；本轮两次 smoke 合计 4 次真实模型请求，报告分别保留各 run 和响应元数据。此证明仅覆盖当前端点的查询工具，不替代 T19 独立业务全流程验收。

verification-report.json 记录完整命令、退出码、匹配数量、工具版本、源码/日志/快照 SHA-256、模型响应 ID/耗时及二进制路径/hash。knowledge-roundtrip.json、knowledge-budget.json、knowledge-invalid.json、identity-import.json、tool-definitions.json 是本地模拟 provider 经实际 Core 执行的结果；Fixture 只证明机制，不冒充业务真值。

扫描仍为 185 份源码、零字节 0。native/SDK、MCP、发布验收和范围外 SCIP 保留既有阶段归属。T19 的当前项目独立真值、真实完整构建/知识与恢复故障验收仍待完成。本轮 Windows x64，无业务数据库连接；未修改冻结真值，未宣称生产准确率或四平台交付。
