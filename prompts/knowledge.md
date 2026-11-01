# 生成业务知识与 Skill 材料（knowledge）

运行时输入：主题、`generate context` 导出的事实与代码地图、源码片段、
> 证据引用、覆盖缺口与预算。

## 目标

围绕用户指定的业务主题和当前索引证据产出跨层说明，交给 Core 校验与渲染。

## 产物结构

- Markdown 正文：结论、链路、页面操作、后端与数据、取数口径；
- 使用的来源引用列表（原样回传 `readSource` 得到的引用，不手工拼 ID）；
- 推断项清单与未知项清单；
- 可选 Mermaid 图（节点必须对应真实实体）。

## 要求

1. 每条结论都能追溯到文件或行范围；只能从代码顺序推断的流程标为推断。
2. 台账口径先找已有 SQL / ORM 查询，说明字段来源、粒度、聚合与关联条件，输出待验证方案。
3. 状态变更与流程资料缺失时明确写“未采集”，不要用调用顺序冒充流程顺序。
4. 引用的来源版本必须与 context 快照一致；过期则返回过期而不是改写事实。
5. 输出 Core TopicArtifact：原样保留 protocolVersion/topicId/snapshot/inputDigest/promptHash/templateHash；保留 context 的全部 gaps。
6. claims 逐条给出 id/text/basis/origin/evidenceRefs/gapIds。observed/declared 引用 context.sourceReferences 中的原始 evidenceId；AI 判断使用 origin=ai、basis=inferred，并写 gapIds。
7. 正文每个解释行使用 `[[claim:id]]`。Mermaid 节点和边的每行在 `%% [[claim:id]]` 注释中映射声明。标题可不带引用；不嵌入 HTML、额外链接或其他代码块，Core 渲染原始引用链接。
8. 引用有效只说明可追溯。不要改写独立真值、补造未知单位/状态、引用其他 topic 或生成知识自证。
