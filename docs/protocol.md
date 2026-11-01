# Core 协议 1.0

唯一契约源为 `crates/core/src/contract/`。`pnpm run build:contracts` 导出 JSON Schema 2020-12、类型/错误注册表与 SDK 类型；`pnpm run check:contracts` 只比较生成物并编译类型边界用例。运行时以 Core 校验为准，TS 无法表达字符串正则或禁止所有结构性额外属性。

公共对象拒绝未知字段；CollectorOutput 的 entities/relations/evidence/dependencies/coverage/gaps 数组不可省略。其他可选字段是否允许 null 以 Schema 为准。IdentitySlot.value 省略表示缺省，显式 null、布尔和浮点数拒绝；身份值为字符串、i64 整数和递归数组。

CollectorInput.evidence 是可省略、默认空数组的原始证据导出（D23）。Core 仅导出被本次 entities/relations 断言引用，且 sourceId/contentHash 同时属于 sources 的范围记录；link 可复用这些调用、注册和实例绑定范围。它不会因此取得其他来源的内容，也不能把实体位置本身当成关联依据。link 输出仍须重新列出实际使用的 evidence，经 Core 的来源、hash、UTF-8 边界和行号校验。

candidateSets 的摘要绑定 selector、完整实体断言和相关导出关系断言；关系或候选发生变化时必须重新判定绑定。零候选有摘要，截断候选返回 CANDIDATE_SET_INCOMPLETE，不能凭剩余一个名字确认目标。显式人工原始断言派生的关联保留 declared/human；缺绑定、冲突与多 resolver 保留 candidate/inferred 和缺口。

内置实体 ID 使用计划 §3.3 的有序槽位；可选值编码为 `[]` 或 `[value]`。字符串保持原始 Unicode，不做大小写或 Unicode 归一化；孤立 surrogate 拒绝。完整 SHA-256 加 ent_/rel_/src_/ev_ 前缀。字节位置基于原始 UTF-8，区间左闭右开，行号一基。关系 qualifiers 的对象键按 UTF-8 字节序递归排序并参与 ID（Node 不使用默认 UTF-16 排序），位置和生产者不进入身份。RelationRef 由 snapshot 与 relationId 构成。

扩展由 PluginManifest.extensions 登记：entities 包含 name、identityComponents、attributesSchema；relations 包含 name、allowedFrom、allowedTo、qualifiersSchema。名称为 `x.<pluginId>.<localName>`，插件只能登记自己的命名空间；同名实体/关系或重复登记拒绝。身份组件必须有序、唯一，以 required scopeId 开始。

扩展实体规范身份为 `[projectId,kind,[[slotName,[]|[value]],...]]`，每个槽位都带名称和存在包装，required 仅决定校验规则。从持久身份恢复槽位无需执行插件或读取业务 Schema，登记变化不会无声改变编码。扩展关系沿用通用关系身份；内置身份格式不变。

载荷 Schema 必须描述 object，最多 256 KiB/64 层；允许自身或 bundled `$defs` 引用。Core 的 Entity/Relation/Source 等定义可引用但不可覆盖，插件可提供其他本地 `$defs`。远程/文件 `$ref`、`$id`、动态引用、非 2020-12 方言拒绝。类型声明的未知字段拒绝，不能重定义 basis、origin、位置或公共引用语义。

结构 Schema 接受语法正确的扩展名称；是否登记由当前候选组与未替换的活动 manifest 共同决定。CollectorInput.allowedExtensions 由 Core 设置。输出必须满足登记身份、attributes/qualifiers 与端点约束，并继续执行公共 ID、作用域、证据、来源和 AI basis 校验。未登记扩展返回 SCHEMA_INVALID 及精确 kind 路径，不写入正式库。

通用持久化、搜索和重新导出保留扩展 kind，业务查询不会据此猜测语义。扩展 smoke 属协议一致性测试，不代表独立业务准确率。

lexicalPath 和 normalizedSignature 生成规范见 `crates/core/src/syntax/` 与计划 §3.3：语法 token 去空白/注释，保留标识符/字面量；匿名节点包含结构路径，不支持语法报告 gap。共享向量见 fixtures/protocol/identity-vectors.json；非法结构与未登记扩展分别验收。

T15/D24 的 `neighbors` 接受单根 EntityRef、direction、relationKinds/filters/pageSize/continuation/budget；与 trace 分开。邻接 keyset 绑定完整请求 hash、indexId/generation 和最后 `(direction,kind,relationId,assertionId)`；search 使用精确等级、BM25、实体和断言的稳定键。修改请求拒绝，跨代/跨库返回 SNAPSHOT_EXPIRED。BFS 只返回有界 frontier 和 nextQueries，不提供恢复队列的 token。

