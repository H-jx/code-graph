# T19 续做验收

2026-10-28 开始，2026-10-29 完成；平台为 Windows x64。保留既有工作区成果与历史证据，Git HEAD 为 `2d1da7ca11efe95987449a8e54884a3388bc0ffe`，未提交 Git。

## 实现

恢复候选并激活后继续同一 run 的知识步骤，保留模型/工具/修复预算及持久截止时间。识别 Core 激活已提交、run.json 尚未更新的窗口时，严格核对候选、制品哈希、validation、已提交 activation 和完整活动版本；不接纳其他版本变化。完成前与已完成 run 的恢复均重新核对事实、活动制品与 Skill 文件。

关系 ID 与 transitions_to/joins 限定字段 Schema 由 Core 提供。修复候选继承经过验证的活动文件，模型只发送变更；case/input 不可改写。resume 可补充 goal，repair 可 with-skill。Core 的知识地图改为沿事实实体引用选择，忽略属性对象里的同名元数据；主题名称不能改变选择范围。D28 已写入计划，无公共 wire 协议或依赖版本改动。

## 独立业务与真实模型

来源为 T06 已冻结的 `fixtures/purchase-workflow/{rules.md,flow.ts}`，独立预期保存于 business-04/independent-oracle.json，truthHash 不变。候选自带 case 不参与独立预期计算。

- 核对 6 状态、6 条带原文 trigger/guard 的流转、2 条单据关系与有序 tenant_id/purchase_no/line_no；原始字节范围和内容哈希一致。
- 最终真实修复 run `agent_924184f7-f2b1-46c3-8f6c-7a271f5ed6c2` 使用既有配置端点：18 轮模型请求、21 次工具、0 次插件修复，发布 3.0.0，并得到两个当前主题和 fresh/complete Skill。激活后尝试 update-base 被拒绝，错误保留；最终 Core 产物复核通过。
- 独立引用审查先拒绝了只覆盖 `finish` 名称的范围，失败报告为 business-04/citation-review-before.json。最终两个 guard 声明引用包含 netReceived 与 allReceiptsValid 的完整原文；详见 independent-guard-support.json。
- 移除凭据变量并关闭 Agent，修改守卫、删除来源和还原后正常增量；模型计数不增加。旧主题/导出保持 stale，不因还原原文复活。示例 Skill 绑定生成时快照，负例执行后的 stale 是预期状态。

business-01/02/03、business-04 的早期生成/知识 partial、元数据和引用错误、修订、预算耗尽均保留。一个验收驱动曾传错 runId，拒绝结果单独保存，未发起模型请求。模型累计按 durable hook 的 runId 去重计数，包含被中断的请求启动；不是服务端计费统计。没有保存 key、完整模型请求或 provider 原始错误正文。

## 恢复与外部制品

external-core-only-final/ 使用关闭 Agent feature 的不可变 CLI：submit → prepare → validate → activate → 重复 activate，并核对独立状态真值；依赖树无 Rig，模型请求为 0。

subprocess-recovery-final/ 在激活完成、知识开始前实际终止本验收启动的 CLI PID，再用本地模拟 provider 恢复。累计轮数 7 → 11、工具次数 6 → 9、deadline 不变；只有一次 committed activation，SQLite 完整性通过。此处是恢复机制验收，不冒充真实模型业务验收。

## 验证与复现

最终 Agent 21、CLI 6、indexing 57（knowledge 8）、collectors 36、lifecycle 17，共 137 passed，0 failed/ignored。另有全目标 Clippy -D warnings、fmt、Rust/TS 契约及类型边界、typecheck、Core 依赖隔离、50 冻结文件与独立数值 oracle、计划和源码检查。185 份源码零字节为 0；native/SDK/MCP/发布与范围外 SCIP 的未完成项没有被改成空成功。

一次全工作区回归在 900 秒超时，workspace-timeout.log 保留，不计通过。随后完成上述与本轮变更相关的定向回归；不宣称全工作区已通过。

```powershell
python docs/evidence/T19/continuation-2026-10-28/verify.py
python docs/evidence/T19/continuation-2026-10-28/verify.py --finalize
```

真实模型与故障驱动分别为 live_business.py、external_candidate.py、subprocess_recovery.py。重新执行真实模型应使用新的 attempt，避免覆盖证据，并会产生新的模型请求；普通 verify 不调用模型。

verification-report.json 绑定命令、日志、源码、快照、不可变 CLI 的哈希；integrity-report.json 保存交付前再次核对结果。未连接业务数据库，不声称生产准确率；三业务场景的完整 CLI/SDK/MCP 对照与四平台发布仍由 T22/T23 验收。下一项为 T20。
