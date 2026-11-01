# T02 — 契约闭环

2026-10-26：done。当前报告 verification-report.json 记录生成物/fixture 哈希。

- workspace-tests.log：92 passed（CLI 3、collectors 33、indexing 45、lifecycle 11），0 failed。clippy.log：-D warnings 通过。
- collectors-tests.log：扩展登记、extract/link/case/激活/搜索/再导出/活动登记消费，以及载荷/端点/证据拒绝实测。未知扩展的结构名称合法，语义登记单独拒绝。
- identity-vectors-node-report.json / identity-vectors-python-report.json：21/21；同一 fixture 的 Rust 检查随 indexing 执行。原 18 条未改，新增扩展缺省/空串和关系。
- contracts-check.log：5 个 Rust 生成文件与 TS 重生成一致；类型边界正例 7、负例 8；typecheck.log 编译通过。
- probes.log：重跑原生/MCP/Rig/Python 集成，4 个 Rust、5 个 Node、21 条 Python 向量通过。
- export.log：2020-12 Schema 与所有公共 DTO/RelationRef/业务载荷导出。json-schema-to-typescript 16.0.0 只在 object 根处理 unreachableDefinitions，使用无字段生成锚点导出全部 Core 定义；不维护平行模型。TS 的正则/整数/结构性额外字段限制仍由 Core 运行时校验。

实施细化见 docs/protocol.md 与计划 §3.3。IdentitySlot.value 省略表示缺省，null/布尔/浮点拒绝；扩展身份带槽位名和存在包装，内置编码不变。

初次扩展回归发现所有权迁移时误改 SQL 中 kind，已还原并由全量存储/采集/激活回归覆盖；初次搜索断言路径应使用 entityRef.entityId，已修正测试。未放宽产品校验。扩展 smoke 不作为独立业务准确率。