查询入口使用独立读事务和 query_only，取消令牌同时检查 Core 循环和 SQLite VM；不刷新索引、不发现插件、不调用模型。verify 在同一预算内复用来源选择扫描，确认不完为 unknown/partial，发现磁盘变化时默认不返回旧事实；indexed 不宣称磁盘 fresh，allowStale 明确标注历史。readSource 必须原样传回存储中的原始引用，篡改范围/版本拒绝，过期来源只有 allowStale 才返回历史片段。

responseChars 统计完整紧凑 JSON 的 Unicode scalar 数（含 envelope），超限删除完整投影并标注 partial；不裁断序列化 JSON，也不保留跳过已删除结果的游标。最小 envelope 无法容纳返回 BUDGET_EXHAUSTED。CodeMap 按真实入口、直接依赖、同文件原始证据选择声明轮廓和签名，仓库与作用域隔离，不以路径长度判断业务正确性。

T16/D25：分页请求摘要使用 Core canonical_json，不依赖 serde_json 是否开启 preserve_order；字符裁剪在数组长度并列时以字段名稳定决策。信封预留固定时间/计数宽度，实际 responseChars 仍是返回值的真实紧凑 JSON 字符数。readSource 额外给出 evidenceId/startByte/returnedEndByte/snippetTruncated，预算内的部分原文仍可标为 current；过期与片段裁剪分别表达。

page 仅用原始登记的 router mode、大小写敏感的路径/alias 匹配；参数模式是候选，不猜框架优先级。RootFilter 与 TraversalFilter 分开，页面链路只顺已有调用/绑定/读写事实扩展，服务或表的共同归属不扩成所有查询。workflow 的业务 transitions/precedes/documentLinks 与可观察 codeCalls 分开；report 逐生产者返回已有实现、字段 lineage、joins/grain/aggregates/states/NULL/units 及 unknowns，不执行用户 SQL。locateFeature 可有界地按显式词项召回，返回 matchedTerms 和 PARTIAL_QUERY_MATCH，不把整句需求变成新事实。

T17/D26：TopicContextRequest、TopicContext、TopicArtifact、TopicClaim、KnowledgeDependency 和 ConventionsImport 从 Rust 同源导出。context 是显式维护入口，保存 Core 收据；inputDigest 绑定 snapshot/materialDigest，材料含查询事实、原始范围/片段、代码地图、coverage、派生约定及 sourceSet/facts/plugins/batch/selectors/config/prompt/template 依赖。输出大小含整个 context，无法容纳必要元数据则 BUDGET_EXHAUSTED；不把被裁剪材料当完整证据。

导入主题必须匹配收据和当前 snapshot，逐 claim 验证原始证据；AI origin 必须 inferred，inferred 需 gap，原始 context gaps 不可删除或改写。正文每个说明行用 `[[claim:id]]`，Mermaid 每个节点/边行用 `%% [[claim:id]]` 映射。标题外不接受无引用解释，链接由渲染器生成。generatedKnowledge/未展开的 derivedConvention/topic 引用不能充当原始来源。ConventionsImport 用 expectedConfigHash 做配置 CAS，绑定依据必须是导入的原始 evidence；返回变更后 snapshot，索引事实单向失效。保存的派生约定保留原始 provenance，新 context 可在内容未变且原始证据复核后附带当前引用。

主题保留原生成 snapshot/version。无关 generation 变化复核输入依赖后可 fresh；相关来源/候选/插件/配置/Prompt/模板变化持久 stale，恢复旧文件或单纯重渲染不恢复解释。status 的 knowledge 是只读依赖和输出完整性核验，预算不足/未查磁盘为 unknown；不更新数据库状态。当前依赖选择保守覆盖选中仓库，无生产业务准确率外推。

Skill 输出含 SKILL.md、全部 references、topics、index-manifest 和 COMPLETE.json。写入 `<output>/versions/<exportId>/` 完整版本，临时文件逐项同步后原子替换 CURRENT.json；返回 directory 指向可安装 Skill。manifest 记录原 topic snapshot/input/Prompt/template/来源依赖、活动插件与逐文件 SHA-256，COMPLETE.json 绑定 manifest hash，避免自哈希循环。发布前重验输入；失败保留旧 CURRENT 和完整版本，尝试记录为 failed 或 writing（宿主中断后的未完成尝试）。仓库内输出只允许既定硬排除目录 `.agents/skills/`、`.project-knowledge/`，生成内容不回流采集。
