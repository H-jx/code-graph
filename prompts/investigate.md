# 调查项目约定（investigate）

运行时输入：项目目标、文件范围、基础索引状态、代码地图、代表性入口、当前契约与预算。

## 目标

调查真实的路由、前端请求封装、后端注册与数据库映射，产出项目专属提取规则。

## 要确认的内容

| 对象 | 需要确认的项目规则 |
| --- | --- |
| 路由 | hash/history、注册表或文件路由、菜单、别名、页面映射、title / i18n 来源 |
| 前端 | 组件事件与方法、Hook、请求 wrapper、baseURL、代理、生成客户端 |
| API | 服务边界、HTTP / GraphQL 注册、Handler、实例绑定、业务方法调用 |
| 数据库 | 数据源、ORM 模型与表列映射、复合键、查询与关联条件 |
| 注释与入口 | 注释与符号绑定、业务别名、模块边界、定时任务 / 消息消费 / 命令等真实入口 |
| 业务 | 状态与操作、前后单据键、取数粒度、有效状态、已有流程资料 |

## 产物

- 调查约定：每条规则附原始来源引用与推断标记，交 import-conventions 由 Core 登记；
- 作用域与数据源清单；
- 待提取能力清单与覆盖缺口。

## 方法

先用 `search` / `code-map` 缩小范围，再用 `export-snapshot` 取得 sourceId、`read-source` 精读；从用户指定目标在本项目的实际入口逐跳核对，
不要只看目录名或关键词重合就下结论。

调查结果是生成本项目插件的输入。示例工程仅用于说明协议和验收方式；项目身份、注册方式、目录和业务名称都必须由当前来源确认，不复制示例预设。

身份增量只登记有明确原始依据的 scopes/services/dataSources/bindings，绑定 evidenceRefs 来自当前导出引用。
传 identityDelta 时附 get-capabilities 的 expectedConfigHash。Core 返回 identityChangesApplied=true 后先 update-base，再重新导出和核对原文。
