---
name: project-onboarding
description: 调查目标项目约定，生成路由、前端、API、数据库与关联采集插件及验收样例，并推动验证与激活
---

# 项目接入与提取器生成

外部生成器通过 Core 的同一套公共协议提交调查材料与插件。开始前读取实际能力与当前索引状态。

## 适用时机

- 目标项目只有基础索引，需要建立业务链路索引；
- 框架约定变化导致采集失败，需要修复插件；
- 需要新增采集能力（注释、业务模块、定时任务、消息消费等自定义入口）。

## 流程

1. **读协议**：取当前契约与关系谓词（`getCapabilities` / schema 资源），确认索引状态与新鲜度。
2. **调查约定**：按 `prompts/investigate.md` 的清单核对路由、请求封装、接口注册、数据源与 ORM 映射。
3. **发布约定**：调用 `project-knowledge generate --config <project.json> conventions --input <conventions.json>`。
   ConventionsImport 包含 expectedConfigHash、notes、sourceReferences 和可选 identityDelta；原始引用必须来自当前索引。
   Core 以 CAS 保存派生约定及其原始依据，返回新快照；身份变化后刷新基础索引。
4. **生成候选**：在 `.project-knowledge/work/candidates/<run-id>/` 产出脚本、manifest、依赖声明、验收样例
   （见 `prompts/collector.md` 与 `references/extractors.md`）。
5. **验证**：`validate-plugin` + `run-plugin-cases`，Core 检查协议、身份、证据位置、依赖与引用。
   使用 get-capabilities 的 relation Schema 核对严格限定字段；identity 可计算实体或关系 ID，避免猜测 case 的预期哈希。
6. **修复**：失败时按 `prompts/repair.md` 在预算内重试；不绕过失败。
7. **激活**：验证通过后发布到不可变版本目录，由 storage 单事务切换活动版本并替换该插件负责的事实。
8. **核对**：页面能否连到正确服务；不同数据源同名表是否被串联；动态目标是否保留缺口；
   新增候选或输入文件是否会触发重新关联。
9. **知识产物**：`generate --config <project.json> context --topic <id> --request <query.json> --output <context.json>`，
   按 TopicArtifact 填写外部解释，再 `generate --config <project.json> docs --input <topic.json>`；
   最后 `generate --config <project.json> skill --name <name> --output <control-directory>`。
   仓库内输出使用 `.agents/skills/` 或 `.project-knowledge/`；返回 directory 是完整 Skill，CURRENT.json 原子选中版本。
   内置 Agent 使用 `agent build --with-skill` 时，激活后仍需在同一 run 调用 export-topic-context、import-topic、export-skill。
   中断后用 agent resume 继续，预算和截止时间不重置；pending_activation、partial 或缺少完整 Skill 均不等于接入完成。
   repair --with-skill 可沿已验证的活动文件创建新版本并继续知识步骤；只提交修改文件，保留原 case/input。声明必须引用包含该内容的原文范围，引用函数名不证明正文守卫。

## 边界

- 只写候选目录与约定草稿，不改目标业务源码。
- 生成的验收样例与自身输出一致，只能证明内部一致性，不能当作精准度提升的独立证据。
- 直接由模型判断的关系保持 `inferred`；AI 生成静态脚本不等于脚本的业务判断已通过验收。
