---
name: project-onboarding
description: 调查目标项目约定，生成路由、前端、API、数据库与关联采集插件及验收样例，并推动验证与激活
---

# 项目接入与提取器生成

> 骨架文件。与内置 Rig Agent 使用同一套协议（`prompts/shared.md`）和模板（`prompts/`）。
> 外部编码 LLM 按本文流程操作，产物入口与内置 Agent 完全一致。

## 适用时机

- 目标项目只有基础索引，需要建立业务链路索引；
- 框架约定变化导致采集失败，需要修复插件；
- 需要新增采集能力（注释、业务模块、定时任务、消息消费等自定义入口）。

## 流程

1. **读协议**：取当前契约与关系谓词（`getCapabilities` / schema 资源），确认索引状态与新鲜度。
2. **调查约定**：按 `prompts/investigate.md` 的清单核对路由、请求封装、接口注册、数据源与 ORM 映射。
3. **发布约定**：把带依据的调查材料交给 `import-conventions`，写入 `.project-knowledge/conventions.md`
   并登记为 documentation 来源；后续生成使用登记后的来源快照。
4. **生成候选**：在 `.project-knowledge/work/candidates/<run-id>/` 产出脚本、manifest、依赖声明、验收样例
   （见 `prompts/collector.md` 与 `references/extractors.md`）。
5. **验证**：`validate-plugin` + `run-plugin-cases`，Core 检查协议、身份、证据位置、依赖与引用。
6. **修复**：失败时按 `prompts/repair.md` 在预算内重试；不绕过失败。
7. **激活**：验证通过后发布到不可变版本目录，由 storage 单事务切换活动版本并替换该插件负责的事实。
8. **核对**：页面能否连到正确服务；不同数据源同名表是否被串联；动态目标是否保留缺口；
   新增候选或输入文件是否会触发重新关联。

## 边界

- 只写候选目录与约定草稿，不改目标业务源码。
- 生成的验收样例与自身输出一致，只能证明内部一致性，不能当作精准度提升的独立证据。
- 直接由模型判断的关系保持 `inferred`；AI 生成静态脚本不等于脚本的业务判断已通过验收。
