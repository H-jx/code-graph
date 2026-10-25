# 新项目目录与使用流程设计：Rust 核心

功能沿用 [功能设计](./NEW-PROJECT-DESIGN.md)，实现语言与目录归属以本文为准。本文中的目录、命令、配置和接口均为新项目的目标设计，尚未实现或发布。Rust Core 负责索引和查询，基于 Rig 的简单 Agent 负责按 Prompt 调查项目、生成采集插件并推动验证和激活。

## 1. 源码目录

```text
new-project/
├── Cargo.toml                         # Rust workspace
├── Cargo.lock
├── package.json                       # SDK、MCP 与 native binding 的构建入口
├── pnpm-workspace.yaml
│
├── crates/
│   ├── core/                          # Rust 核心库，内部按功能拆模块
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs                 # 对外 facade，组合采集和查询用例
│   │   │   │
│   │   │   ├── contract/              # 共用数据与接口契约
│   │   │   │   ├── mod.rs
│   │   │   │   ├── entity.rs          # 实体、作用域、稳定身份
│   │   │   │   ├── relation.rs        # 关系、候选、依据与解析状态
│   │   │   │   ├── evidence.rs        # 来源、位置、内容版本与引用
│   │   │   │   ├── collector.rs       # 提取器 manifest、输入输出与运行记录
│   │   │   │   ├── query.rs           # SDK/CLI/MCP 共用查询请求与响应
│   │   │   │   └── error.rs           # 统一错误码
│   │   │   │
│   │   │   ├── project/               # 项目接入与来源管理
│   │   │   │   ├── mod.rs
│   │   │   │   ├── config.rs          # 仓库、应用、数据源、文件范围配置
│   │   │   │   ├── scan.rs            # 文件发现、忽略规则与集合变化
│   │   │   │   └── snapshot.rs        # 内容哈希、来源快照与新鲜度
│   │   │   │
│   │   │   ├── syntax/                # 可选通用语法能力
│   │   │   │   ├── mod.rs
│   │   │   │   ├── parser.rs          # Tree-sitter 调度
│   │   │   │   ├── symbols.rs         # 声明、导入、引用与保守调用事实
│   │   │   │   └── scip.rs            # 导入外部 SCIP 精确索引
│   │   │   │
│   │   │   ├── collect/               # 外部提取器宿主
│   │   │   │   ├── mod.rs
│   │   │   │   ├── manifest.rs        # 命令、运行时、输入及 artifact 配置
│   │   │   │   ├── registry.rs        # 发现插件、校验候选、组织激活与版本切换
│   │   │   │   ├── runner.rs          # JSON stdin/stdout、超时、取消与限额
│   │   │   │   ├── validate.rs        # Schema、证据位置、引用与依赖校验
│   │   │   │   └── import.rs          # 组织事务导入、替换与运行结果
│   │   │   │
│   │   │   ├── index/                 # 索引构建与增量更新
│   │   │   │   ├── mod.rs
│   │   │   │   ├── build.rs           # 来源→语法→提取→关联→索引的执行顺序
│   │   │   │   ├── dependencies.rs    # 文件、配置、脚本与候选集合依赖
│   │   │   │   └── incremental.rs     # 变化检测、失效、受影响任务重跑
│   │   │   │
│   │   │   ├── storage/               # SQLite 持久化与事务
│   │   │   │   ├── mod.rs
│   │   │   │   ├── database.rs        # 连接、版本、事务与一致读快照
│   │   │   │   ├── sources.rs         # 来源内容、哈希、位置和依赖
│   │   │   │   ├── entities.rs        # 实体、业务属性与作用域
│   │   │   │   ├── relations.rs       # 正反向关系、候选和证据
│   │   │   │   ├── plugins.rs         # 插件版本、活动版本及原子激活事务
│   │   │   │   └── runs.rs            # 提取记录、替换范围、状态与覆盖缺口
│   │   │   │
│   │   │   ├── search/                # 统一文本与事实搜索
│   │   │   │   ├── mod.rs
│   │   │   │   ├── documents.rs       # 标题、路由、别名、源码的搜索投影
│   │   │   │   ├── tokenize.rs        # 中文 n-gram 与标识符拆词
│   │   │   │   └── rank.rs            # 精确匹配、全文结果与入口排序
│   │   │   │
│   │   │   ├── graph/                 # 通用关系查询
│   │   │   │   ├── mod.rs
│   │   │   │   ├── neighbors.rs       # 上下游、方向、关系类型与范围过滤
│   │   │   │   └── paths.rs           # 有界路径、条件、候选与断点
│   │   │   │
│   │   │   ├── codemap/               # 面向当前问题的代码地图
│   │   │   │   ├── mod.rs
│   │   │   │   ├── rank.rs            # 根据入口、符号和依赖选择相关代码
│   │   │   │   └── render.rs          # 方法轮廓、文件地图与内容预算
│   │   │   │
│   │   │   ├── query/                 # 需求定位与三个业务场景的事实组合
│   │   │   │   ├── mod.rs
│   │   │   │   ├── feature.rs         # 从需求定位入口、业务方法和关联代码
│   │   │   │   ├── page.rs            # 路由→页面→操作→请求→后端→表字段
│   │   │   │   ├── workflow.rs        # 查询已有流程、状态、操作及上下游
│   │   │   │   └── report.rs          # 查询已有取数实现、字段血缘和关联口径
│   │   │   │
│   │   │   ├── evidence/              # 证据读取与核验
│   │   │   │   ├── mod.rs
│   │   │   │   ├── position.rs        # 原文件 UTF-8 字节与行范围校验
│   │   │   │   └── read.rs            # 按需读片段、版本核对与过期反馈
│   │   │   │
│   │   │   └── knowledge/             # 知识材料组织与输出
│   │   │       ├── mod.rs
│   │   │       ├── context.rs         # 按主题收集事实、源码和引用
│   │   │       ├── documents.rs       # 接收、校验并保存 LLM 生成的业务说明
│   │   │       └── skill.rs           # 渲染 SKILL.md、参考资料与脚本导航
│   │   ├── migrations/
│   │   │   └── 0001_initial.sql       # 来源、实体、关系、运行记录与 FTS
│   │   ├── templates/
│   │   │   ├── skill.md.hbs           # Skill 入口模板
│   │   │   ├── references.md.hbs      # 四类事实与代码地图模板
│   │   │   └── topic.md.hbs           # 业务主题、引用与缺口模板
│   │   └── tests/
│   │       ├── indexing.rs            # 导入、搜索、事务与增量综合用例
│   │       ├── collectors.rs          # 外部脚本运行与异常综合用例
│   │       └── business_scenarios.rs  # 页面、流程、台账三类场景
│   │
│   ├── agent/                         # 简单 Rust Agent；依赖 Core，使用 Rig
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                 # 项目增强索引与知识生成任务入口
│   │       ├── config.rs              # 模型、凭据环境变量与运行预算
│   │       ├── prompts.rs             # 加载通用模板、任务目标和当前契约
│   │       ├── tools.rs               # 将 Core 能力注册为受控 Rig 工具
│   │       └── runner.rs              # 调查→生成→验证→修复→激活的任务循环
│   │
│   └── cli/                           # Rust 命令行入口
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs
│           └── commands/
│               ├── init.rs            # 初始化目标项目的配置与工作目录
│               ├── index.rs           # 全量/增量索引
│               ├── collect.rs         # 运行指定提取器或采集集合
│               ├── agent.rs           # 配置模型、生成和修复项目插件
│               ├── query.rs           # 搜索、页面、流程、台账和证据查询
│               ├── generate.rs        # 导出 LLM 上下文、文档与 Skill
│               └── status.rs          # 新鲜度、覆盖、错误与缺口
│
├── bindings/
│   └── node/                          # Rust → Node.js 原生绑定
│       ├── Cargo.toml
│       ├── package.json
│       ├── src/lib.rs                 # napi-rs；转换参数、异步任务和取消信号
│       └── npm/                       # 各平台预编译 native 包
│
├── packages/
│   ├── sdk/                           # TypeScript SDK
│   │   ├── package.json
│   │   └── src/
│   │       ├── index.ts
│   │       ├── client.ts              # 打开索引、调用 native、关闭资源
│   │       └── generated/             # 从 Rust 契约生成的类型与 Schema
│   └── mcp/                           # 本地 stdio MCP 接入
│       ├── package.json
│       └── src/
│           ├── index.ts
│           ├── tools.ts               # 将 SDK 查询映射为 MCP 工具
│           └── resources.ts           # 事实、证据、项目说明与 Skill 资源
│
├── prompts/                           # 产品提供的 Markdown Prompt 模板
│   ├── shared.md                      # 统一协议、证据、未知项和完成条件
│   ├── investigate.md                 # 调查项目结构、封装和业务约定
│   ├── collector.md                   # 按指定能力生成采集插件和验收样例
│   ├── link.md                        # 根据真实绑定连接跨层事实
│   ├── repair.md                      # 按结构化验证错误修复候选插件
│   └── knowledge.md                   # 按主题生成业务说明与 Skill 材料
│
├── skills/                            # 外部 LLM 可复用的工作指南，引用同一协议和模板
│   ├── project-onboarding/
│   │   ├── SKILL.md                   # 调查项目、生成提取器与验收样例
│   │   └── references/
│   │       ├── extractors.md          # 路由、前端、API、数据库的调查规范
│   │       └── business-docs.md       # 流程、取数口径、证据与未知项的整理规范
│   └── project-query/
│       └── SKILL.md                   # 如何查事实、读源码、解释链路和核对新鲜度
│
├── collectors/                        # 可选脚本模板与示例
│   ├── templates/
│   │   ├── node/                      # Node.js JSON 输入输出模板
│   │   └── python/                    # Python JSON 输入输出模板
│   └── examples/
│       ├── frontend-backend-a/        # 第一种项目布局的四类提取器与关联器
│       └── frontend-backend-b/        # 不同语言/布局的独立接入样例
│
├── fixtures/                          # 源码、流程文档、SQL 与人工真值
│   ├── page-location/
│   ├── purchase-workflow/
│   └── purchase-ledger/
│
├── xtask/                             # Rust 开发辅助命令
│   └── src/main.rs                    # 导出契约、组织原生包与 CLI 制品
├── scripts/                           # Node 侧契约类型生成与打包辅助
├── docs/                              # 产品文档与采集协议
└── .github/workflows/                  # Rust/Node 检查、多平台构建与发布
```

