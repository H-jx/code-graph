# T14 业务流程与台账语义采集

前置 T13 已完成。新增 A/B 独立 semantics 制品，workflow extract 与 report link 两阶段经 Core prepare、运行、case、验证、整组激活和无模型增量；没有在 SDK/Core 放置采购适配规则。完整命令与源码/制品/日志 SHA-256 在 verification-report.json；复现为 `python docs/evidence/T14/verify.py`。

## 实际验收

- 每布局：6 个状态、14 条独立保留的文档/代码转换断言、2 个有原文的页面先后关系、2 条含 3 个有序键对的单据关系；gross/net 冲突保留两种依据与 CONFLICTING_EVIDENCE。
- 缺文档时保留代码操作、状态写入和明确 calls，transitions/precedes 不再输出。错误对象的 state 写入返回 WORKFLOW_STATE_OWNER_UNKNOWN；参数重新绑定也不能被确认成业务状态。
- 正确 SQL 的 9 个字段逐项匹配独立审阅的完整原始列集合，含 CTE 预聚合后的 receipt/return/invoice lineage。粒度 3 键、3 个实际聚合、状态过滤、NULL 回填和明确货币单位均有源码/policy 依据。
- 原始 fanout、LEFT JOIN 后 WHERE 子表过滤、遗漏 tenant/文档键、错有效状态、缺删除条件、错误非零回填均有具体 gap。删除 policy 后 grain/validStates/nullHandling/timeFields/units 返回 unknown，仍保留可证明的表达式和原始字段事实。
- 时间字段、未声明的物理数量单位与不能证明的基数仍是缺口，不给默认值。业务单据顺序来自文档，不从最短调用路径推导。

独立补充来源、原文行范围、输入 hash、手工审阅的字段 lineage 位于 fixtures/truth-supplements/T14-semantics.json，reviewHash `4dea260f6c096557bf40f0bacf10ee762f21b02cfedd6d6a5d7024ca7aea698f`。T06 的 50 个冻结文件和 truthHash 保持不变；仅为源码审阅的合成验收，不声称生产口径正确或人类签认。

## 验证门禁

按 T14 计划执行 workflow/report 插件联合入口与 semantics_ 综合用例；同时回归受共享 TS 参数规范化影响的 frontend_ 用例，并运行全工作区 Clippy、fmt、Rust Schema/TS 同源、冻结真值与独立整数 oracle、计划校验。全部实际结果以 verification-report.json 为准；本轮没有重跑与插件改动无关的整套工作区测试。T13 的全工作区 131 passed 证据作为历史版本保留。

新增的参数 token 规范化保留字面量内容，跳过格式和注释。负例同时验证 API 与 workflow 生产者在参数插入注释/换行后仍指向同一 operation ID；不把格式差异当作方法改名。guard 证据覆盖原始函数/条件范围，不仅引用赋值位置。

工具与范围沿锁定 Rust 1.97.1、Node 24.14.0、插件 Python 3.12；本轮 Windows x64。普通采集模型调用 0、产品模型请求 0、业务数据库连接 0。SQL 仅解析；数值真值继续由独立 T06 oracle 检查。场景查询组合为 T16，四平台交付为 T23，未提前声明完成。
