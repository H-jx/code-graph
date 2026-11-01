`api_ast.py` 提供示例 Python API 注册解析。`source_ast.py` 与 `database_ast.py` 提供 T12 的独立数据库插件逻辑，由打包脚本复制进 A/B 制品。

数据库采集仅读取 Core 提供的来源快照。SQLGlot 30.24.0 使用显式 SQLite 方言解析 DDL/SELECT/CTE，不使用执行器或优化器；Python 使用 stdlib AST，TypeScript 使用锁定的 py-tree-sitter 0.25.2 与 TypeScript grammar 0.23.2。依赖由 Core prepare 安装并校验 hash-lock，普通采集只读已准备缓存。

支持的样例模式：顶层显式 `orm.connect(dataSourceId)`、`instance.model(table, orderedPrimaryKeyColumns)`，具备实际相对 import/词法绑定的 `instance.query(staticSql)` 和 `model.findAll()`。SQL 静态变量允许追溯原始声明，参数/局部变量遮蔽、重复绑定、缺失 import、动态实例/SQL 和未调查的方法返回 gap。仅凭同名 `erp`、表名或字段名不会关联。

DDL 声明产生按 dataSource 隔离的表、列和有序主外键。连接保存原始条件、left/right/inner 类型和有序列对；不完整键不生成 confirmed joins，基数保留 unknown。嵌入 SQL 使用原始调用/声明完整范围作为 UTF-8 证据，不将解码后的 SQL token offset 冒充源码位置。

业务粒度、有效状态、时间、NULL 业务策略、单位和聚合口径仍由 T14 独立采集；T12 明确返回 unknown 与 gap。当前插件只用于源码审阅过的合成样例，真实项目适配规则由调查后生成的项目制品提供。

T14 的 semantic_facts.py、workflow_ast.py 和 report_ast.py 为 workflow/report 两阶段生成原始流程、代码状态写入、文档单据键和既有查询字段 lineage。独立 policy 缺失、无法证明的状态所有者/必要过滤/粒度/基数/单位均保留 gap；没有 SQL 执行器或代码求值。规则范围见 docs/collectors.md；source_ast.py 与 Node AST 的参数签名按语法 token 保留字面量，去除参数间格式/注释。
