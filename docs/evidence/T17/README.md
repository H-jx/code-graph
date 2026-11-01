# T17 主题、依赖新鲜度与 Skill 输出

本次从 T16 交接继续，替换 knowledge/{context,documents,skill}.rs 三个纯注释文件及 generate CLI 的失败骨架。实现及验证仅针对 Windows x64；不调用模型，不连接业务数据库，不宣称生产业务准确率。

`python docs/evidence/T17/verify.py` 是最终门禁：实际 CLI 验收、全工作区测试、Clippy -D warnings、fmt、Rust/Schema/TS 同源、类型边界/typecheck、冻结真值、源码扫描、计划和 diff 检查。只有全部命令成功才生成 verification-report.json；完整命令、退出码、匹配测试数及源码/日志/快照 SHA-256 均在报告中。

本轮全工作区运行期间复核发现 partial 原因需要强制传播到主题。最终批次 `verify.py --reuse-workspace` 只在原工作区日志哈希仍相同、且产品源码相对已保存 inventory 仅有 context 和知识测试两处变化时复用已通过的广泛回归；随后重跑完整 indexing、实际 CLI 和全部静态/契约/真值门禁。报告对复用命令显式标记 reused，并列出最终变更与覆盖。默认不加参数会从头重跑全部命令。

context 按 QueryRequest 选事实、代码地图、原始完整字节范围/有界片段及覆盖缺口，保存 Core 输入收据。topic 导入校验快照/摘要/逐声明证据与 inference/gap，拒绝伪造范围、生成知识和未展开派生依据、循环主题及无引用解释。派生约定保留原始 provenance，identityDelta 通过 expectedConfigHash CAS，返回新快照并失效索引。

依赖包括选中仓库的完整选择集合、业务批次/候选集、活动插件、配置、约定、Prompt 与模板。无关仓库变化可保留原 topic snapshot/version 并标明已复核；SQL 等相关来源变化持久 stale，改回源文件或只重渲染都不能刷新旧解释。只有当前 context 的新 topic 导入产生下一版本。依赖选取按仓库保守覆盖，后续可细化；引用可追溯不能代替独立业务真值。

Skill 用 strict Handlebars 输出 SKILL.md、conventions/routes/frontend/api/database/code-map、topics、原始引用和 index-manifest。文件写入完整版本目录并同步，COMPLETE.json 绑定 manifest，CURRENT.json 原子选择可安装 directory。失败/取消或发布前来源变更保留上次完整输出。status 核对文件哈希且不写库，不能完成磁盘核验时 unknown；仓库内输出只允许硬排除控制目录。

6 个 knowledge 综合测试覆盖：外部主题与 Markdown 链接/文件哈希、14 组伪造/循环/推断/版本/删除缺口反例及生成/派生引用拒绝、SQL stale/重导入/无关 generation、3 个取消点与渲染中来源变更、约定/身份 CAS、真实 Node 插件事实与候选集变化及带声明映射的 Mermaid。验收源码在 crates/core/tests/support/knowledge.rs，归属 indexing suite。事实、代码地图或原文读取有任一 partial 时，context 添加必保留缺口；不能把已丢失的语法轮廓包装成完整主题。

cli_verify.py 使用独立手写本地 SQL，实际执行 context→docs→skill、逐文件校验、原文回读和 stale→重导入。cli-report.json 记录所有 CLI 调用及不可变二进制路径/hash；cli-skill/ 保存完整首版输出，可直接检查。context.json、topic.json、manifest.json、freshness-timeline.json、atomic-export.json、conventions.json、plugin-dependencies.json、rejections.json 为 Core 测试快照。

早期失败日志保留：初次表名接线错误、Windows COMPLETE 文件句柄未关闭导致目录 rename 被拒绝，以及旧测试按同一时间戳中的随机 exportId 排序定位错误。已分别修正实际表连接、关闭句柄及按 exportId 核对原输出；未放宽新鲜度/哈希/原始引用断言。

源码清单见 source-audit.json；零字节文件与注释/空导出分别统计。模块 re-export、小型转发、故障注入 fixture 的空回调不当作缺失产品逻辑。Agent 仍明确声明知识/身份导入尚未接入，后续由 T18/T19 完成；native/SDK、MCP、发布验收分别保留 T20/T21/T23 未完成状态，SCIP 不在首版范围。