## 2. 功能归属与实现工具

| 功能 | 实现目录 | 语言与工具 |
| --- | --- | --- |
| 文件发现、忽略规则、多仓库接入 | `core/project` | Rust、ignore、globset |
| 通用语法与精确索引接入 | `core/syntax` | Rust、Tree-sitter、外部 SCIP indexer |
| 脚本运行、超时、取消、输出限制 | `core/collect` | Rust、异步进程运行与取消控制 |
| 插件发现、验证、自动加载与版本激活 | `core/collect`、`core/storage` | Rust、manifest、制品哈希、SQLite 事务 |
| manifest、JSON 与证据校验 | `core/contract`、`core/collect` | Rust、serde、schemars、[jsonschema](https://docs.rs/jsonschema/latest/jsonschema/) |
| 索引构建、依赖失效与增量重跑 | `core/index` | Rust、内容哈希、依赖记录 |
| 事实、关系、来源与运行记录 | `core/storage` | Rust、[rusqlite](https://docs.rs/rusqlite/latest/rusqlite/)、SQLite |
| 标题、路由、别名和源码搜索 | `core/search` | Rust、SQLite FTS5、中文 n-gram |
| 正反向关系与路径查询 | `core/graph` | Rust、BFS/有界路径遍历、SQLite 邻接索引 |
| 代码地图与上下文选择 | `core/codemap` | Rust、符号提取、图排名、内容预算 |
| 页面、流程、台账事实查询 | `core/query` | Rust、搜索与图查询组合 |
| 原文件定位与版本核验 | `core/evidence` | Rust、SHA-256、UTF-8 位置映射 |
| 上下文导出、Markdown 与 Skill 输出 | `core/knowledge` | Rust、[Handlebars](https://docs.rs/handlebars/latest/handlebars/)、Markdown |
| Prompt 模板与任务上下文 | `prompts`、`crates/agent`、`skills` | Markdown、Rust 模板加载、当前契约与有界源码材料 |
| 项目调查、插件生成与失败修复 | `crates/agent` | Rust、[Rig](https://rig.rs/docs/concepts/agent)、Core 工具；也支持外部 LLM |
| 项目提取脚本 | 目标项目 `collectors` | Node.js／Python 等项目适合的运行时、统一 JSON 协议 |
| CLI | `crates/cli` | Rust、[clap](https://docs.rs/clap/latest/clap/)、Core |
| Node 原生绑定 | `bindings/node` | Rust、[napi-rs](https://napi.rs/docs/introduction/getting-started) |
| SDK | `packages/sdk` | TypeScript、生成类型、Node 原生绑定 |
| MCP | `packages/mcp` | TypeScript、MCP SDK、统一 SDK |
| 安装与制品发布 | `xtask`、`bindings/node/npm`、`.github/workflows` | Rust CLI 平台二进制、npm native 预编译包、GitHub Actions |

## 3. 模块边界

| 模块 | 职责边界 |
| --- | --- |
| Rust Core | 统一拥有扫描、运行、校验、事务、索引、检索、追踪、增量和材料输出。 |
| Rust Agent | 组合 Prompt、模型与 Core 工具，调查项目、生成和修复插件、整理业务说明；不直接写索引表。 |
| Rust CLI | 解析命令，调用 Core 或 Agent，提供独立终端入口。 |
| Node binding | 转换调用与错误，传递进度和取消，管理 native 生命周期。 |
| TypeScript SDK | 提供易用的类型化接口；算法和持久化调用 Rust 实现。 |
| MCP | 将 SDK 操作映射成工具和资源。 |
| 项目提取器 | 分析实际框架、路由、客户端和 ORM，输出统一事实、关系及依赖。 |
| 外部 LLM | 调查项目约定、生成提取器、解释业务和整理主题文档。 |
| `query/workflow`、`query/report` | 查询、组织已采集的流程与取数依据；业务解释交给 LLM，缺失信息明确返回。 |
| `contract` | 共用类型的唯一来源；SDK 类型与 JSON Schema 从这里生成。 |
| `storage` | 数据库写入的唯一入口；其他模块通过其接口使用事务和查询。 |

索引和查询功能先保留在一个 Core crate 内，通过功能模块划分。Agent 单独作为依赖 Core 的 crate，使 Core、Node binding 和普通查询无需依赖模型框架。CLI 的 Agent 命令调用该 crate；初期只实现一个 Rig Agent，不建立多 Agent 调度或通用引擎适配层。

## 4. 调用关系

```text
Rust CLI ───────────────────────→ Rust Core
Rust CLI → Rust Agent（Rig）─────→ Rust Core 工具
TypeScript SDK → napi-rs binding → Rust Core
MCP → TypeScript SDK ───────────→ Rust Core

Prompt + 项目目标 → Agent → 候选插件 → Core 验证/激活 → 索引
Rust Core → 项目提取脚本 → JSON 事实 → Rust 校验与索引
Rust Core → 主题上下文 → Rust Agent / 外部 LLM → 主题产物 → Rust 组织与输出
```

## 5. 目标业务项目中的目录

```text
target-project/
├── .project-knowledge/
│   ├── project.json                   # 仓库、范围、插件发现和可选 Agent 配置
│   ├── conventions.md                 # 已调查/确认的项目约定，作为生成输入
│   ├── prompts/                       # 可选的项目 Prompt 补充，不覆盖统一协议
│   ├── work/
│   │   ├── candidates/<run-id>/       # 待验证的 manifest、脚本和验收样例
│   │   ├── contexts/                  # 生成任务按需使用的源码与事实材料
│   │   └── agent-runs/                # 任务进度、诊断和预算使用记录
│   ├── collectors/<plugin-id>/<version>/
│   │   ├── manifest.json              # 能力、阶段、协议版本、入口和依赖
│   │   ├── src/                       # routes/frontend/api/database/link 等实现
│   │   ├── cases/                     # 输入、预期结果和来源引用
│   │   └── deps/                      # 需要额外依赖时保存依赖声明与锁文件
│   └── index.sqlite                   # 事实、依赖、运行记录及插件活动版本
│
└── .agents/skills/<project-name>/
    ├── SKILL.md                       # 项目入口、使用场景和查询方式
    └── references/
        ├── conventions.md             # 从调查输入生成的约定参考
        ├── routes.md
        ├── frontend.md
        ├── api.md
        ├── database.md
        ├── code-map.md
        ├── index-manifest.json        # 文档对应的索引快照、来源版本与覆盖范围
        └── topics/
            ├── purchase-ledger.md     # 跨层页面与取数说明
            └── purchase-workflow.md   # 状态、操作和上下游流程说明
```

插件 ID 与目录名不绑定框架，一个插件可以声明路由、注释、模块等多类能力。已发布版本按制品哈希保持不可变，活动版本由 SQLite 注册记录决定；候选文件不参与日常采集。项目配置和确认过的约定作为输入保存；索引及 Skill 参考资料可重新生成。work 内的临时材料和运行记录排除在采集范围之外。

## 6. 开发者：建立基础索引

以下命令、包名和接口是新项目的目标设计，尚未实现或发布。暂用 `project-knowledge` 作为 CLI 名，示例在目标业务仓库根目录执行。

### 6.1 初始化项目

开发者安装对应平台的 Rust CLI，执行以下命令。基础索引只需要 CLI；内置 Rig Agent 使用 Rust，无需额外 Node.js Agent 进程，但需要配置模型；SDK/MCP 使用 Node.js 及匹配平台的 native 包；外部提取脚本使用 manifest 声明的运行时。

```sh
project-knowledge init --root .
```

生成 `.project-knowledge/project.json`，登记仓库及文件范围。配置示例：

```json
{
  "schemaVersion": 1,
  "projectId": "erp",
  "repositories": [{ "id": "repo.main", "root": ".." }],
  "selection": {
    "include": ["**/*"],
    "exclude": [".git/**", "node_modules/**", "target/**", "dist/**", ".project-knowledge/**", ".agents/skills/**"],
    "maxFileBytes": 2097152
  },
  "syntax": { "enabled": true },
  "knowledgeInputs": [],
  "collectors": [],
  "plugins": {
    "directories": ["collectors"],
    "autoLoad": true,
    "autoActivate": true
  },
  "agent": { "enabled": false }
}
```

`root` 相对于配置文件所在目录解析，`..` 指向目标仓库根目录；selection 相对于各仓库根目录解析。collectors 的显式 manifest 路径与 plugins.directories 均相对于 project.json，按插件 ID 去重；重复 ID、版本冲突或不同制品使用同一版本时报错。knowledgeInputs 通过 repositoryId 和仓库相对路径绑定来源。前后端分仓时，在 repositories 中分别登记路径。应用、服务和数据源在调查后补充，不要求固定目录。

### 6.2 建立与检查索引

```sh
project-knowledge index --config .project-knowledge/project.json --base
project-knowledge status --config .project-knowledge/project.json --check-freshness
project-knowledge query --config .project-knowledge/project.json --operation search --text "purchaseLedger"
```

| Rust 执行内容 | 得到的能力 |
| --- | --- |
| 扫描选中文本、计算哈希并建立源码 FTS | 按路径、关键词和源码定位文件 |
| 为支持的语言提取符号、导入、引用和保守调用 | 查定义、关联符号和代码地图 |
| 按配置导入可选 SCIP 产物 | 增强可用语言的精确导航 |
| 记录未知语法、跳过文件和未配置能力 | 明确基础索引的范围与缺口 |

基础索引不需要 LLM、框架插件或业务数据库连接。没有语法解析器的选中文本仍可被搜索；动态路由、请求代理、ORM 绑定和业务流程在未采集时保持未知。

status 分别返回 `base`、`collectors`、`knowledge` 状态，以及 `freshness=fresh/stale/unknown`。基础扫描成功时，业务层可以是 `not_configured` 或 `partial`，不能统一显示成“完整索引完成”。CLI、SDK 和 MCP 使用同一配置及其 index.sqlite，不分别建立独立索引。

## 7. 开发者与 LLM：建立项目专属索引和 Skill

### 7.1 让 LLM 调查真实约定

开发者可以使用第 11 节的内置 Rig Agent，也可以向外部编码 LLM 提供目标项目及产品的 `skills/project-onboarding/SKILL.md`。两种方式使用相同的 Prompt 模板和采集协议。LLM 先读取当前契约、索引状态和基础代码地图，再有针对性地读源码。

可以提供这样的任务：

```text
为当前项目建立业务链路索引。先读取工具能力和采集协议，再调查路由、
前端请求封装、后端注册和数据库映射。从采购台账等代表性入口逐跳核对。
生成四类提取脚本、必要的关联脚本、manifest、项目约定和验收样例。
保留动态值、重名候选和未覆盖范围；每个判断附实际源码依据。
```

| 调查对象 | LLM 需要确认的项目规则 |
| --- | --- |
| 路由 | hash/history、注册表/文件路由、菜单、别名、页面映射、title/i18n |
| 前端 | 组件事件和方法、Hook、请求 wrapper、baseURL、代理、生成客户端 |
| API | 服务边界、HTTP/GraphQL 注册、Handler、实例绑定、业务方法调用 |
| 数据库 | 数据源、ORM 模型与表列映射、复合键、查询与关联条件 |
| 注释、模块和特定入口 | 注释与符号的绑定、业务别名、模块边界、定时任务/消息消费/命令等真实入口 |
| 业务 | 状态与操作、前后单据键、取数粒度、有效状态和已有流程资料 |

LLM 整理已调查的约定，保留依据和推断标记。内置 Agent 先写入候选材料，由 Core 的 import-conventions 发布到 `.project-knowledge/conventions.md` 并作为显式 documentation 输入登记到 knowledgeInputs；外部 LLM 也可使用同一维护接口。插件生成和验证使用登记后的来源快照。采集器、配置是控制依赖；自动生成的 Skill 文档不重新冒充源码事实参与采集。

### 7.2 生成与登记提取器

LLM 在目标项目的 work/candidates 目录生成插件候选，包含脚本和 manifest：

| 产物 | 内容 |
| --- | --- |
| routes/frontend/api/database 脚本 | 按本项目语法提取实体、属性、关系和证据 |
| 按需增加的采集能力 | 注释、业务模块、自定义入口、状态流转等，仍输出相同协议 |
| link 脚本 | 读取已有实体，依据明确绑定建立跨层关系 |
| manifest | ID、版本、协议版本、能力、extract/link 阶段、运行命令、入口、artifact 哈希、输入选择与依赖 |
| 验收样例 | 正常链路、同名误关联、动态值、遗漏目标、源码变化后的预期结果 |

内置 Agent 调用 Core 验证候选；默认通过后自动发布到 collectors、登记并激活，无需手工编辑 manifest 列表。验证失败时返回结构化错误，Agent 在预算内修复后重试。外部 LLM 通过维护接口提交同样的候选。需要人工管理时可关闭 autoActivate，再显式激活验证通过的候选。脚本可以使用项目适合的 Node.js/Python 等运行时，统一通过 JSON stdin/stdout 接入 Rust；插件无需依赖 Rig。

Rust 负责导出来源快照、提供有界的已有实体与引用、执行脚本、校验输出和写库。实体及关系均携带来源路径、哈希、准确位置和 basis；关系可引用本批或已有实体的两端，并有独立证据。link 的输入在 extract 提交后重新导出。

### 7.3 验证、采集与复核

```sh
project-knowledge collect --config .project-knowledge/project.json --phase extract --validate-only
project-knowledge collect --config .project-knowledge/project.json --phase extract
project-knowledge collect --config .project-knowledge/project.json --phase link --validate-only
project-knowledge collect --config .project-knowledge/project.json --phase link
project-knowledge status --config .project-knowledge/project.json --check-freshness
```

以上为已激活或显式登记提取器的维护命令；候选的验证与激活由第 11 节流程完成。validate-only 运行脚本，并检查结构、位置、快照、依赖和引用；不提交事实，不增加索引 generation。正式采集再次校验当前输入，失败整批回滚。通过位置校验后，仍需用源码真值核对关系语义。

复核至少确认：页面能连到正确服务；不同数据源同名表不串联；动态目标保留缺口；新增候选或输入文件会触发重新关联。直接由 AI 判断的关系保持 inferred；AI 生成静态脚本不等于脚本的业务判断已经通过验收。

### 7.4 生成业务知识和项目 Skill

```sh
project-knowledge generate context --config .project-knowledge/project.json --topic "采购台账的页面操作与取数链路" --output .project-knowledge/work/purchase-ledger.context.json
```

Rust 导出当前主题的事实、代码地图、必要源码片段、证据引用、覆盖缺口和预算信息。内置 Agent 或外部 LLM 读取这些材料，生成主题产物，包含 Markdown、使用的来源引用、推断和未知项；需要补充代码时继续通过查询接口读取。

```sh
project-knowledge generate docs --config .project-knowledge/project.json --input .project-knowledge/work/purchase-ledger.topic.json
project-knowledge generate skill --config .project-knowledge/project.json --name erp --output .agents/skills/erp
```

Rust 检查主题产物的格式、引用和版本，将内容保存为项目知识，再用模板输出 Skill：

- `SKILL.md`：项目适用范围、常见业务入口、查询工具和阅读顺序；先检查新鲜度，过期说明仅作为调查线索。
- 四类参考文件：已采集路由、组件/请求、接口和数据库映射。
- `topics`：跨层业务解释、流程、统计口径及其证据。
- `index-manifest.json`：绑定索引快照、来源版本、覆盖与生成记录。

文档格式和引用有效不代表业务解释自动正确。开发者核对代表性页面、流程与数值样例；无法证明的内容保留推断或缺口。Skill 提供导航与已整理知识，当前事实和原文仍通过 SDK/MCP 按需获取。

## 8. LLM 如何通过 SDK/MCP 查询

### 8.1 查询接口

| SDK 方法 | MCP 工具/资源 | 用途 |
| --- | --- | --- |
| `getCapabilities()` | `project-knowledge://schema` 资源 | 获取实际接口、类型、关系谓词和资源上限 |
| `status()` | `index_status` | 查看源码新鲜度、采集覆盖和知识版本 |
| `search()` | `search` | 按业务词、路由、符号、路径和范围检索候选 |
| `locateFeature()` | `locate_feature` | 从需求组合搜索、入口定位、链路与关联代码 |
| `trace()` | `trace_links` | 从已返回实体沿入边/出边追踪，可筛选关系类型 |
| `getCodeMap()` | `get_code_map` | 返回相关符号和文件轮廓，帮助选择精读范围 |
| `readSource()` | `read_source` | 原样使用返回的来源引用读取片段并核对版本 |

MCP 查询路径为 `LLM → MCP → SDK → napi-rs → Rust Core`。LLM 直接调用工具，无需每次编写 TypeScript。SDK 也可由开发者封装成其他 Agent 的工具，或在具备终端执行能力的环境中调用。

导出快照、写入候选、验证、激活、运行采集器和生成文档属于维护操作，和上述只读查询分开登记。内置 Rig Agent 直接使用 Core 工具；外部 Agent 可连接维护模式的 MCP。普通查询 MCP 默认只暴露本节工具，一次需求定位不会隐式生成或执行新脚本。

### 8.2 SDK 使用示例

以下包名为设计占位，正式名称由新项目确定：

```ts
import { ProjectKnowledge } from '@project-knowledge/sdk';

const index = await ProjectKnowledge.open({
  configPath: '.project-knowledge/project.json',
});

try {
  const status = await index.status({ checkFreshness: true });
  if (status.freshness !== 'fresh') {
    throw new Error('请先刷新索引，或明确选择按旧版本调查');
  }

  const result = await index.locateFeature({
    query: '采购台账增加供应商筛选，相关业务逻辑代码在哪里？',
    routeHint: '/#/Other/purchaseLedger',
    view: 'compact',
    traversal: { direction: 'both', maxDepth: 8 },
    budget: {
      maxEntities: 60,
      maxRelations: 120,
      maxResponseChars: 24000,
    },
  });

  const sources = await index.readSource({
    references: result.sourceReferences.slice(0, 5),
    maxChars: 12000,
  });

  console.log({ status, result, sources });
} finally {
  await index.close();
}
```

调用方先判断 status/completeness；索引过期时刷新索引，或明确按旧版本解释。示例中的来源引用由查询结果提供，不手工拼 ID。每次组合查询使用一致读快照；后续读取若发现版本过期，返回 expired，调用方重新查询。

### 8.3 MCP 使用流程

开发者启动本地 stdio MCP，并在客户端登记可执行程序及参数：

```sh
project-knowledge-mcp --config /absolute/path/to/target-project/.project-knowledge/project.json
```

这是 `packages/mcp` 的目标 bin；Rust CLI 仍负责独立命令行功能。MCP 提供的 schema 和生成的项目 Skill 共同指导 LLM 使用工具。

LLM 收到需求后，按以下顺序调用：

| 步骤 | 调用 | 如何使用结果 |
| --- | --- | --- |
| 1 | schema 资源、`index_status`，开启 checkFreshness | 了解实际支持的能力、索引版本与缺失采集范围 |
| 2 | `locate_feature`，传需求和可选 routeHint | 找候选入口、核心业务方法、链路和关联代码 |
| 3 | 有歧义时 `search` / `get_code_map` | 根据标题、模块、作用域与方法轮廓缩小范围 |
| 4 | `trace_links`，原样传入选中实体引用 | 查看谁调用它、它继续调用谁、数据读写及共享依赖 |
| 5 | `read_source` | 精读关键方法、条件、SQL/ORM 和调用点 |
| 6 | LLM 整理答案 | 按代码职责解释位置、关联原因、证据和未确认项 |

locate_feature 输入示例：

```json
{
  "query": "采购台账增加供应商筛选，业务逻辑在哪里，关联哪些代码？",
  "routeHint": "/#/Other/purchaseLedger",
  "view": "compact",
  "traversal": { "direction": "both", "maxDepth": 8 },
  "budget": { "maxEntities": 60, "maxRelations": 120, "maxResponseChars": 24000 }
}
```

`locateFeature` 的事实组合由 Rust `query/feature.rs` 实现；最终自然语言解释由调用方 LLM 完成。SDK 与 MCP 共用请求和响应，不分别实现定位算法。

## 9. 一个需求如何找到业务逻辑与关联代码

以“采购台账增加供应商筛选”为例，以下文件名和方法名只是结果形态示意，实际路径来自目标项目。

### 9.1 Rust 的定位过程

1. 用给定 URL 匹配真实应用的 hash/history 路由、别名和模板；没有 URL 时按业务标题、别名及已有主题文档召回候选。
2. 找到页面、筛选操作和事件处理方法；继续追踪请求封装、目标服务、接口 Handler、业务查询和表字段。
3. 从核心方法查入边和出边，收集上游调用方、下游数据访问、共享组件、请求封装及已有测试。
4. 将根选择条件与遍历范围分开：选根为页面时，允许后续经过请求、后端方法和数据实体。
5. 按“核心实现、调用上下游、共享依赖、数据来源、验证用例”组织结果，并给出每项关联理由。

### 9.2 应返回什么

| 类别 | 示例内容 | 为什么相关 |
| --- | --- | --- |
| 页面入口 | `web/src/pages/PurchaseLedger.vue` | 路由映射到该页面，包含筛选区域 |
| 组件/事件 | `SupplierFilter`、`onSearch` | 产生供应商筛选条件并触发查询 |
| 前端请求 | `web/src/api/purchase-ledger.ts` 的 `listLedger` | 把页面条件转换为接口参数 |
| 后端入口 | `api/src/purchase/ledger-handler.ts` | 接收并校验查询参数 |
| 核心业务逻辑 | `api/src/purchase/ledger-query.ts` 的 `queryLedger` | 组合供应商、状态等业务过滤和聚合 |
| 数据关系 | 采购明细、供应商、入库明细的表列与关联键 | 解释筛选字段、统计来源和一对多聚合位置 |
| 共享依赖 | HTTP wrapper、权限守卫、复用查询方法 | 说明参数传递、访问条件及可能受影响调用 |
| 已有验证 | 索引中实际存在且可关联的测试/SQL 样例 | 帮助核对参数、过滤和统计结果 |

每项附实体/来源引用、位置、basis 和关联证据。仅有同名或关键词重合时标为候选；未索引到测试时不生成假路径。

### 9.3 给 LLM 的结果契约

| 字段 | 内容 |
| --- | --- |
| `snapshot` | 本次结果的索引身份、generation 与来源版本信息 |
| `roots` | 命中的入口、名称、路径和匹配理由 |
| `paths` | 逐跳实体/关系、方向、条件与来源引用 |
| `relatedCode` | 相关符号、文件、职责、关联原因和依据 |
| `data` | 表列、字段映射、读写、join 与已知统计口径 |
| `sourceReferences` | 供 readSource 原样传回的有界源码引用 |
| `gaps` | 缺少采集能力、动态值、歧义、范围排除等原因 |
| `completeness` | 新鲜度、相关采集覆盖、候选集完整性、预算截断 |

LLM 最终回答应先指出核心业务实现，再解释前后端链路与关联文件，最后列出需核对的条件和断点。需求涉及变更时，可基于这些结果提出修改位置；查询本身不修改业务代码。

## 10. 覆盖不足与后续更新

| 情况 | 开发者/LLM 的处理 |
| --- | --- |
| 只有基础索引 | 用 search、代码地图和原文调查候选；业务链路显示未采集，不声称不存在 |
| 业务名称搜不到 | 补查标题、菜单、i18n、注释和真实标识符；有依据后加入业务别名 |
| 请求目标或实例绑定不确定 | 返回候选和缺失依据，补充项目提取规则后重新采集 |
| 缺少业务流程资料 | 读取已有操作说明或补充开发者确认；代码调用顺序不自动成为流程顺序 |
| 台账口径不明确 | 返回需要确认的粒度、有效状态、时间字段和关联条件；优先查看已有查询 |
| 源码或脚本变化 | Rust 根据文件、配置、artifact 与候选集合依赖撤销过期事实，重跑受影响采集 |
| 项目约定改变导致采集失败 | 保留诊断与 stale 状态，通过 agent repair 生成新候选并重新验证激活 |
| Skill 来源已过期 | 标记过期主题；依据新索引重新收集上下文、核对说明并生成 Skill |
| 查询达到预算 | 返回 partial、停止位置和继续查询所需引用，LLM 缩小范围或分段查询 |

开发者日常更新入口：

```sh
project-knowledge index --config .project-knowledge/project.json --incremental
project-knowledge status --config .project-knowledge/project.json --check-freshness
project-knowledge generate skill --config .project-knowledge/project.json --name erp --output .agents/skills/erp
```

增量索引默认维护基础事实并按依赖运行已登记及自动加载的活动提取器和关联器；`--base` 只更新基础层，并将受影响业务层标为过期。generate skill 可以刷新结构化参考资料；需要 LLM 重写的过期主题继续标为 stale，直到重新导入与当前来源相符的说明。日常增量采集不调用模型；插件修复或新能力生成通过 Agent 命令启动。

## 11. Prompt、Rig Agent 与插件自动加载

### 11.1 Agent 选型

采用 Rig 作为内置 Agent 库，复用模型接入、多轮工具调用和 Hooks，在 `crates/agent` 中补充项目工具及任务完成条件。依据 2026-10-25 的官方文档调研，以下为集成工作量判断，尚未做运行效果对比：

| 方案 | 已有能力与接入方式 | 本项目选择 |
| --- | --- | --- |
| [Rig](https://rig.rs/docs/concepts/agent) | Rust 库，组合模型、Prompt 和工具，执行多轮循环；[工具接口](https://rig.rs/docs/concepts/tools/)可直接调用 Rust 函数。 | 内置首选，复用 Core 能力；补齐受控源码读取、候选写入、验证和激活工具。 |
| [Goose](https://goose-docs.ai/docs/guides/recipes/) | 完整 Rust Agent，Recipes 封装 Prompt、参数和扩展；可无交互执行并接入 MCP。 | 可作为外部 Agent；原生 [GDK SDK](https://goose-docs.ai/docs/gdk/sdk/)仍标注 Alpha，嵌入需锁定版本。 |
| [Pi](https://pi.dev/docs/latest/sdk) | TypeScript SDK 提供编码 Agent、会话和工具；支持 [Prompt 模板](https://pi.dev/docs/latest/prompt-templates)。 | 可快速作为外部 Agent，但需要 Node.js Agent 运行时。 |
| [AutoAgents](https://liquidos-ai.github.io/AutoAgents/) | Rust，支持 ReAct、CodeAct、工具、记忆和基于 Actor 的协作。 | 可用，当前简单任务无需引入其协作机制。 |

Rig 的[嵌入式 Agent 运行库](https://github.com/0xPlaygrounds/rig/blob/main/crates/rig-agent/README.md)支持 Windows、Linux 和 macOS；独立终端编码 Agent 当前列出的运行平台为 Linux/macOS。本文使用嵌入式库，锁定依赖版本，避免将独立终端 Agent 的平台约束带入产品。

### 11.2 Prompt 模板如何组织

| 模板 | 注入内容 | 产物与完成条件 |
| --- | --- | --- |
| shared | 当前协议/Schema、实体身份规则、证据要求、工具清单、运行预算 | 所有任务遵守相同契约；动态值和缺失信息保留未知，不编造链路。 |
| investigate | 项目目标、文件范围、基础地图、代表性入口 | 项目约定、作用域、数据源、待提取能力和覆盖缺口，均附来源。 |
| collector | 选定能力、已调查约定、来源快照、脚本模板 | manifest、可运行脚本、依赖声明、验收样例；注释、模块、自定义入口与四类采集器使用同一输出格式。 |
| link | extract 后的实体快照、服务/数据源边界和实际绑定位置 | 有独立证据的跨层关系；同名、动态目标及绑定不明时返回候选和缺口。 |
| repair | 候选版本、结构化错误、失败样例、相关源码 | 修复后的候选和运行结果；不能通过删除验收样例、扩大忽略范围或修改公共协议来绕过失败。 |
| knowledge | 主题、已验证事实、原文引用、新鲜度和缺口 | 带来源与推断标记的主题产物，交给 Core 导入并渲染 Skill。 |

Prompt 不预设 NestJS、React、Vue、目录名或 ORM。框架约定由调查阶段写入上下文；项目可在 `.project-knowledge/prompts/` 补充业务词汇、注释规则和入口示例。契约与 Schema 从 Core 动态读取，模板只维护任务指令。产品 Skill 引用这些模板和协议，外部 LLM 可以复用同样的流程。

### 11.3 LLM 配置入口与使用方式

```sh
project-knowledge agent configure --config .project-knowledge/project.json
```

该目标命令配置 `.project-knowledge/project.json` 中的 agent 块。以下是本产品的配置格式，不是 Rig 的直接配置 API：

```json
{
  "agent": {
    "enabled": true,
    "provider": "<provider-id>",
    "model": "<coding-model-id>",
    "apiKeyEnv": "PROJECT_KNOWLEDGE_LLM_KEY",
    "maxTurns": 24,
    "maxRepairAttempts": 2,
    "timeoutSeconds": 600
  }
}
```

provider 使用已接入 Rig 的提供商标识；兼容端点可增配 baseUrl，由 config.rs 映射到对应客户端。API Key 从指定环境变量读取，不写入项目配置、插件制品或任务日志。启动任务时检查模型配置及工具调用能力；缺少配置只影响 Agent 命令，基础索引与普通查询仍可使用。外部 Pi/Goose 等 Agent 使用各自的模型配置，无需重复配置内置 Agent。

```sh
project-knowledge agent build --config .project-knowledge/project.json --focus comments,modules,entrypoints,routes,frontend,api,database --goal "围绕采购台账提取业务注释、模块和自定义入口，连接页面、请求、接口及表字段" --with-skill
project-knowledge agent repair --config .project-knowledge/project.json --plugin erp.purchase
```

build 先复用新鲜基础索引，缺失或过期时调用 Core 更新，然后执行调查、生成、验证和激活。focus 是采集能力选择，不限定框架；`--with-skill` 在索引更新后继续生成主题材料和项目 Skill。关闭 autoActivate 时停止在待激活状态，不将候选当成活动索引来生成正式 Skill。repair 读取指定插件的当前版本、失败原因及变化来源，产出新候选。单次任务采用一套总时间和轮数预算，默认最多修复两次；达到限额即保存诊断并停止，不把未完成任务标为成功。

### 11.4 Agent 工具与职责

| 工具组 | Core 操作 | 约束 |
| --- | --- | --- |
| 调查 | schema/status/search/code-map/read-source/export-snapshot | 有界读取选定项目；返回来源引用和快照，按需取原文。 |
| 写候选 | write-candidate | 只向本次候选目录写入插件、约定草稿和样例，不改目标业务源码；使用结构化路径和内容。 |
| 项目约定 | import-conventions | 发布带依据的调查材料，登记 documentation 来源，刷新依赖快照。 |
| 验证 | validate-plugin/run-plugin-cases | Core 执行运行时检查、受限子进程采集和验收，返回错误码、位置及失败样例。 |
| 激活 | activate-plugin | 只能激活校验通过且来源仍匹配的版本；注册和事实写入统一经过 Core。 |
| 知识输出 | export-topic/import-topic/generate-skill | 核对引用与来源版本，使用同一套知识材料与渲染逻辑。 |

这些是产品工具名，不依赖 Rig 的插件格式。维护 MCP 映射同一 Core 操作，SDK 查询统一事实即可，不为每个采集插件增加专属查询接口。初期不暴露任意 shell 工具；脚本及依赖运行由 Core 按 manifest 管理，使用超时、取消、输出限制和任务专属目录。子进程隔离不等于操作系统沙箱，需单独处理任何明确需要的额外执行能力。

### 11.5 候选验证、自动激活与失败处理

1. **生成候选**：Agent 在 work/candidates 下写入 manifest、脚本、锁定的依赖声明和验收样例；Core 根据实际文件计算制品哈希。运行时缺失时返回诊断，不将候选标为可运行。
2. **校验并试运行**：Core 检查协议版本、入口及文件范围，按 manifest 准备依赖并运行插件；校验实体身份、输出 Schema、证据路径/哈希/位置、依赖、关系引用及声明的覆盖范围。原始输出保存在候选任务中，不直接覆盖活动索引。
3. **验证完整链路**：在临时索引快照中先导入 extract 结果，再执行 link 和验收样例。以源码中的代表性页面、误关联案例和增量场景核对；已有人工真值样例保持独立。Agent 新写的样例与自身输出一致，只能证明内部一致性，不能作为精准度提升的独立证据。
4. **有限修复**：将结构化错误和失败样例交给 repair 模板。预算内重试；仍失败则记录错误，保留当前活动版本。源码已经变化时，旧版相关事实仍标为 stale，不因回退而重新变成 fresh。
5. **自动激活**：通过后先将制品发布到不可变版本目录，再复核验证所用的输入和活动版本；发生并发变化则重跑或返回冲突。随后由 storage 在同一事务中切换活动版本、替换该插件负责的事实、更新受影响搜索投影与依赖并推进 generation。成组插件在全部必要阶段成功后一起切换，失败回滚；未激活的制品不参与采集。
6. **自动加载**：index/collect 启动时扫描配置的插件目录并核对活动注册和制品哈希。autoLoad 控制自动发现和加载；autoActivate 允许验证通过的候选自动切换活动版本。发现一个 manifest 不代表立即执行或替换活动版本，所有新候选都经过相同验证入口。
7. **日常更新**：已激活插件直接运行，不需要模型参与。来源文件、匹配集合、配置或制品变化触发失效和重跑；新框架约定或新业务能力需要生成代码时再启动 Agent。SDK/MCP 查询只读取当前索引与覆盖状态。

插件统一声明 ID、版本、协议版本、能力、阶段、运行命令、输入选择、依赖和替换范围，统一输出实体、关系、证据、覆盖缺口与依赖；公共字段由 `core/contract/collector.rs` 定义并生成 Schema。生成脚本使用 Node.js、Python 或其他合适运行时，后续采集无需 Rig/Pi/Goose。精准度通过来源证据和独立验收评估，采集不到的内容明确返回缺口。
