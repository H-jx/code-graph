---
name: project-query
description: 在已建立的项目索引中定位页面、链路与取数口径；只读查询，不生成或执行新脚本
---

# 项目事实查询

先使用已安装 CLI：`project-knowledge status --config <project.json> --check-freshness`。
查询使用 `project-knowledge query --config <project.json> --request <request.json>`，参数来自 Core QueryRequest Schema。
其他入口以实际 capabilities 为准；维护操作由 project-onboarding 处理。

## 调用顺序

| 步骤 | 调用 | 目的 |
| --- | --- | --- |
| 1 | schema 资源、`index_status`（checkFreshness） | 了解实际能力、索引版本与缺失采集范围 |
| 2 | `locate_feature`（需求 + 可选 routeHint） | 找候选入口、核心业务方法、链路与关联代码 |
| 3 | `search` / `get_code_map` | 按标题、模块、作用域与方法轮廓缩小范围 |
| 4 | `trace_links`（原样传回实体引用） | 看谁调用它、它继续调用谁、数据读写与共享依赖 |
| 5 | `read_source` | 精读关键方法、条件、SQL / ORM 与调用点 |
| 6 | 整理答案 | 按代码职责说明位置、关联原因、证据与未确认项 |

## 结果判读

- 先看 `completeness`：freshness、相关采集覆盖、候选集完整性、预算截断。
- `gaps` 说明缺什么能力、动态值、歧义或范围排除，不要把缺口当结论。
- `basis=inferred` 的关系需要源码核对后才能引用为事实。
- 版本过期返回 `expired`：重新查询，不要在旧快照上继续解释。
- `data.knowledge.topics` 保留主题原始 snapshot/version；`dependenciesRechecked=true` 表示当前依赖复核通过。
- Skill 输出由 `CURRENT.json` 指向完整版本目录；先核对 `COMPLETE.json` 和 `references/index-manifest.json` 中的文件哈希。
- 重渲染保留过期主题的 stale 状态；重新 export context/import docs 后才有新的解释版本。

## 回答格式

先指出核心业务实现，再解释前后端链路与关联文件，最后列出需核对的条件与断点。
每项附文件路径、行范围与依据；查询本身不修改业务代码。
