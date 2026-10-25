| 功能 | 精简描述 | 使用什么工具实现 |
| --- | --- | --- |
| 项目接入 | 接入本地前后端仓库，选择源码、配置、SQL 和业务文档范围。 | Rust、ignore、globset；[Repomix](https://repomix.com/guide/usage)（可选材料打包） |
| LLM 配置 | 为内置 Agent 配置模型提供商、模型、兼容端点、凭据环境变量及运行预算。 | Rust 配置、Rig 模型客户端、环境变量 |
| Prompt 模板 | 提供通用调查、采集、关联、修复和知识生成模板，允许补充项目业务规则。 | Markdown、Rust 模板加载、当前协议与 JSON Schema |
| 项目约定调查 | 调查实际路由、请求封装、接口注册和 ORM 用法，生成项目专属提取规则。 | Rust、[Rig Agent](https://rig.rs/docs/concepts/agent)、Core 工具；支持外部 LLM |
| 提取器生成 | 根据项目规则生成采集脚本，覆盖路由、前端、API、数据库及自定义能力。 | Rig Agent、Node.js／Python 等运行时、JSON Schema |
| 业务定向采集 | 按项目目标提取注释、业务模块、定时任务、消息消费等特定入口，输出统一格式。 | 通用 Prompt、项目专属插件、源码证据 |
| 插件验证与修复 | 试运行候选插件，核对协议、证据和验收样例，将结构化错误交给 Agent 有限修复。 | Rust 校验器、受控子进程、Rig 工具调用 |
| 插件自动加载 | 验证通过后自动登记和激活版本，后续索引自动加载；失败保留上一版本，来源变化时标记过期。 | Rust 插件宿主、制品哈希、SQLite 活动版本与事务 |
| 路由提取 | 提取 URL、hash/history 模式、别名、页面文件、标题和国际化来源。 | 项目路由提取脚本、配置解析、Tree-sitter |
| 前端提取 | 提取组件、按钮/事件、处理方法、fetch/axios/GraphQL 请求及封装关系。 | 项目前端提取脚本、Tree-sitter、ripgrep |
| API 提取 | 提取接口方法、路径、目标服务、Handler、Resolver 和业务方法调用。 | 项目 API 提取脚本、Tree-sitter、[SCIP](https://github.com/scip-code/scip)（可选精确导航） |
| 数据库提取 | 提取表列、模型映射、复合键、关系基数、读写及 join，区分声明与推断。 | 项目数据库提取脚本、ORM/DDL 分析、已有 schema 导出文件 |
| 提取器运行 | 通过 JSON 输入输出运行脚本，记录依赖、版本、超时、失败和覆盖缺口。 | Rust 异步子进程、取消控制、[jsonschema](https://docs.rs/jsonschema/latest/jsonschema/) |
| 统一事实输出 | 各类提取器输出统一实体、关系、证据和依赖，允许项目扩展类型。 | Rust 契约、serde、schemars、JSON Schema |
| 代码地图 | 输出相关文件、符号和方法轮廓，按当前问题排序并限制上下文体积。 | Rust、Tree-sitter、图排名；参考 [Aider](https://aider.chat/docs/repomap.html) |
| 事实存储与搜索 | 保存来源、实体、关系及采集记录，检索路由、标题、业务别名和源码。 | Rust、rusqlite、SQLite [FTS5](https://www.sqlite.org/fts5.html)、中文 n-gram |
| 链路关联与查询 | 按服务、方法、路径和数据源连接各类事实，支持正向、反向和有界路径查询。 | 项目关联脚本、Rust 图遍历、SQLite 索引 |
| 业务知识生成 | 围绕采购台账、审批、入库等主题生成跨层说明、图和源码引用。 | 内置 Rig Agent／外部 LLM、Markdown、Mermaid、Core 事实查询 |
| 项目 Skill 生成 | 生成简短 SKILL.md、采集规则说明、业务参考文档和可执行脚本入口。 | Rust、Handlebars、Markdown；参考 [Repomix Skill 输出](https://repomix.com/guide/agent-skills-generation) |
| 页面定位 | 输入页面路由，返回标题、组件、操作、请求、后端方法及相关表字段。 | 路由精确匹配、链路查询、LLM 解释 |
| 操作流程解释 | 结合状态变更与流程资料，说明前后流程、页面操作和执行条件，标记未知项。 | 工作流提取脚本、业务文档、LLM、Mermaid |
| 台账取数辅助 | 优先定位已有 SQL/ORM 查询，解释字段来源、粒度、聚合和关联条件，输出待验证方案。 | 事实查询、已有 SQL/ORM 分析、LLM、数值验收样例 |
| 证据追溯 | 为结论保存文件路径、行范围、内容哈希和推断标记，支持按需读取源码。 | Rust 文件接口、SHA-256、SQLite |
| 增量更新 | 检测文件、匹配集合、规则与脚本变化，重跑受影响提取器；需 LLM 重写的主题标记过期。 | Rust 文件扫描、内容哈希、依赖记录、插件运行记录 |
| SDK | 提供路由定位、事实搜索、链路追踪、上下游查询和证据读取。 | TypeScript 库、napi-rs、Rust Core、同源 Schema 类型 |
| CLI 与 MCP | 为终端和 LLM 提供采集、查询、生成和状态入口，查询与维护能力分别登记。 | Rust clap CLI、[MCP TypeScript SDK](https://modelcontextprotocol.io/docs/sdk)、统一 SDK |
| 场景验收 | 验证页面定位、操作流程、台账数值、误关联、源码变化和知识过期。 | Rust 综合用例、SDK/MCP 集成用例、SQLite 数据、人工真值 |
