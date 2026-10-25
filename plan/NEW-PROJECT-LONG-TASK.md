# 新项目持续实施任务

编制日期：2026-10-25。本文只定义待实施工作，不表示代码、命令或测试已经存在。

## 1. 目标、交付物和明确范围

### 1.1 事实基线

| 资料或现场 | 已确认内容 | 执行含义 |
| --- | --- | --- |
| `docs/NEW-PROJECT-DESIGN.md` | 已完整阅读；功能目标清单 | 功能要求，不是已实现能力清单 |
| `docs/NEW-PROJECT-STRUCTURE.md` | 已完整阅读，共 659 行；明确声明目录、命令、配置和接口尚未实现 | 本文具体化其契约；冲突按第 2 节决策解决 |
|  |
| 当前工作目录 | 存在两份设计、旧 SQLite、`target/`、`node_modules/` 和旧发布 workflow；没有根 `Cargo.toml`、`package.json`、锁文件或可用源码；不是 Git 工作树 | 缓存、数据库和 workflow 不证明功能存在，不复用旧表结构或发布命令 |
| 当前工具 | 本次只读取版本：Rust/Cargo 1.95.0、Node 24.14.0、pnpm 9.15.9、Python 3.10.16 | 不等于目标依赖可编译；T00 解决工具链差异。本次未构建或测试新项目 |

新源码默认落在当前目录的 `new-project/`，即 `E:/Project/my-project/code-graph/new-project/`。下文产品源码路径均相对此目录；目标业务项目路径均以 `.project-knowledge/` 开头。实施时可将整个新目录独立为仓库；不得从旧数据库、旧 native 包或缓存恢复所谓实现。

### 1.2 必交付能力

1. 无 LLM、无业务数据库连接时，CLI 可接入多仓库并建立文件、文本和受支持语法的基础索引，查询证据及覆盖缺口。
2. Rust Core 统一拥有协议、插件运行与校验、SQLite、索引、搜索、图查询、业务事实组合、证据、增量、知识材料导入导出。
3. 一个简单 Rust Rig Agent 使用通用 Prompt 完成调查、生成、验证、有限修复、自动登记与激活；之后普通采集只运行插件。
4. Node.js/Python 插件通过独立 JSON 协议接入。外部 Pi、Goose 或其他编码 Agent 可提交同一制品，无需依赖或嵌入它们的 SDK。
5. TypeScript SDK 和 stdio MCP 调用同一个 Rust Core；具备三个业务场景及失败、歧义、过期、增量的独立真值验收。
6. 生成带源码引用、版本清单、推断和缺口的业务主题、`SKILL.md`、参考文档，并提供可安装的 CLI/native/SDK/MCP 制品。

初期不做：旧数据迁移或接口兼容、前后端产品 UI、在线业务数据库探测、SQL 改写或优化器、多 Agent 平台、通用工作流引擎、后台守护服务、任意 shell 工具。SCIP 和 Repomix 是可选后续接入，本版不依赖、不宣称支持；基础索引必须在不安装它们时成立。

### 1.3 执行规则

- 仅执行依赖均为 `done` 的任务。状态为 `todo / doing / blocked / done`；所有任务初始 `todo`。`blocked` 必须写阻塞原因和可独立推进项，不得伪造通过记录。
- 任务完成须同时有实现、可复现验证和证据；先修复影响本阶段的契约问题再进入下阶段，不统一拖到联调。
- 第 3 节是目标契约。修改必须在执行记录写明决策、影响任务和相应 Schema/测试；不得只在某个 SDK 或插件里私改字段。
- 所有产品命令和自定义验证入口均为**目标接口**，由对应任务创建后才能执行。`cargo`、`pnpm`、`node`、`python` 是已知工具，不代表对应 package、脚本或测试已存在。
- 证据默认保存新项目内的 `docs/evidence/<任务ID>/`：工具版本、完整命令、退出码、输入/制品/真值哈希、有界输出和断言报告；密钥、原始凭据、完整模型请求不进入证据。

## 2. 设计缺口及已作出的实现决策

### 2.1 决策表

| ID | 缺口或冲突 | 本版确定决策 | 落实任务 |
| --- | --- | --- | --- |
| D01 | 设计目录易被误认为现有实现；历史资料缺失 | 独立 `new-project/` 从零建设，历史参考不作为依赖；缺失记录不阻塞基础建设 | T00、T01 |
| D02 | Core、Agent、SDK/MCP 可能各自持久化和验证 | Core 单一契约与存储 owner；Node 仅转换/调度，SDK/MCP 不连 SQLite、不实现图遍历 | T02、T04、T20、T21 |
| D03 | 稳定身份、同名、跨库和跨仓关系未落地 | 身份包含明确作用域；不按名称全局合并；同一实体的多生产者断言独立存储；声明和推断不覆盖事实 | T02、T04、T13 |
| D04 | extract 先提交与整组原子切换相冲突 | 候选和默认索引组在临时库先提交 extract，再导出 link 输入；正式库一次切换整组。显式分阶段 collect 允许单阶段提交，但立即使下游 link 失效 | T08、T09、T10 |
| D05 | 文件替换、跨插件共享实体和空输出删除不明确 | v1 插件阶段按整个项目替换自己的断言批次；不做文件分片替换；共享实体身份不删除其他插件断言；成功空输出确实清空该阶段旧批次 | T04、T08 |
| D06 | 只记录已引用文件无法发现新候选和新增文件 | 每次扫描重算文件选择集合；每个消费已有实体的阶段登记候选选择器和完整集合摘要，包括零匹配和歧义集合 | T03、T08、T10、T13 |
| D07 | 候选失败或版本回退可能使旧事实错误 fresh | 失效事务先提交；旧批次失效为单向状态。失败只保留诊断与旧制品；回退必须按当前输入重新运行、验证、提交 | T09、T10 |
| D08 | 自动发现是否等于自动执行不明确 | `autoLoad` 扫描并加载登记状态；新制品一律进入候选验证。`autoActivate` 仅切换已验证制品；仅维护入口可发现/执行/维护 | T07、T09、T21 |
| D09 | 查询新鲜度与来源读盘可能产生写入 | 默认查询验证相关来源和集合，限额内不能确认则 `unknown`；状态检查可读盘但不写库、不迁移、不执行插件或 LLM | T05、T15、T20、T21 |
| D10 | 短路由、中文词无法直接由 FTS trigram 满足 | 路由/身份精确索引优先；Rust 生成中文单字、双字 token 与标识符拆词，存入普通 FTS5；不把 trigram 宣称为任意中文分词器 | T05、T15 |
| D11 | 调用链、最短路径可能被当成业务结论 | `calls` 不产生 `transitions_to`；流程需要状态、条件或原始流程资料；台账输出已有实现和逐项口径，缺项返回 gap | T14、T16 |
| D12 | Rig 文档可能混用不同 crate 和 API | 选择 `rig = "=0.45.0"` facade 的简单 Agent；Core 不依赖 Rig。使用已发布包的 MSRV，不能照抄网页安装示例的较低 MSRV | T00、T18、T19 |
| D13 | MCP SDK 主版本和 Schema 接口未固定 | v2 `@modelcontextprotocol/server` / `client`，目标 2.4.0，stdio；Rust Schema 经小型 Standard Schema 适配器登记，校验仍调用 Core | T00、T02、T21 |
| D14 | Node-API AbortSignal 是否能停止已运行任务 | 使用请求 ID 和 Core 取消令牌/SQLite interrupt；不依赖 AsyncTask 排队取消去中止已运行计算 | T20 |
| D15 | 模型生成内容可能递归成为“事实证据” | 来源区分原始资料、派生约定和生成知识；派生约定须追溯原始依据；生成知识不得进入事实采集选择集合 | T02、T17、T19 |
| D16 | 依赖准备与插件执行混合，不可复现 | 验证时由宿主准备带锁依赖到制品哈希缓存；采集时只用已准备依赖，无网络安装、无模型调用；不继承业务项目 node_modules | T07 |
| D17 | 发布包名、平台支持和安装要求不明确 | 内部 Rust 包固定 `project-knowledge-core/agent/cli`；npm 暂用 `@project-knowledge/native/sdk/mcp`，公开命名权限在发布前核验；平台见第 6 节 | T01、T23 |
| D18 | 语法分析与真正的语言解析混淆 | 内置 TS/TSX/JS/Python Tree-sitter 基础声明与保守引用；缺少类型/实例证据的跨文件调用保持 candidate；其他文本仍可搜索 | T05 |
| D19 | 台账 SQL 与 ORM 支持易扩大为通用引擎 | 插件复用现成 AST 库，识别已调查的 ORM 模式；Core 保存统一语义。执行数值真值只针对本地 fixture，不执行用户 SQL | T12、T14、T16 |

### 2.2 外部依赖与 API 核对

以下为 2026-10-25 的官方文档、源码或包元数据核对；**未做本项目编译验证**。精确候选版本不是已通过的兼容性声明。T00/T01 将依赖及传递依赖写入锁文件，随后构建禁止使用浮动 `latest` 或 Git `main`。网页链接可滚动，证据须另记精确版本/源码提交。

| 依赖 | 核对来源与确切边界 | 采用约束与验证门禁 |
| --- | --- | --- |
| Rig | [官方安装](https://rig.rs/docs/installation)、[rig 0.45.0 API](https://docs.rs/rig/0.45.0/rig/)、[已发布版本元数据](https://crates.io/api/v1/crates/rig/0.45.0)。网页安装示例为 0.44.0/Rust 1.95+，0.45.0 元数据却要求 Rust 1.97.1 | 本版固定 `rig = "=0.45.0"`，Rust 1.97.1；T00 首先实际安装/编译确认。不能用当前本机 1.95 的存在证明可构建 |
| Rig Agent/工具/Hooks | [Agent](https://rig.rs/docs/concepts/agent)、[工具](https://rig.rs/docs/concepts/tools)、[Hooks](https://rig.rs/docs/concepts/hooks)描述工具循环、轮数和错误回传 | 使用 `AgentBuilder`、受控工具、总预算；0.45.0 的客户端构造、任意凭据变量映射、baseUrl、结构化错误和取消须 T00 最小例确认。对外配置不是 Rig 直接 API |
| SQLite/rusqlite | [rusqlite 0.41.0](https://docs.rs/rusqlite/0.41.0/rusqlite/)、[隔离](https://www.sqlite.org/isolation.html)、[Online Backup](https://www.sqlite.org/backup.html)、[FTS5](https://www.sqlite.org/fts5.html) | 固定 0.41.0，启用 bundled、backup；T00 探测实际 `sqlite_version()`、JSON 函数、FTS5、WAL、外键。临时库用 backup API，禁止只复制活跃 WAL 数据库主文件 |
| Schema | [schemars 1.2.3](https://docs.rs/schemars/1.2.3/schemars/)、[jsonschema 0.60.1](https://docs.rs/jsonschema/0.60.1/jsonschema/) | 固定上述版本；JSON Schema 2020-12；拒绝远程 `$ref`，只允许已登记本地 Schema；Serde 和 Schema 用相同用例验证 |
| 文件扫描 | [ignore 0.4.33](https://docs.rs/ignore/0.4.33/ignore/)、[globset 0.4.20](https://docs.rs/globset/0.4.20/globset/) | 固定上述版本，显式设 hidden、父级/global ignore、Git 条件与 glob separator；不依赖默认值；T03 验证无 Git 仓库亦生效 |
| 语法 | [Tree-sitter 0.27.1](https://docs.rs/tree-sitter/0.27.1/tree_sitter/) | 固定 0.27.1；TS/JS/Python grammar 的精确版本及 ABI 在 T00 锁定，加载和 UTF-8 位置转换必须实测；不声称自动理解框架 |
| Rust 进程 | [Tokio Command 1.54.1](https://docs.rs/tokio/1.54.1/tokio/process/struct.Command.html)、[Windows Job Objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects) | Tokio 固定 1.54.1；process、time、io-util、rt-multi-thread、macros；Unix 进程组、Windows Job Object 负责进程树终止；`kill_on_drop` 不独自证明后代已终止 |
| 模板 | [Handlebars 6.4.4](https://docs.rs/handlebars/6.4.4/handlebars/) | 固定 6.4.4，strict 模式；Markdown 转义由模板约束；模板只组织已验证输入 |
| Node binding | [napi-rs 入门](https://napi.rs/docs/introduction/getting-started)、[AsyncTask](https://napi.rs/docs/concepts/async-task) | napi/napi-derive/@napi-rs/cli 固定 v3 系列精确 patch（T00 记录），Node-API 8；官方构建最低要求与产品 Node 24 基线分别检查 |
| MCP | [官方 SDK](https://modelcontextprotocol.io/docs/2026-07-28/sdk)、[v2 README](https://github.com/modelcontextprotocol/typescript-sdk)、[server 2.4.0 元数据](https://registry.npmjs.org/@modelcontextprotocol%2fserver/2.4.0)、[Schema 接口源码](https://github.com/modelcontextprotocol/typescript-sdk/blob/main/packages/core-internal/src/util/standardSchema.ts) | server/client 2.4.0；协议目标 2026-07-28，实际协商结果记录；不混用 v1 包路径。`~standard.validate` 调 Core，`~standard.jsonSchema` 返回同源 Schema；T00 测 tools/list/call |
| TS 类型生成 | [json-schema-to-typescript](https://github.com/bcherny/json-schema-to-typescript)、[16.0.0 元数据](https://registry.npmjs.org/json-schema-to-typescript/16.0.0) | 固定 16.0.0；T02 验证 `$defs`、联合类型、optional/null、不允许额外字段的生成一致性；不另写 TS 真相模型 |
| 示例插件 AST | [TypeScript Compiler API](https://github.com/microsoft/TypeScript/wiki/Using-the-Compiler-API)、[Python ast](https://docs.python.org/3.12/library/ast.html) | TS AST 插件固定 6.0.x 的已发布精确 patch，明确 `<7`；官方说明现有 Compiler API 文档面向 6.0 及以前，不随 npm latest 7.x 升级。Python 示例固定 3.12.x；T00 检查非 ASCII 位置 |
| SQL 示例插件 | [SQLGlot API](https://sqlglot.com/sqlglot.html)、[30.24.0 元数据](https://pypi.org/pypi/sqlglot/30.24.0/json) | 固定纯 Python SQLGlot 30.24.0，显式 dialect；仅复用解析/AST，不使用优化器推导业务正确性。SQLGlot minor 也可能破坏兼容，禁止宽松 minor 升级 |

其余基础库使用 serde 1.x、serde_json 1.x、sha2 0.10.x、clap 4.x、thiserror 2.x、tokio-util 0.7.x；T01 选已发布精确 patch 并登记官方版本页、license 和 MSRV。Node 24.14.0、pnpm 10.x 精确 patch 在 T00 固定；Python 3.12.x 仅是 Python 插件/开发样例要求，不是 Core 安装要求。

### 2.3 未验证事项及处置

| ID | 待验证事项 | 负责与不通过时行为 |
| --- | --- | --- |
| U01 | Rig 0.45.0 在四个平台的构建、MSRV、工具错误保真和真实模型工具调用 | T00 本地最小例、T18/T19 故障模拟与真实模型、T23 平台构建；不支持的 provider/端点不得列入 capabilities |
| U02 | Tree-sitter grammar ABI、napi v3、Schema 生成与 MCP 2.4.0 组合 | T00、T02、T20、T21；失败先锁兼容 patch 或记录显式版本变更，不自行写替代解析器/协议库 |
| U03 | Windows 子进程树清理、文件发布/重命名、SQLite 并发崩溃恢复 | T07、T09、T10、T23；失败阻塞该平台发布，不能仅将其标为一般 gap |
| U04 | 真实项目约定、有效状态、字段口径和独立真值 | T06、T11—T16、T22；fixture 通过只说明 fixture 范围，真实项目必须独立复核，未知口径返回 gap |
| U05 | 历史审查文件恢复和 npm/发布渠道权限 | T00 记录历史资料缺失；T23 核验发布身份。历史文件缺失不阻塞编码，公开发布权限缺失只阻塞公开发布 |

## 3. 最终模块、协议和存储契约

### 3.1 模块与依赖

| 模块/目标文件 | 职责与依赖 |
| --- | --- |
| `crates/core/src/contract/` | 唯一 Rust DTO、错误、manifest、查询、知识契约；serde/schemars 导出，不引用 Agent |
| `core/src/project/`、`evidence/` | 范围、身份、扫描、不可变来源内容、位置校验和证据读取 |
| `core/src/storage/`、`crates/core/migrations/` | 唯一 SQL owner、migration、读快照、批次替换、失效和激活事务 |
| `core/src/syntax/`、`search/`、`graph/`、`codemap/`、`query/` | 基础语法、检索、双向图查询、代码地图、三个场景的确定性事实组合 |
| `core/src/collect/`、`index/` | manifest、依赖准备、运行、校验、临时库、登记、激活、DAG 排序、增量 |
| `core/src/knowledge/`、`crates/core/templates/` | 主题上下文、导入、版本核对、Skill 输出；不调用模型 |
| `crates/agent/src/{config,prompts,tools,runner}.rs`、`prompts/` | 仅依赖 Core 和 Rig；一个 Agent，有限任务状态机，不直接 SQL 或写业务源码 |
| `crates/cli/` | clap bin `project-knowledge`；默认 `agent` feature，支持 `--no-default-features` 的 Core-only 构建 |
| `bindings/node/{src/lib.rs,build.rs}` | napi v3；依赖 Core，不依赖 Agent；异步、取消、错误、句柄生命周期 |
| `packages/sdk/`、`packages/mcp/` | SDK → native → Core；MCP → SDK；维护客户端显式打开，普通客户端只读 |
| `collectors/templates/`、`collectors/examples/` | Node/Python 模板及两种独立布局样例；项目框架规则只存在插件和调查材料中 |
| `fixtures/`、`scripts/`、`xtask/`、`.github/workflows/` | 独立真值、契约生成、验证/打包入口、多平台交付 |

遵循结构资料的模块划分，按任务创建实际需要的文件，不预建空适配层。crate 路径缩写 `core/...` 在本节表内指 `crates/core/...`。

### 3.2 项目配置与扫描

`project.json` 的 `schemaVersion=1`。必须定义：

- `projectId`：显式、创建后稳定；`repositories:[{id,root}]`：`root` 相对配置文件，ID 不随机器绝对路径变化；根规范化后不得重叠，禁止同一物理目录登记两次。
- `selection:{include,exclude,respectGitIgnore,maxFileBytes}`：glob 相对仓库，使用 `/`、大小写敏感；`maxFileBytes=2097152`。先硬排除生成/控制输出，再应用仓库 `.gitignore`/`.ignore`，最后 include/exclude，exclude 优先；显式 include 不能覆盖排除。`respectGitIgnore=false` 可关闭 Git 忽略。
- 硬排除 `.git/`、`.project-knowledge/work/`、插件制品/依赖缓存、索引文件及其 WAL、登记的 Skill 输出；`conventions.md` 仅通过显式 knowledgeInputs 导入。默认额外排除 node_modules、target、dist；变更排除须使相关批次失效。
- 隐藏文件不默认忽略；禁用机器 global ignore 和仓库外父级 ignore，仓库无 `.git` 时也按配置读取其 ignore 文件；忽略文件本身作为控制依赖。
- 不跟随 symlink/reparse point；记录 `SYMLINK_SKIPPED`。非 UTF-8、二进制、超限、不可读和路径大小写冲突分别登记 gap；不静默丢弃。路径禁止绝对值、`..`、NUL 和越界解析。
- `scopes:[{id,repositoryIds,roots,kind}]`，`kind=application|module|service`；roots 是 `{repositoryId,path}`。基础符号按同仓库最长匹配根选择 scope，同等长度不同 scope 视为配置冲突；没有匹配则用稳定的仓库默认 scope。服务跨仓库必须显式同一 scope；不同 scope 的同名符号不自动绑定。
- `services:[{id,scopeId,protocol,baseUrls}]`、`dataSources:[{id,scopeId,dialect,catalog,schema}]`，ID 人工配置或带原始证据的调查结果，不能由变量名猜测；不保存连接密码。`bindings` 显式登记客户端/代理→service、ORM instance→dataSource，附原始配置引用和依据。
- `knowledgeInputs:[{repositoryId,path,sourceClass}]`；原始业务文档/schema 导出需在此登记。生成的主题和 Skill 不可登记为原始资料。
- `plugins:{directories,autoLoad:true,autoActivate:true}`、`collectors:[manifestPath]`；路径相对配置文件；同 ID 同版本不同哈希为冲突，多版本可存在但只允许一个活动版本，绝不按文件名或最高 semver 自动选版。

扫描读取原始文件字节并保存 SHA-256；不改 CRLF、BOM、Unicode 或缩进。构建期间读前后元信息与哈希不一致则最多重读两次，再返回 `SOURCE_CHANGED`。文件集摘要包含排序后的 `(repositoryId,path,hash,selectionStatus)`，因此新增、删除、被忽略和零匹配变化均可检测。无法得到稳定文件集时不提交 fresh 快照。

### 3.3 协议 v1、身份、引用与证据

所有 JSON 字段使用 camelCase；Core Rust 可用 snake_case 并统一 serde 映射。`protocolVersion="1.0"`，`schemaVersion=1` 只用于配置；二者不得混用。主版本不匹配直接拒绝；v1 当前只接受已支持 minor，未知公共字段拒绝。扩展只能放 `extensions` 或命名空间类型，不能依靠忽略错误兼容。

| 概念 | 目标字段/规则 |
| --- | --- |
| Snapshot | `{indexId,generation,sourceSnapshotId}`；`indexId` 创建库时 UUID；`generation` 为无符号十进制字符串，每次对读者可见的数据/新鲜度/活动版本变更递增，失败仅写诊断不递增 |
| Source | `{id,repositoryId,path,sourceClass,language,contentHash,byteLength}`；`sourceClass=code\|config\|schema\|documentation\|derivedConvention\|generatedKnowledge`。source 身份按仓库和相对路径；源版本按 sourceId+contentHash |
| Entity | `{id,kind,scopeId,identity,name,attributes,basis,origin,evidenceRefs,extensions}`；实体身份和生产者断言分离；属性冲突分别返回断言，不以最后写入者覆盖 |
| Relation | `{id,kind,fromId,toId,qualifiers,basis,origin,resolution,evidenceRefs,extensions}`；`resolution=confirmed\|candidate`，候选指向各个已存在候选实体，不确定目标一个也没有时只输出 gap |
| 依据 | `basis=observed\|declared\|inferred`：原始代码/配置可观察结构、原始规范/资料声明、推断；`origin=static\|human\|ai`。AI 判断必须 inferred/ai；脚本来自 AI 不自动使全部结果 inferred，也不自动证明其准确 |
| Evidence | `{id,sourceId,contentHash,startByte,endByte,startLine,endLine,role}`；区间 UTF-8 原始字节 `[startByte,endByte)`，行从 1 开始、endLine 为最后覆盖行；非空范围。行列不参与身份计算；禁止 JS UTF-16 offset 直接写入 |
| 引用 | 协议批次关系可引用本批或输入快照明确提供的 `entityId`；已有端点还须记录消费的断言/候选集合依赖。查询 `entityRef={snapshot,entityId}`、`sourceReference={snapshot,evidenceId,sourceId,contentHash,startByte,endByte}` 原样传回，禁止客户端拼接 ID |
| Dependency | `{kind,key,digest}`；kind 为 `source\|sourceSet\|config\|artifact\|runtime\|candidateSet\|upstreamBatch\|prompt\|topicInput`；key 对应声明的文件、选择器或制品，digest 由 Core 计算/复核，输出不能自报任意新鲜值 |
| Gap | `{id,code,capability,scopeId,sourceRefs,candidateIds,reason,requiredEvidence}`；如动态目标、未配置、排除、解析失败、歧义、缺少口径、预算；可为空候选，但不可省原因 |
| 覆盖 | `coverage:[{capability,scopeId,sourceSetId,status,counts,gapIds}]`；status 为 `complete\|partial\|not_configured\|failed`；仅对声明范围谈 complete，不代表整个项目无遗漏 |

稳定 ID：Core 提供纯函数 `computeIds` 并随 Schema 发布测试向量。`id = <前缀> + sha256(UTF8(canonicalIdentity))`，SHA-256 使用完整 64 位小写十六进制；canonicalIdentity 为固定顺序 JSON 数组，内部允许字符串、整数、数组及键递归排序的对象，无多余空白，禁止浮点数；可选身份值统一用 `[]`（缺省）或 `[value]`（存在）编码，不能将缺省和空字符串混同；字符串原样，不做隐式小写或 Unicode 归一化。实体前缀 `ent_`、关系 `rel_`、证据 `ev_`、来源 `src_`；来源 identity=`[projectId,repositoryId,path]`，证据 identity=`[sourceId,contentHash,startByte,endByte,role]`。插件可计算但 Core 必须重算，不接受随机实体 ID。

| 类型 | 必需 identity 成分（除 projectId、kind 外） |
| --- | --- |
| file / symbol / component / operation | repositoryId、path、scopeId；符号追加 lexicalPath、symbolKind、normalizedSignature；匿名符号用父实体+AST 结构路径，不使用行号；同一规范身份碰撞则拒绝并要求补充 disambiguator |
| route | application scope、routerId、mode(hash/history)、原始声明 path；URL 查询参数与输入页面的 `/#/` 不参与实体 ID |
| httpEndpoint | serviceId、HTTP method 大写、已确认的注册路径模板；不随意删除末尾 `/`、合并大小写或动态 segment |
| graphqlOperation / graphqlField | serviceId、operationType、operationName 或 schema type+field；共享 `/graphql` 不是业务目标身份 |
| dataTable / dataColumn / dataKey | dataSourceId、catalog、schema、table、column 或有序 keyColumns；未确认数据源使用 scope 内 unresolved 身份并输出 gap，不允许跨源绑定 |
| module / entrypoint / workflowState / documentType / queryImplementation / reportField | scopeId、项目声明的语义 key 或来源文件+lexicalPath；流程状态追加 workflowId；业务名只能是检索属性 |

关系 ID 的 identity 为 `(projectId,kind,fromId,toId,规范化 qualifiers)`；qualifiers 包含条件、调用点或有序复合键映射等语义区分项，证据位置及生产者不进入关系 ID。断言 ID 在 Core 中包含 `(producerKey,batchId,entityId/relationId,payloadHash)`，因此相同身份可有多条独立证据，重复同批断言幂等。

内置 entity kind：上述各类型及 `event`、`request`、`title`、`comment`、`dataSource`、`service`。内置 relation kind：`contains`、`defines`、`imports`、`references`、`calls`、`renders`、`handles`、`triggers`、`wraps`、`requests`、`serves`、`resolves`、`reads`、`writes`、`maps_to`、`joins`、`transitions_to`、`precedes`、`relates_document`、`describes`。端点 kind 的合法组合在 T02 的类型注册表逐项声明，不允许任意连线。

扩展类型必须 `x.<pluginId>.<localName>`，在 manifest 注册 entity/relation 和 attributes/qualifiers JSON Schema、允许端点 kind；公共顶层结构、basis、位置和引用规则不可扩展。扩展 Schema 只可引用 bundled schema；不加载远程代码或远程 `$ref`。未知扩展可被通用搜索/图查询展示，但场景查询不凭名称赋予业务语义。

所有实体/关系断言至少一个原始 evidence；关系不能只拿两端实体的位置作为关联证据，必须覆盖实际绑定/调用/条件。来自原始文档的 declared 关系可以 confirmed，但只能表示“文档这样声明”，不可改称代码已实现。candidate 不因置信分数高而 confirmed。生成约定需 `derivedFrom` 原始证据链；AI 推断作为单独断言，不进入默认确定性 confirmed 路径。

业务载荷必须进入 T02 的同源 Schema：

- `joins.qualifiers={joinType,keyPairs:[{leftColumnId,rightColumnId,operator}],cardinality,condition?}`；keyPairs 按来源键顺序，operator 初版仅 `=`，不支持条件明确 gap；cardinality=`oneToOne|oneToMany|manyToOne|manyToMany|unknown`，附声明或验证依据，不能从列名推断。
- `transitions_to.qualifiers={operationId,trigger,guard,documentTypeId,keyPairs}`；guard 保存原始表达式/文档引用，不执行通用表达式引擎。`precedes` 另存业务先后依据，不能从 calls 转换。
- `queryImplementation.attributes` 保存 dialect、原始语句范围、grainColumnIds、joins、aggregates、validStates、timeFields、nullHandling、units；`reportField.attributes={queryId,outputName,sourceColumnIds,expression,transforms}`。确定字段必填；未确定项使用 `{status:"unknown",gapIds:[...]}`，不能把空数组解释为已确认“无条件/无来源”。不实现通用 SQL 求值器。
- 查询实体投影为 `{entityRef,kind,scopeId,identity,assertions:[{assertionId,producerKey,batchId,basis,origin,name,attributes,evidenceRefs}]}`，关系投影同理加 relationRef/fromId/toId/qualifiers，resolution 在断言中。采集 Entity/Relation 是生产者的一条断言，查询返回独立断言集合，不把冲突属性压成唯一值。

错误统一使用 `ErrorEnvelope={error:{code:string,message:string,retryable:boolean,runId?:string,phase?:string,issues:Issue[]}}`，其中 `Issue={code,instancePath,sourceReference?,expected?,actual?,repairHint?}`；可选字段缺省，不使用无意义 null。`instancePath` 用 JSON Pointer；消息、actual 和 stderr 均限额并脱敏。关键 code：`CONFIG_INVALID`、`PROTOCOL_UNSUPPORTED`、`SCHEMA_INVALID`、`IDENTITY_CONFLICT`、`REFERENCE_MISSING`、`EVIDENCE_INVALID`、`SOURCE_CHANGED`、`SNAPSHOT_EXPIRED`、`DEPENDENCY_CHANGED`、`CANDIDATE_SET_INCOMPLETE`、`RUNTIME_MISSING`、`DEPENDENCY_PREPARE_FAILED`、`PLUGIN_EXIT`、`PLUGIN_TIMEOUT`、`OUTPUT_LIMIT`、`ARTIFACT_CHANGED`、`ACTIVATION_CONFLICT`、`INDEX_MISSING`、`INDEX_STALE`、`CANCELLED`、`BUDGET_EXHAUSTED`、`MODEL_CONFIG_INVALID`、`BUSY`、`CLOSED`。Gap code 也从 Core 注册表导出，至少包含本计划的歧义、动态目标、缺失流程/口径、证据冲突、键映射不全和跳过文件类别。未知故障映射 `INTERNAL_ERROR`，不把堆栈传给模型。

### 3.4 SQLite 逻辑表、约束与事务

下表是 T04 migration 的必要字段契约，非已存在 DDL。`PK` 主键，`UQ` 唯一约束；所有 `*_json` 必须 `CHECK(json_valid(...))`，枚举 CHECK，引用显式 FK。一个项目一库，禁止 SDK 自建平行库。

| 表 | 关键字段与约束 |
| --- | --- |
| `schema_migrations`、`index_meta` | migration version/checksum；index_meta 单行 PK=1，indexId UQ、projectId、generation INTEGER>=0、currentSourceSnapshotId、configHash；API generation 转字符串并检查溢出 |
| `repositories`、`scopes`、`scope_roots` | repo id PK/root；scope id PK/kind；scope_roots PK(scopeId,repoId,path)；根/作用域 FK |
| `services`、`data_sources`、`project_bindings` | 显式 id PK、scopeId FK、协议/方言和身份字段、binding 两端身份及 evidence；配置快照保存每次变更依据，不保存密钥 |
| `sources`、`source_versions` | sources id PK、repoId/path UQ、currentHash 可空、selectionStatus；versions PK(sourceId,contentHash)，原始 content BLOB、byteLength、lineMap、sourceClass；删除保留版本，currentHash=null |
| `source_snapshots`、`snapshot_sources` | snapshot id PK/configHash/sourceSetHash/createdAt；成员 PK(snapshotId,sourceId)，hash 可空且带 selectionStatus；完整有序集合可重建 |
| `plugin_versions`、`active_plugins` | versions PK(pluginId,version)、artifactHash、manifestJson、runtimeDigest、artifactPath、validationId；同版本不同哈希拒绝；active PK(pluginId)、version FK、activationRevision |
| `runs`、`batches` | runs id PK、kind、stage、status（pending/running/succeeded/partial/failed/cancelled/interrupted）、inputSnapshotId、started/finished、errorJson；batches id PK、runId FK、producerKey、replacementKey、artifactHash、inputDigest、state（fresh/stale/unknown）、invalidatedGeneration；旧 invalidatedGeneration 不允许清空 |
| `current_batches` | replacementKey PK → batchId UQ/FK；replacementKey 为 `(pluginId,stageId,projectId)` 的规范串；基础层为 `(core.base,repositoryId)`，失效时可仍指向旧批次但查询排除 |
| `entities`、`entity_assertions` | entities id PK、kind、scopeId FK、identityJson UQ；assertions id PK、entityId FK、batchId FK、basis/origin、name、attributesJson、payloadHash；UQ(batchId,entityId,payloadHash) |
| `relations`、`relation_assertions` | relations id PK、kind、fromId/toId FK entities、qualifiersJson；assertions id PK、relationId FK、batchId FK、basis/origin、resolution、payloadHash；UQ(batchId,relationId,payloadHash) |
| `evidence`、`assertion_evidence` | evidence id PK、sourceId/hash 复合 FK source_versions、字节/行区间 CHECK；assertion_evidence PK(assertionId,evidenceId)，assertionId 引用独立 `assertions(id,kind,batchId)` 公共父表，禁止无外键的多态 owner 字段 |
| `dependencies`、`selectors` | dependencies PK(batchId,kind,key)，digest；selectors id PK、definitionJson、kind(sourceSet/candidateSet)、lastDigest；candidate 摘要含匹配 entity IDs、断言 payloadHash、有效批次与 freshness；零结果也有摘要 |
| `coverage`、`gaps` | coverage PK(batchId,capability,scopeId,sourceSetId)、status/counts；gaps id PK、batchId FK、code、payloadJson；未配置能力由配置范围与必要能力比较动态得出 |
| `search_documents`、`search_fts` | documents rowid PK、assertionId FK、entityId、scopeId、routeExact、text、tokens，UQ(assertionId,projectionKind)；FTS5 保存 rowid/tokens，不把失效文档当当前结果；同事务更新 |
| `validations`、`activations` | validation id PK、artifact/input/activeVersionMap/candidateSetDigest、truthHash、status、stagedResultHash；activation id PK、expectedGeneration、groupJson、status；记录 pending/committed/failed，用于恢复 |
| `topics`、`topic_inputs`、`knowledge_exports` | topic id/version 复合 PK、Markdown、snapshot、template/prompt/model/refsHash、state；inputs 绑定 source 版本/断言/集合；export id PK、topicVersions、generation、outputHash/path、state |

父表 `assertions` 与子表 entity/relation assertions 由同一 storage 函数插入并校验 exactly one subtype，子表 id 为 PK 且 FK assertions，batchId 必须与父表一致。身份表不存可冲突的业务属性，未被当前批次引用的身份行不参与查询。关系仅在自身断言和两端至少一条匹配 freshness/basis 过滤的当前实体断言均有效时可见，不因身份行仍保留就暴露悬空边。v1 保留旧批次和来源版本用于诊断，不做自动 GC；按显式清理策略做后续任务，不影响当前结果。

必要索引：`sources(repoId,path)` UQ；`entity_assertions(entityId,batchId)`、`assertions(batchId,kind)`；`relations(fromId,kind,toId)` 和 `(toId,kind,fromId)`；`relation_assertions(relationId,batchId,resolution,basis)`；`entities(scopeId,kind,id)`；`dependencies(kind,key,batchId)`；`search_documents(scopeId,routeExact,entityId)`；`evidence(sourceId,contentHash,startByte)`；`runs(status,startedAt)`。实体业务名称只作检索投影，不作跨 scope 唯一键。

数据库以 WAL、`foreign_keys=ON`、`busy_timeout=5000`、`synchronous=FULL` 初始化；不使用网络文件系统数据库。migration 仅维护连接执行，新版数据库被旧客户端只读打开时返回版本错误，不自动降级。每个组合查询在一个独立只读连接的读事务内完成；查询上下文不得跨线程共享裸 Connection。写入用短 `BEGIN IMMEDIATE`，脚本/模型/文件准备在事务外。

必须实现的事务边界：

1. **失效事务 A**：检测来源/配置/制品/候选集合变化，记录新扫描与依赖状态，使受影响批次和下游批次 stale，更新当前搜索可见性及知识状态，generation+1 后提交。失败采集不能撤销 A。
2. **结果事务 B**：对目标 replacementKey 全量替换断言、证据引用、依赖、coverage、搜索投影和 current_batches；成功空输出同样替换；验证任一失败整组回滚。身份行可复用，其他生产者事实不动。
3. **激活事务 C**：按 expectedGeneration/activeVersionMap/inputDigest 做 CAS；一次写活动制品、整组批次与下游 link、投影、知识失效和 generation。CAS 不符返回冲突，不能仅切活动指针。
4. **诊断事务**：保存 run/validation/失败信息，不更改当前事实或使旧批次 fresh。中断后按 runId 可判断 C 是否已提交。

### 3.5 插件 manifest、输入输出和执行

manifest v1 的目标形态如下；ID 和选择器均为示例，不代表已实现插件：

```json
{
  "protocolVersion": "1.0",
  "pluginId": "example.purchase",
  "version": "0.1.0",
  "capabilities": ["routes", "frontend"],
  "runtime": {"kind": "node", "versionRange": ">=24 <25"},
  "artifacts": ["manifest.json", "src/collect.mjs", "cases/basic.json", "deps/package.json", "deps/package-lock.json"],
  "dependencies": {"kind": "npm", "manifest": "deps/package.json", "lockFile": "deps/package-lock.json"},
  "stages": [
    {
      "id": "extract",
      "phase": "extract",
      "entry": "src/collect.mjs",
      "args": ["--stage", "extract"],
      "sourceSelectors": [{"id": "web", "repositoryIds": ["repo.web"], "include": ["**/*.ts", "**/*.tsx"], "exclude": []}],
      "entitySelectors": [{"id": "baseSymbols", "kinds": ["symbol"], "scopeIds": ["app.web"]}],
      "dependsOn": [],
      "replacement": "project"
    },
    {
      "id": "link",
      "phase": "link",
      "entry": "src/collect.mjs",
      "args": ["--stage", "link"],
      "sourceSelectors": [{"id": "bindings", "repositoryIds": ["repo.web"], "include": ["**/*.ts"], "exclude": []}],
      "entitySelectors": [{"id": "targets", "kinds": ["request", "httpEndpoint"], "scopeIds": ["app.web", "svc.api"]}],
      "dependsOn": ["example.purchase:extract", "example.backend:extract"],
      "replacement": "project"
    }
  ],
  "limits": {"timeoutMs": 60000, "maxInputBytes": 33554432, "maxOutputBytes": 33554432, "maxStderrBytes": 1048576},
  "acceptanceCases": ["cases/basic.json"],
  "extensions": {}
}
```

`dependsOn` 可跨插件，宿主验证 DAG、阶段存在与协议相容；extract 只能依赖基础层或 extract，link 在必要 extract 成功后执行，可依赖更早 link；不允许环、自依赖或依赖任意活动版本来绕过候选验证。`entitySelectors` 默认显式读取基础符号以避免重复创建 symbol；非生产者只能引用其身份或增加自己的断言，不得删除/更改他人事实。

`runtime.kind=node|python|executable`；node/python 映射宿主配置的绝对可执行路径。executable 仅允许项目显式配置的 runtime ID，不接受 manifest 中任意 shell 字符串。entry/artifacts/锁文件必须在候选目录内、拒绝 symlink。Core 计算 `artifactHash`，manifest 不自包含其自身哈希；哈希输入为排序后的规范路径与逐文件内容哈希，另绑定 Schema、运行时版本和依赖锁摘要。

依赖准备：无依赖允许省略 dependencies；Node 使用独立缓存中的 `npm ci --ignore-scripts`，锁文件为 package-lock；Python 使用独立 venv 和 `pip install --require-hashes -r requirements.lock`，仅允许带哈希的可用 wheel，缺少 wheel 报错，不回退执行源码构建。生命周期安装脚本默认拒绝，确有需要必须由项目配置显式登记；安装有独立 120 秒超时和日志上限。网络只用于显式 prepare，不在 extract/link 隐式安装。缓存摘要包括 OS/arch、runtime、artifact、lock；运行前验证缓存完整性。

stdin 一个 UTF-8 JSON 对象，EOF 表示完成：

```text
CollectorInput = {
  protocolVersion, runId, pluginId, version, stageId,
  snapshot, inputSnapshotId, configSubset,
  sources:[Source + {content}],
  sourceSets:[{selectorId,digest,sourceIds,complete}],
  entities:[{entityId,kind,scopeId,identity,assertions}],
  relations:[{relationId,kind,fromId,toId,qualifiers,assertions}],
  candidateSets:[{selectorId,digest,entityIds,complete}],
  allowedExtensions, limits
}
CollectorOutput = {
  protocolVersion, runId, stageId, inputSnapshotId,
  entities:[], relations:[], evidence:[], dependencies:[], coverage:[], gaps:[]
}
```

只有 sources 包含内容、实体范围仅按声明选择器导出；宿主导出的清单/完整性和摘要具有权威性。CollectorInput 的 snapshot 是正式基线，inputSnapshotId 另绑定已成功的 staging 上游批次；这些 entityId/relationId 只在本次输入中可引用，未激活的新实体不能伪装为正式 Query 的 EntityRef。v1 不分页 stdin，超输入限额返回 `OUTPUT_LIMIT`（issue 指明 input），由调用方缩小插件作用域或显式提高有上限配置；不得截断候选后继续宣称唯一匹配。输出数组空也必须存在；stdout 只能一个对象，调试日志写 stderr。退出码非零、混杂日志、多对象、非法 UTF-8、引用缺失均整阶段失败；不导入部分 JSON。

每次执行使用任务临时 cwd，输出大小、stderr、耗时和并发（默认 2）有硬上限。环境变量白名单含 PATH、必要系统/临时目录变量及显式插件配置；不继承模型凭据。进程组/Job Object 在取消、超时和宿主恢复时清理。插件处理宿主提供快照，禁止导入并执行目标业务代码。工作目录和 env 控制不等于 OS 沙箱；本版只运行本机受信任插件，不声称限制其一切系统调用或强制硬内存沙箱。

插件验收 case：`{id,inputFixture,expected:{entities,relations,forbiddenRelations,gapCodes},truthRef,truthHash}`；可按稳定身份比较，不把位置变化误判身份变化。Agent 自建 case 标记 `origin=agent`，只证明内部一致性；固定外部真值由 T06 创建，候选工具无修改权限。验证输出包括每条失败路径和修复提示。

### 3.6 自动激活、失效与恢复

候选状态：`generated → prepared → validated → published → active`；失败 `failed`，被新候选替代 `superseded`。validated 必须绑定：artifactHash、inputSnapshotId、sourceSet/candidateSet 摘要、configHash、runtimeDigest、活动版本映射、truthHash、临时结果哈希。

1. 在 `.project-knowledge/work/candidates/<runId>/` 接收候选；在独立 staging SQLite 用 backup 得到一致快照，去除本次 replacementKey 的旧断言，再执行 extract→link→验收；正式库保持旧活动版本及准确 stale 状态。
2. 验证通过后发布 `.project-knowledge/collectors/<pluginId>/<version>/<artifactHash>/`，先写同磁盘临时目录、flush、再 rename；只发布完整制品。设计资料中的 `<version>/` 下增加哈希一级以明确不可变身份。依赖环境在独立 `work/deps/<digest>/`，不混入源码制品。同一 version 在发布前可修订候选，发布后不同 hash 必须用新 version 并重新验证，不覆盖已发布版本。
3. 发布不等于激活。重新核对候选内容、真实文件/选择集合/配置/runtime 和 activeVersionMap；有变更即验证失效，返回冲突或由外层限额重跑，不执行盲重试。
4. 正式事务 C 原子切换插件组及其受影响活动 link 的闭包；同组任一必要阶段失败则整组不切换。没有受影响的既有 link 不运行；已有下游依赖则必须纳入组或保持其 stale 并将组明确声明 partial，正式“完整激活”不能省略该闭包。
5. 制品存储和 SQLite 没有跨资源事务：先不可变发布，再数据库 CAS；崩溃留下的未引用制品不加载。数据库是活动指针唯一来源。不得把整个 SQLite 文件 rename 来替换正在使用的数据库。
6. 启动维护命令取得单项目 OS 文件锁（进程退出释放），再协调 SQLite 写锁；并发写任务返回 `ACTIVATION_CONFLICT` 或 `BUSY`，不无限等待。提交仍检查 generation，文件锁不能代替来源复核。
7. 宿主中断：未提交批次不当前可见；重启维护入口将旧 running 标记 interrupted、清理可识别进程和未完成 staging；若 C 已提交，以 activation/runId 确认成功，不重复激活。查询入口只报告，不修复。
8. 活动制品被修改、丢失或 hash 不符，立刻令相关批次 stale 并报 `ARTIFACT_CHANGED`；变化内容作为新候选，不覆盖活动版本。`autoLoad=false` 仍可使用明确登记的活动版本，停止目录发现；`autoActivate=false` 到 validated/published 即停，不切正式事实。
9. 全量与增量均重算来源选择集合。已消费文件内容变化、文件删除/新增、ignore/配置变化、artifact/runtime/锁变化、候选集合内容变化使依赖闭包失效。v1 重跑受影响阶段的整个项目范围，优先正确性，不做微分补丁。
10. `index --base` 只刷新基础层，同时使相关业务/知识 stale；`index --incremental` 重跑已激活受影响插件，不调用模型。显式 `collect --phase extract` 提交后使下游 link stale；link 输入从新批次重新导出。

批次 freshness 与 run status 独立：`fresh|stale|unknown`；run 可 succeeded 但 coverage partial。历史批次一旦失效，不因来源内容改回、候选失败、版本回退或重启而直接恢复 fresh；只有当前输入下的新验证运行产生 fresh 新批次。只读 freshness 检测发现未登记变化时在响应中覆盖计算 freshness，不写失效事务；下一维护入口再持久化失效。

### 3.7 查询、SDK、MCP 与 CLI

Core 暴露 `QueryRequest` tagged union，`operation` 为 `status|search|locateFeature|trace|codeMap|readSource|page|workflow|report`；`getCapabilities` 无索引也可调用。Node SDK 提供对应方法；MCP snake_case 工具名只做映射，JSON 参数字段不改名。维护调用使用独立 `MaintenanceRequest`，不得从 QueryRequest 派生隐式执行。

共同过滤：`scopeIds/repositoryIds/kinds`；根选择 `rootFilter` 与图遍历 `traversalFilter` 分开，筛根为 route 不得限制后续 endpoint/table 类型。`basisFilter` 默认 observed/declared，`includeCandidates=false`，`freshnessPolicy=verify|indexed|allowStale`，默认 verify。verify 在预算内核对相关文件和选择器集合，确认不了返回 unknown/partial；indexed 只说明库内状态，未核验磁盘时标记 `diskChecked=false` 且不能自称当前 fresh；allowStale 明确返回历史断言和警示，不能混成当前结论。

路由输入按已登记 application/router mode 解析：hash 模式取 `#` 后的路由 path，故 `/#/Other/purchaseLedger` 变为 `/Other/purchaseLedger`；history 模式取 URL path。路由内查询参数独立保留为请求上下文，不作身份；百分号编码只规范化一次，编码的 `/` 不当成路径分隔，非法编码报配置/请求错误；保留大小写、末尾 `/` 与声明差异。只有显式 alias 或路由模板证据允许匹配其他 path，多个 app/mode 同时可匹配则返回候选，不由输入形状猜应用。

| 操作 / SDK / MCP | 必需输入及特有输出 |
| --- | --- |
| getCapabilities / getCapabilities / schema 资源 | 无；协议/Schema hash、支持的类型/关系、操作和 limits、实际 provider/language/runtime，不宣称未构建 feature |
| status / status / index_status | `{checkFreshness:true}` 默认；base/collectors/knowledge 分项 freshness、coverage、activeVersions、gaps、扫描是否完整；false 等价 indexed |
| search / search / search | `{query,fields?,filters?,pageSize?,continuation?}`；fields 支持 businessTerm/route/title/comment/module/symbol/path/source；返回匹配断言、证据与排序原因 |
| locateFeature / locateFeature / locate_feature | `{query,routeHint?,rootFilter?,traversal?,view?}`；roots、paths、relatedCode、data、sourceReferences、gaps；不生成自然语言新事实 |
| trace / trace / trace_links | `{roots:[entityRef],direction:in\|out\|both,relationKinds?,maxDepth?}`；逐跳关系、方向、条件、断点、候选；不把 root kind 当遍历过滤 |
| codeMap / getCodeMap / get_code_map | `{query?,roots?,maxFiles?,maxSymbols?}`；确定性相关文件与符号轮廓、关联理由与未覆盖语言 |
| readSource / readSource / read_source | `{references:[sourceReference],maxResponseChars?}`；每条 `status=current\|expired\|missing`、原始片段/lineMap；核验磁盘变化，默认不把历史片段当当前；allowStale 时显式返回 storedVersion |
| page / locatePage / locate_page | `{route,scopeIds?}`；规范化路由、title/i18n 来源、操作、请求、HTTP/GraphQL、服务业务方法、表字段及缺失跳 |
| workflow / explainWorkflow / explain_workflow | `{query,roots?,scopeIds?}`；页面操作、前后页面、转换条件、状态、单据键、basis；无流程依据时返回 gap |
| report / explainReport / explain_report | `{query,roots?,scopeIds?}`；已有 queryImplementation、fieldSources、joinKeys、cardinality、grain、aggregates、validStates、conditions、unknowns |

统一响应：

```text
QueryResponse<T> = {
  protocolVersion, requestId, snapshot,
  status: complete|partial,
  data: T,
  sourceReferences: [], gaps: [],
  completeness: {
    freshness: fresh|stale|unknown, diskChecked, checkedAt,
    coverage: complete|partial|not_configured|failed,
    candidateSetsComplete: boolean, truncated: boolean, reasons: []
  },
  budgetUsed: {entities,relations,visited,elapsedMs,responseChars},
  continuation?: opaqueToken,
  nextQueries: []
}
```

`complete` 仅代表请求范围和预算内结果完整，不能表示业务全知；相关数据 stale/unknown、覆盖缺失、候选集合不全或预算截断均 partial。歧义在完整覆盖范围也用 partial 并附 `AMBIGUOUS_BINDING`。错误请求、过期引用、无库和非法 continuation 用统一 error；“无命中且覆盖完整”为 complete+空 data；“无命中且未采集”必须 partial。业务文本由调用方 LLM 解释，Core 不执行 LLM。

默认预算：maxEntities=60、maxRelations=120、maxVisited=2000、maxDepth=8、maxResponseChars=24000、timeoutMs=2000；服务端硬上限分别 1000、3000、20000、16、200000、30000；pageSize 默认 20、最大 100。字符预算按 Unicode scalar 计数，另设序列化 UTF-8 传输上限；预留 envelope 空间，不输出截断 JSON。取消返回 CANCELLED，预算达到返回 partial；不得继续无界遍历。

搜索稳定排序 `(exactRank,bm25,entityId,assertionId)`；邻居排序 `(direction,relationKind,relationId,assertionId)`。search/直接邻接支持无服务端状态的 keyset continuation，包含 indexId、generation、requestHash、最后排序键，编码并严格验参，变更 generation 返回 SNAPSHOT_EXPIRED，不跨代拼页。有界路径采用确定性 BFS，仅解释可达候选链，不承诺业务最优；截断返回 frontier 的有界 entityRefs 和缩小条件的 nextQueries，**不提供**可恢复整个 BFS 的 continuation。返回字段区分二者，不伪造完整路径分页。

跨请求 sourceReference/entityRef 必须同 indexId/generation，默认过期拒绝；证据可在显式 allowStale 模式读取保存的历史字节。单次读事务保证索引一致性；没有文件系统事务，响应仅保证核验时刻并附 checkedAt，采样后文件变化由下一次读取重新检出。

CLI 目标约定：JSON 输出统一 stdout，日志 stderr；成功完整 exit 0、partial/stale exit 2、请求/执行失败 exit 1、取消 exit 130。维护 validate-only 允许写诊断/临时目录，但不改 generation/活动事实；不得混同查询只读。

| 命令族（目标接口） | 明确行为 |
| --- | --- |
| `project-knowledge init --root <dir>` | 生成配置与空索引目录，已有配置不覆盖 |
| `index --config <file> --base` / `--incremental` | 基础更新 / 活动插件依赖更新；二选一 |
| `status --config <file> --check-freshness` | 只读核验；不建库、迁移或运行插件 |
| `query --config <file> --request <json-file>` | 接受统一 QueryRequest；便捷 `--operation search --text ...` 必须转换成同 DTO |
| `collect --config <file> --phase extract\|link [--plugin <id>] [--validate-only]` | 活动阶段运行；依赖不新鲜时报错，不跳过依赖 |
| `plugin submit/validate/activate/rollback --config <file> ...` | 外部 Agent 与内置 Agent 共用候选入口；rollback 是重新验证旧制品，不恢复旧批次 |
| `agent configure/build/repair/resume --config <file> ...` | 模型配置、调查生成、有限修复、中断后显式恢复；不进入日常查询 |
| `generate context/docs/skill --config <file> ...` | 无模型的 Core 上下文导出/主题导入/Skill 渲染；LLM 重写必须 agent 入口 |
| `project-knowledge-mcp --config <file> [--mode query\|maintenance]` | 默认 query；maintenance 显式增加写工具，不能由 prompt 或工具参数提升模式 |

### 3.8 Agent 与 Prompt 契约

`agent={enabled,provider,model,baseUrl?,apiKeyEnv,maxTurns:24,maxToolCalls:80,maxRepairAttempts:2,timeoutSeconds:600,maxInputChars:80000,maxOutputTokens:8192,autoActivate?}`。初版 provider allowlist 为 `openai|anthropic|openai-compatible`，每个只有 T18 实际验证后才进入 capabilities；兼容端点必须做工具调用/错误/取消 smoke，不因 HTTP 通就宣称支持。`apiKeyEnv` 默认 `PROJECT_KNOWLEDGE_LLM_KEY`，只在进程内读取并传客户端，不修改全局环境，不放进提示词、插件 env、日志或数据库。没有凭据只阻塞 Agent，不阻塞 Core。模型 ID 必须用户配置，不虚构默认可用模型。

Prompt 组合顺序：产品 shared → 调查/collector/link/repair/knowledge 模板 → 生成的当前协议/Schema/tool 列表 → 有引用的项目约定 → goal/focus/范围 → 有界上下文。项目 `.project-knowledge/prompts/` 只追加业务约定，不能覆盖协议、预算、真值和写入范围。源码、注释、业务文档作为带边界的数据，不作为工具执行指令。

工具 allowlist：`get-capabilities/status/search/code-map/read-source/export-snapshot/update-base/write-candidate/import-conventions/prepare-plugin/validate-plugin/run-plugin-cases/activate-plugin/export-topic/import-topic/generate-skill`。均调用 Core facade；没有任意 shell、SQL、网络抓取或业务源码写入工具。write-candidate 只接受当前 runId 下的结构化相对路径和内容，Core 防越界；并行工具写候选串行化，同一任务只有一个写 owner。

`import-conventions` 同时接收结构化 `identityDelta={scopes,services,dataSources,bindings}`、原始 evidence 和 expectedConfigHash。Core 只接受未冲突且有依据的新增/显式修改，不能按名字覆盖既有 ID；未确认建议保留在草稿。先原子写约定材料，再 CAS 原子更新 project.json 的身份和 knowledgeInputs；配置失败时新材料不登记使用并保存诊断。配置变更后先重扫、持久失效并取得新 inputSnapshotId，再生成或验证插件，不能复用变更前 validation。这样 Agent 无需任意配置文件写权限就能完成项目身份接入。

| 状态 | 输入 → 产物 | 通过条件/停止条件 |
| --- | --- | --- |
| preflight | 模型配置、goal/focus、当前状态 → run 预算/配置摘要 | 缺配置、无工具能力或运行时不满足即失败；基础过期可显式调用 update-base |
| investigate | 基础地图和有界源码 → conventions 草稿、来源引用、服务/数据源建议、gaps | Core 验证依据后登记 derivedConvention；未证实绑定保持候选，不自动写成 observed |
| generate | 当前 Schema、调查结果和模板 → manifest、脚本、锁文件、agent cases | 只写候选，制品和依赖可计算；必需的独立真值不可修改 |
| validate | 候选 + 当前快照 + 独立真值 → 结构化 issues、临时结果、validationId | 完整 Schema/引用/位置/集合/语义样例通过才 validated |
| repair | issues + 有界失败样例和源码 → 新候选修订 | 最多 2 次；不能删真值、放宽公共协议、扩大忽略范围或降低阈值；每次新 hash 重做验证 |
| activate | validationId + 预期活动映射 → activationId、正式 generation | Core CAS 提交；autoActivate=false 返回 pending_activation，不宣称成功，不生成正式当前 Skill |
| knowledge | 活动新鲜索引、topic/goal → 带引用主题产物 + Skill | Core 导入版本相符；partial 时产物保留 gaps，不能写成全覆盖；with-skill 未完成则整体任务 partial |

总轮数按实际模型请求计（包括工具反馈后续请求与重试），不能每阶段重置 24；每次 provider 重试也计预算，最多 2 次且仅限可重试错误；工具调用另计 80 次。600 秒包括模型、准备、验证和修复，子步骤 timeout 取剩余总预算的较小值；耗尽即取消 Core 子进程并保存失败状态。使用 Rig Hooks/工具包装器向模型回传结构化 issue，不能只回“工具失败”。终止由 Core 状态机核对产物决定，模型声称完成不代表完成。

中断保存 phase、candidate hash、validationId、source/input digest、预算累计及错误，不持久化密钥。`resume --run <id>` 复核来源和活动版本，预算不自动重置；预算耗尽需显式新 run，已提交激活用 runId 去重。知识生成失败不得撤销已成功插件，但整体返回 partial，说明已完成子步骤。

### 3.9 业务知识与 Skill

`TopicArtifact={protocolVersion,topicId,title,snapshot,inputDigest,markdown,claims:[{id,text,basis,origin,evidenceRefs,gapIds}],sourceReferences,gaps,promptHash,templateHash,modelInfo?}`。语义断言逐条引用，Mermaid 节点/边映射 claim/entity ID；无引用解释必须 inferred 并给出未知项。引用有效只证明来源可追溯，业务准确性仍按第 5 节独立真值验收。

Core 导出主题上下文时包含选中实体/断言、源码片段、代码地图、相关 coverage、候选集合摘要和预算；不把旧生成主题当正确性证据。导入时检查 indexId/generation、inputDigest、原始引用及推断标记，不允许通过另一个 topic 循环证明自身。derivedConvention 必须展开到原始 code/config/schema/documentation 依据；依赖环直接拒绝。

Skill 输出包含 `SKILL.md`、`references/{conventions,routes,frontend,api,database,code-map}.md`、`references/topics/*.md` 和 `references/index-manifest.json`。manifest 绑定 snapshot、source/input hashes、活动插件制品、topic versions、模板/Prompt hash、各文件 hash。SKILL.md 有 name/description、适用范围、先查 status 再读源码的调用导航和缺口入口；脚本入口只引用真实已安装 CLI/MCP，不写不存在的业务命令。

主题 freshness 按其输入依赖计算；无关 generation 变化可保留绑定旧 generation 但标明经过依赖复核，不能直接改写为“新生成”。相关来源、选择集合、插件、Prompt/模板或原始业务资料变化时 topic/export stale；`generate skill` 只重渲染结构化材料，不能自动把 LLM 旧主题改为 fresh。需 `agent build --with-skill` 或外部 Agent 重新 export/import 当前主题。写输出采用临时目录+逐文件清单+完整标记，失败保留上次完整输出，status 明确新版本失败/旧输出是否 stale。

## 4. 按依赖排序的实施阶段与任务

### 4.1 阶段门禁与验证命令

所有下列路径、Rust 集成测试名、pnpm script 和 xtask 子命令均为本计划要求创建的**目标接口**。工作目录是 `new-project/`；`cargo run -p project-knowledge-cli -- ...` 是开发版 CLI，不要求先安装全局 bin。测试中对时间敏感的断言使用可控时钟/同步点，不依赖随意 sleep。

| 阶段 | 任务 | 阶段完成后可运行流程 |
| --- | --- | --- |
| A | T00—T05 | 初始化多仓库 → 基础索引 → 文本/符号检索 → 读证据；全程无模型与 Node 插件 |
| B | T06—T10 | 独立真值 → 外部候选提交 → 验证 → 原子激活 → 无模型增量 → 失败后准确 stale |
| C | T11—T16 | 两种项目布局的采集 → 跨仓关联 → 页面/流程/台账事实查询与数值核验 |
| D | T17—T19 | 导出上下文 → Rig 调查/生成/有限修复 → 激活 → 导入主题和生成 Skill |
| E | T20—T22 | 同一索引由 CLI/SDK/MCP 查询维护 → 三端结果一致 → 端到端回归 |
| F | T23 | 四平台制品 → 干净环境安装 → 离线/模型/插件依赖分层验证 → 发布候选 |

T01 创建以下 script，脚本为跨平台 Node `.mjs`，避免依赖 Bash：

| 目标命令 | 行为/归属 |
| --- | --- |
| `pnpm run build:contracts` / `check:contracts` | 调 `cargo run -p xtask -- export-contracts`，生成 SDK 类型与 Schema；check 在临时目录重生成并比较，不能修改后再声称一致；T02 |
| `pnpm run build:native` / `build:sdk` / `build:mcp` | 依次构建当前平台 binding、SDK、MCP；未实现阶段应明确失败，禁止空脚本报绿；T20/T21 |
| `pnpm run typecheck` | SDK/MCP/生成类型的 `tsc --noEmit`，实际 workspace package 列表固定；T20/T21 |
| `pnpm run test:sdk` / `test:mcp` | Node 内置 test runner 执行各 package 的构建后测试；T20/T21 |
| `pnpm run test:plugins` | 运行 Core 驱动的独立 fixture/case，不由 TS 再实现协议校验；T11—T14 |
| `pnpm run verify:scenarios -- --scenario all --transport cli\|sdk\|mcp\|all` | 同一冻结真值的验收驱动，输出逐项断言与数值差异；T16 执行 cli，T22 补齐 all |
| `pnpm run verify:docs -- --plan ../plan/NEW-PROJECT-LONG-TASK.md` | 检查任务 ID、依赖 DAG、需求/验收映射、目标命令登记、文档链接；T01 基础、T22 完整 |
| `cargo run -p xtask -- package --target <triple>` | 构建 CLI、嵌入资源及 native 发布候选，核对版本清单；T23 |
| `pnpm run verify:packages -- --artifacts <dir>` | 临时干净 consumer 安装 pack 包/平台包并运行 SDK/MCP，用系统 PATH 排除编译工具；T23 |

Rust 测试目标按功能集中：`indexing`、`collectors`、`lifecycle`、`business_scenarios`，以及 Agent crate 的 `agent_lifecycle`；后续任务在所属 suite 添加有业务断言的 case。过滤执行的任务必须保存匹配测试数量，匹配 0 个视为失败。验收不要求把每个内部 helper 单独写测试。

### T00 — 来源复核、依赖锁定前的最小集成验证

- 状态：`doing`；前置依赖：无。
- 本轮结论（2026-10-26）：已确认 Rust 1.97.1 / Node 24.14.0 / pnpm 10.34.8 可用；`rig = "=0.45.0"`、`rusqlite 0.41.0`、`tree-sitter 0.27.1`、`napi 3.15.0`、`schemars 1.2.3`、`jsonschema 0.60.1` 已锁定并可编译；SQLite 的 FTS5 / JSON1 / 外键 / WAL / backup 与四类 Tree-sitter grammar 已实测通过。**未完成**：napi v3 实际加载、MCP 2.4.0 stdio 握手、Rig 工具循环最小例；本机没有可用的 Python 3.12，Python 侧向量无法执行。证据：`docs/evidence/T00/`。
- 完成能力：确定可构建的依赖组合、开发工具链与外部 API 用法，并保留资料缺失的事实。
- 模块/文件：`docs/decisions/dependencies.md`、`docs/evidence/T00/`；最小例先放该证据目录，不改旧项目文件。
- 步骤与约束：
  1. 复读两份必读设计并计算文件 hash；检查历史审查是否恢复。存在则逐条提炼问题→回归项；不存在明确记 missing，不编造历史结论。
  2. 安装/验证 Rust 1.97.1、固定 pnpm 10.x patch 和 Node 24.14.0；按第 2.2 节检查精确发布包、license、MSRV。锁定 TS 6.0.x patch、napi v3 patch、grammar ABI；禁止采用缓存里碰巧存在的依赖。
  3. 建最小 Rust probe：Rig 构造一个工具，模拟两轮模型结果、结构化错误、max_turns、取消；不需要真实凭据。分别记录 provider 构造/自定义 key/baseUrl 的实际方法；Schema 导出/校验一条错误；SQLite FTS/backup/WAL/JSON/FK；加载四类 grammar。
  4. 建最小 Node probe：napi v3 加载、Rust Schema 的 Standard Schema adapter、MCP 2.4.0 stdio tools/list/call。probe 使用临时包，不创建产品的平行实现。
  5. 对候选版本不能编译的情况先记录实际错误，选择同系列兼容 patch；如必须改变本文明确版本，更新依赖决策、相关任务和证据。未验证 provider 不能写为支持。
- 输入/输出/失败：官方版本资料与本机工具 → dependency matrix、可编译 probe、精确 API 记录；网络/安装/构建失败仅阻塞受影响路线，历史资料缺失为非阻塞记录。
- 验证与通过：在 probe 目录实际运行 `cargo +1.97.1 test --locked`、锁定 pnpm 的安装与 Node test；每项都保存非零测试数量；MCP 请求获得已声明 Schema 和工具响应，错误可结构化回传；不以官网支持清单代替实测。
- 保存证据：两份设计 hash、缺失历史记录、官方版本/源码 commit 链接、工具版本、probe 源码和 lock、命令/退出码及兼容矩阵。

### T01 — 新工作区、CLI 骨架和统一验证入口

- 状态：`doing`；前置依赖：T00。
- 本轮结论（2026-10-26）：Rust workspace、锁文件、`project-knowledge --help/--version`、stdout/stderr 与退出码边界已就绪；`pnpm run build:contracts` / `check:contracts` / `verify:docs` 为真实入口且可执行；`cargo fmt --all -- --check`、`cargo clippy --workspace --all-targets` 均无告警。**未完成**：新项目独立 CI workflow、`--no-default-features` Core-only 构建的独立验证记录。证据：`docs/evidence/T01/`。
- 完成能力：独立新项目可构建，CLI 帮助可运行，后续命令和检查有一致入口。
- 模块/文件：`Cargo.toml`、`Cargo.lock`、`rust-toolchain.toml`、`package.json`、`pnpm-workspace.yaml`、`pnpm-lock.yaml`、`crates/{core,agent,cli}/`、`bindings/node/`、`xtask/`、`scripts/`、`.gitignore`。
- 步骤与约束：
  1. 以第 3.1 节包名建立 workspace 和依赖方向；Node binding 为 cdylib，添加 `build.rs`；CLI 的 Agent feature 可选，禁止 Core 间接依赖 Agent。
  2. 建 `project-knowledge --help/--version`、配置路径解析、stdout/stderr/exit code 边界；未来未实现命令返回明确错误。
  3. 生成锁文件并固定工具版本；创建第 4.1 节 script 的真实调度入口，未完成子功能不返回成功。创建 `scripts/verify-plan.mjs` 校验本文件任务依赖。
  4. 将旧 workflow 仅作为风险参考，在新项目创建独立 CI；不复制旧包名或不存在的 scripts；CI 首先检查 format/clippy/Core-only 构建和依赖方向。
- 输入/输出/失败：T00 依赖矩阵 → 全新 workspace/锁文件/可运行帮助；目录已有用户文件时检查并合并，不覆盖；初始化依赖失败保留诊断，不生成假 lock。
- 验证与通过：`cargo fmt --all -- --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo test --workspace --locked`、`cargo build -p project-knowledge-cli --no-default-features --locked`；`cargo run -p project-knowledge-cli -- --help`；`pnpm run verify:docs -- --plan ../plan/NEW-PROJECT-LONG-TASK.md`；Core 的 cargo dependency tree 无 Rig。
- 保存证据：workspace/lock/toolchain 摘要、CLI help、CI 实际命令清单、依赖图与检查结果。

### T02 — 统一类型、稳定身份、Schema 和协议测试向量

- 状态：`doing`；前置依赖：T01。
- 本轮结论（2026-10-26）：统一 DTO、稳定身份纯函数、2020-12 Schema 导出、类型注册表与共享身份向量已实现；Rust 与 Node 两种独立实现对 `fixtures/protocol/identity-vectors.json` 的 18 条向量结果一致；`build:contracts` / `check:contracts` 可重复生成且一致。**未完成**：Python 侧向量执行（本机无 Python 3.12）、以 `jsonschema` 对非法 fixture 做精确 code/JSON Pointer 校验、扩展类型注册表的拒绝用例。证据：`docs/evidence/T02/`。
- 完成能力：Core/插件/CLI/SDK/MCP 使用同一套结构、身份和错误格式，脚本可独立实现兼容输出。
- 模块/文件：`crates/core/src/contract/{entity,relation,evidence,collector,query,error}.rs`、`docs/protocol.md`、`docs/schema/`、`fixtures/protocol/`、`xtask/src/`、`scripts/generate-types.mjs`。
- 步骤与约束：
  1. 实现第 3.2—3.9 节 DTO、版本、enum、默认值、可选/null 规则，建立 entity/relation kind 的端点约束和业务属性 Schema。
  2. 定义 lexicalPath、匿名 AST 路径和 normalizedSignature 的规范：签名按 AST 中语法 token 序列（去空白/注释、保留标识符和字面量）编码，字符串/数值不做求值；不支持语法返回 gap；重命名/移动造成身份变化，格式化/增行不改变有名符号身份。
  3. 实现 ID 纯函数及 Rust/Node/Python 共用向量，涵盖同名跨 scope/仓库/数据源、复合键顺序、Unicode、CRLF、匿名/重载碰撞。
  4. 从 Rust 导出 2020-12 Schema、类型注册表和 TypeScript 类型；补 `RelationRef={snapshot,relationId}`；扩展注册禁止覆盖公共字段和远程引用。
  5. 写正确/错误 JSON fixture，核对 Serde、jsonschema 和 TS 类型生成边界一致，不通过手工另写 TS interface 解决差异。
- 输入/输出/失败：本计划契约 → Schema、生成类型、身份向量和协议说明；无法表示的类型先调整统一 DTO 并记录，不让某个传输私自放宽。
- 验证与通过：`cargo test -p project-knowledge-core --test indexing contract_`；`pnpm run build:contracts`、`pnpm run check:contracts`；Node/Python 运行同一 ID 向量，所有合法例通过、非法例返回精确 code/JSON Pointer；未知字段/版本/扩展类型明确拒绝。
- 保存证据：schemaHash、生成物比较、跨语言向量输出、错误样例、字段与端点约束表。

### T03 — 多仓库配置、文件选择和来源快照

- 状态：`done`；前置依赖：T02。
- 本轮结论（2026-10-26）：多仓库配置解析、根重叠拒绝、身份增量 CAS、隐藏文件/无 Git 仓库的 `.gitignore`、中文路径、CRLF、符号链接与超限/二进制/非 UTF-8 缺口、集合摘要随增删变化、复制到不同绝对路径后身份不变均已实现并由 `cargo test -p project-knowledge-core --test indexing project_` 覆盖（6 条用例全部通过）。证据：`docs/evidence/T03/`。
- 完成能力：可接入任意目录布局的多仓库，准确识别选中文件与新增/删除/忽略变化。
- 模块/文件：`crates/core/src/project/{config,scan,snapshot}.rs`、`crates/cli/src/commands/init.rs`、`crates/core/tests/indexing.rs`。
- 步骤与约束：
  1. 实现相对 config 路径解析、repo 根去重/重叠检查、scope/service/dataSource/bindings 配置校验及带 expectedConfigHash 的 identityDelta CAS 写入函数；scope 未配置时默认每仓独立 repository scope，不猜前后端身份。
  2. 按第 3.2 节显式配置 ignore/globset 行为；跨平台统一 `/`，硬排除控制/生成文件；conventions 走显式来源入口。
  3. 扫描保存原始内容/hash/lineMap，处理零字节、二进制、超限、不合法 UTF-8、不可读、symlink、大小写冲突；每项 skip 都有 gap。
  4. 计算 sourceSet 与 sourceSnapshot，读前后变化重试有界；新文件、删文件、ignore 改变均改变集合摘要，空集合也是有效可追踪输入。
- 输入/输出/失败：project.json 与文件树 → SelectedSources、来源版本、选择器摘要和 gaps；不稳定读取返回 SOURCE_CHANGED，不部分提交 fresh。
- 验证与通过：`cargo test -p project-knowledge-core --test indexing project_`；使用临时两仓目录在有/无 Git、隐藏文件、中文路径、CRLF 下比较 manifest；新增/删除/ignore/范围调整必改变摘要，复制到不同绝对路径仍得到同一逻辑身份。
- 保存证据：配置样例、扫描清单、差异前后 hash、跳过原因、平台路径断言。

### T04 — SQLite 断言存储、批次替换和一致读

- 状态：`done`；前置依赖：T03。
- 本轮结论（2026-10-26）：`crates/core/migrations/0001_initial.sql` 已按 §3.4 落地全部逻辑表、外键、CHECK、触发器与索引；WAL / `foreign_keys=ON` / `busy_timeout=5000` / `synchronous=FULL` 已配置；`PRAGMA foreign_key_check` 与 `integrity_check` 通过；批次替换按替换范围切换当前指针而不删除他人断言；失效 generation 单向；backup API 复制一致快照；`cargo test -p project-knowledge-core --test indexing storage_` 6 条用例全部通过。证据：`docs/evidence/T04/`。
- 完成能力：来源、断言、关系、证据、依赖和版本有受约束持久化，插件互不误删，失败整组回滚。
- 模块/文件：`crates/core/migrations/0001_initial.sql`、`crates/core/src/storage/`、`crates/core/tests/indexing.rs`。
- 步骤与约束：
  1. 实现第 3.4 节逻辑表的实际 DDL，保存来源 BLOB、复合 FK、父子 assertion 完整性；在 docs 写出最终 DDL 和查询可见条件。
  2. 封装只读 open 与维护 open；只读不存在/版本不符直接错误，维护执行 migration；所有连接配置外键与超时，写库专用 owner。
  3. 实现事务 A/B/C 的基础 storage API、current_batches 过滤、同身份多断言返回、成功空批次替换；重复 runId/validationId 不重复推进 generation。
  4. 实现源版本保留与历史读取、stale 单向失效、依赖反查；坏 FK/位置/identity 不允许绕过 Core 直接入库。
  5. 用 backup API 创建 staging，证明当前库 WAL 未 checkpoint 时仍复制一致；实现两个读连接+写连接的隔离检查。
- 输入/输出/失败：已验证 DTO 和预期 generation → 提交后 Snapshot 或统一 error；busy 有界返回，任一步失败 current pointer/搜索投影全不变。
- 验证与通过：`cargo test -p project-knowledge-core --test indexing storage_`；执行 `PRAGMA foreign_key_check`、`integrity_check`；两个插件断言相同 entity 后替换其一，另一仍可见；错误批次、空批次、并发读和强制事务 rollback 断言通过。
- 保存证据：migration checksum、实际 schema、约束故障样例、事务前后 DB 摘要、backup/WAL 和一致读报告。

### T05 — 可运行的基础索引、搜索、代码地图和证据读取

- 状态：`done`；前置依赖：T04。
- 本轮结论（2026-10-26）：基础索引链路已打通：扫描 → 来源快照 → TS/TSX/JS/Python 符号与保守引用 → 实体/关系/证据 → 搜索投影 → 事务提交；中文两字、title、注释与标识符可召回；证据按 UTF-8 字节区间回读并核验磁盘；越界与 UTF-16 偏移被拒绝；未采集能力一律登记 `not_configured`；只读查询不推进 generation；来源变化被检测为 stale；`cargo test -p project-knowledge-core --test indexing base_` 与 CLI 端到端（init → index → status → search → readSource → incremental）全部通过。证据：`docs/evidence/T05/`。
- 完成能力：完成首条无 LLM 端到端链路，未知框架仍可检索真实源码并知道覆盖边界。
- 模块/文件：`crates/core/src/{syntax,search,codemap,evidence,index/build.rs}`、`crates/cli/src/commands/{index,query,status}.rs`、`crates/core/tests/indexing.rs`。
- 步骤与约束：
  1. TS/TSX/JS/Python 使用锁定 grammar 提取定义、导入和保守引用；跨文件调用只在显式绑定证据充分时 confirmed，否则 candidate，不按同名连线。
  2. 全部选中文本（包括 SQL、注释、Markdown、无 parser 语言）建搜索投影；字段源保留断言/evidence，中文单字/双字 token 和 camelCase/snake_case 拆词由 Rust 完成。
  3. 实现路径/符号/源码检索、确定性代码地图和基础预算；暂未采集的 routes/frontend/api/database/workflow/report 必须 not_configured。
  4. 实现原始 UTF-8 字节定位、CRLF/BOM/中文/emoji 行映射、证据 hash 与当前文件核验；证据过期返回 expired，显式允许才返回 storedVersion。
  5. CLI 串联 init→index --base→status→search→readSource；只读路径不建库、不迁移、不写诊断；T15 再扩充通用图查询和分页。
- 输入/输出/失败：两仓源码和配置 → 基础批次、检索与源码引用；单文件 parse 失败保留文本并 coverage partial，整个扫描不稳定则不提交 fresh。
- 验证与通过：`cargo test -p project-knowledge-core --test indexing base_`；使用开发 CLI 在临时业务目录执行初始化、基础索引和请求文件查询；断网且移除模型 key/Node/Python PATH 仍可完成基础链路；中文两字、title 文本、注释和标识符可召回；越界/UTF-16 位置拒绝。
- 保存证据：CLI JSON、来源片段/hash、搜索排序、覆盖状态、离线依赖隔离结果。

### T06 — 冻结独立真值、两种布局和数值样本

- 状态：`todo`；前置依赖：T05。
- 完成能力：插件与场景有独立于其生成结果的验收依据，能识别同名误连和统计口径错误。
- 模块/文件：`fixtures/{page-location,purchase-workflow,purchase-ledger}/`、`fixtures/truth/{manifest,scenario1,scenario2,scenario3,negative}.json`、`docs/validation/truth-review.md`。
- 步骤与约束：
  1. 按第 5 节定义源码 fixture、原始业务流程说明、DDL/ORM/SQL 与本地数据；A 为 TS 前后端分仓，B 为不同目录的 JS/TSX 前端+Python 后端；不要求整个业务应用运行，但绑定和取数源码必须完整可读。
  2. 固定 title/i18n、事件/请求/HTTP/GraphQL 注册、代理/实例绑定、两个数据源同名表、复合键与动态目标反例；调整布局/改名后业务真值不变。
  3. 真值记录 expected 与 forbidden entity/relation identity、逐条原始证据和行范围、流程条件、数值预期；从经维护者确认的原始资料或独立审阅建立，不能从待测插件输出反推 expected。
  4. 本文数值样本仅作为待复核的验收设计。以维护者签认或独立审计样本冻结 truthHash；Agent 合成源码/样例只作为 smoke，未完成独立核对时不得报告业务准确率。
  5. 限制 Agent 候选工具只能读真值和报告，不可写 fixtures/truth 或降低阈值；真值变更单独审阅，保存原因、前后 hash。
- 输入/输出/失败：原始源码/流程/SQL/人工口径 → 冻结真值包、review 记录与校验脚本；缺少独立真值时标 blocked，其他 Core/宿主工作仍可按合成 smoke 推进，但不能标该任务 done。
- 验证与通过：`cargo test -p project-knowledge-core --test business_scenarios truth_`；验证每个 expected/forbidden 有原始出处、表格数值可由单独计算逐项复核；真值文件不在 Agent 写范围；至少一个业务资料审阅记录与独立作者/来源信息齐全。
- 保存证据：原始资料 hash、truthHash、审阅来源/人/日期、数值核对表、禁止修改检查、合成与独立样本的分离清单。

### T07 — manifest、运行时、锁定依赖与有界子进程

- 状态：`doing`；前置依赖：T05。
- 本轮结论（2026-10-26）：manifest 校验（协议版本、ID/版本非空、`executable` 运行时必须引用项目已配置 runtime、入口与制品路径不得越界/绝对/符号链接）、阶段 DAG 校验（唯一 ID、`pluginId:stageId` 依赖格式、自依赖与环、extract 不得依赖 link）、制品哈希（排序路径 + 逐文件内容哈希 + 依赖锁摘要）已实现；子进程宿主实现了 stdin 单对象 / stdout 单对象、输出与 stderr 有界、并发 2、超时、取消、env 白名单、任务 cwd，以及 Windows Job Object / Unix 进程组的进程树终止；Node 与 Python 模板及超时/坏 JSON/非零退出/输出洪泛故障脚本已就位；`cargo test -p project-knowledge-core --test collectors` 14 条用例全部通过；CLI `collect --phase extract` 已能发现候选、运行阶段并校验输出（证据 `docs/evidence/T07/e2e/`）。**未完成**：Python 运行时在本机不可用，Python 模板只能静态校验；`npm ci` 与 `pip install --require-hashes` 的真实依赖准备未联网执行（准备代码已实现，但本轮没有可用的锁文件样本）；Windows Job Object 的后代进程清理尚无独立断言（超时用例只验证了返回时限与错误码）。
- 完成能力：Core 能可复现运行 Node/Python 插件，正常、失败、超时、取消和输出溢出行为明确。
- 模块/文件：`crates/core/src/collect/{manifest,runner}.rs`、`collectors/templates/{node,python}/`、`crates/core/tests/collectors.rs`、`crates/cli/src/commands/collect.rs`。
- 步骤与约束：
  1. 实现 manifest Schema、运行时版本检查、受限 executable 映射、入口/artifacts 路径校验、stage DAG 和 replacement 范围验证。
  2. 实现 Node npm ci 与 Python hash-lock venv 准备、平台/runtime/cache 摘要、无依赖模式；依赖失败不执行插件，不从业务 node_modules 补包。
  3. 实现单 JSON stdin/stdout、流式输出计数、stderr 有界保存、超时、取消、并发 2；JSON 全量接收后才交验证器。
  4. 实现 env 白名单、任务 cwd、Unix 进程组和 Windows Job Object；已有中断进程清理需要可验证 ownership，禁止杀名称相同的其他用户进程。
  5. 提供 Node/Python 最小可运行模板与超时、坏 JSON、非零退出、孙进程、输出洪泛故障脚本；明确“不等于 OS 沙箱”的产品边界。
- 输入/输出/失败：manifest + 快照 → 原始 output 与 RunDiagnostics；缺 runtime/依赖/超限/退出均结构化失败，无活动事实变化。
- 验证与通过：`cargo test -p project-knowledge-core --test collectors runner_`；Node/Python 两模板同义输出；取消/超时后父子进程全部退出、无残留句柄；中文空格路径可运行，模型 key 不出现在 env；重跑无需网络准备。
- 保存证据：runtime/lock/cache digest、执行 argv（脱敏）、所有错误码、超限字节数、进程树清理报告和平台结果。

### T08 — 事实校验、临时 extract/link 和范围替换

- 状态：`doing`；前置依赖：T07、T04。
- 本轮结论（2026-10-26）：采集输出校验已实现（协议版本、runId/stageId/inputSnapshotId 回显、实体 ID 重算、AI 判断必须是 inferred、每条断言至少一条原始证据、证据必须指向输入来源且内容哈希一致、字节区间非空且不越界、行号与字节区间一致、生成知识不得作为原始证据、关系端点必须来自本批或已导出快照且端点类型组合合法、依赖与缺口声明自洽、覆盖能力必须在 manifest 的 capabilities 内）；结果事务 B 已实现（按替换范围全量替换断言、证据引用、依赖、coverage、搜索投影与 `current_batches`，旧批次单向失效并记录失效 generation，成功空输出同样替换，其他生产者的断言不受影响，失败整组回滚）；阶段输入导出已实现（来源 + 内容 + 选择集合摘要 + 已导出实体断言）。**未完成**：staging 临时库中的 extract → 重新导出候选 → link → 验收用例的完整链路尚未实现；CLI `collect` 目前只运行并校验，不提交事实（提交路径尚未接线）；Core 自动依赖（config/artifact/runtime/sourceSet/upstream/candidateSet）尚未在导入时自动补齐；验收用例（acceptance case）比较尚未实现。
- 完成能力：外部插件输出必须经过结构、来源、语义约束和批次验证，link 可安全引用其他阶段实体。
- 模块/文件：`crates/core/src/collect/{validate,import}.rs`、`crates/core/src/index/dependencies.rs`、`crates/core/tests/{collectors,indexing}.rs`。
- 步骤与约束：
  1. 校验协议、ID 重算、kind/端点、必需业务属性、每条断言 evidence、原始 source/hash/字节/行，拒绝越界/不存在/生成知识作为原始证据。
  2. 校验 relation 端点只能来自本批或已导出快照；消费已有实体必须绑定来源断言与 selector 摘要；同名唯一性判定必须有 complete candidateSet，包括空集合。
  3. Core 自动加入 config/artifact/runtime/sourceSet/upstream/candidateSet 依赖，插件可细化来源依赖但不能删自动依赖；检查声明 coverage 与被跳过文件/gap 一致。
  4. staging 中移除目标 replacementKey 旧批次，导入 extract、重新导出候选、执行 link、跑 case；链接错误不允许正式导入 extract 作为“部分成功”。
  5. 实现 validate-only 不改正式 generation；正式写入 B 只替换生产者阶段全项目范围；成功空输出和删除实体使下游引用失效，不能留下可见悬空关系。
- 输入/输出/失败：原始 plugin output、输入快照、case → ValidationReport/可提交 StagedBatch；任何 fatal issue 整组失败，issues 保留精确 instancePath/repairHint；partial coverage 可入库但不伪报 complete。
- 验证与通过：`cargo test -p project-knowledge-core --test collectors validation_`、`cargo test -p project-knowledge-core --test indexing import_`；测试同批/跨插件引用、位置错误、证据伪造、空输出、其他 owner 保留、candidateSet 截断、扩展类型；坏输入正式库摘要不变。
- 保存证据：输入/output/schema hashes、逐条校验报告、staging 与正式批次差异、validate-only 前后 generation。

### T09 — 候选登记、不可变发布、原子激活和中断恢复

- 状态：`todo`；前置依赖：T08。
- 完成能力：验证后的候选可自动成为活动插件，切换失败不会暴露混合版本，外部 Agent 使用同一入口。
- 模块/文件：`crates/core/src/collect/registry.rs`、`crates/core/src/storage/{plugins,runs}.rs`、`crates/cli/src/commands/plugin.rs`、`crates/core/tests/lifecycle.rs`。
- 步骤与约束：
  1. 实现 submit/validate/activate/rollback façade 与 CLI；扫描显式 collectors 和目录时按 ID/version/hash 去重；候选发现必须经过验证，autoLoad/autoActivate 分别测试。
  2. 实现候选状态、不可变 hash 目录发布、stagingResultHash 和 validation 绑定；发布过程磁盘满/跨卷/rename失败均保留旧活动状态。
  3. 实现单项目 OS 锁、来源/集合/活动映射二次核验与 C 事务；纳入受影响 active link 闭包，原子写版本、事实、依赖和搜索投影。
  4. 设置可控故障点：发布前、发布后 C 前、C 内任一步、C 提交后答复前；维护重启识别孤立制品、interrupted run 和已完成 activation。
  5. 回退必须按当前输入重新校验运行；制品篡改/丢失令批次 stale，不修补旧 hash；并发候选冲突返回可复核 diagnostics。
- 输入/输出/失败：validationId/expectedGeneration → activationId/新 Snapshot；CAS、制品或依赖变化不切换；新事实未提交则旧事实按真实 freshness 展示，不能强置 fresh。
- 验证与通过：`cargo test -p project-knowledge-core --test lifecycle activation_`；两个并发候选最多一个成功；extract+link 不存在跨版本可见窗口；每个故障点重启后数据库完整、活动状态唯一；查询不能触发恢复和目录发现。
- 保存证据：状态转换、制品清单/hash、并发时序与 CAS、故障注入点和恢复后的数据库快照、外部提交样例。

### T10 — 依赖闭包、增量、持久失效与失败回滚

- 状态：`todo`；前置依赖：T09。
- 完成能力：无 LLM 日常更新可处理修改/删除/新增、候选集合变化和失败；过期事实不会复活。
- 模块/文件：`crates/core/src/index/{build,dependencies,incremental}.rs`、`crates/core/src/project/snapshot.rs`、`crates/cli/src/commands/{index,collect,status}.rs`、`crates/core/tests/lifecycle.rs`。
- 步骤与约束：
  1. 维护入口重扫 selection、ignore/config、source/hash、runtime/artifact/lock 与 selectors；计算依赖反向闭包并先提交失效 A。
  2. 按稳定 DAG 顺序运行受影响活动 extract/link 全范围；未受影响批次不重跑；删除和成功零输出撤下旧断言，新文件进入已登记选择集合。
  3. 处理候选集合从 0→1、1→2、2→1、目标改名/移动、未被原关系引用的新候选加入、候选属性变化；摘要不能只含最终已选 toId。
  4. 插件失败/取消/中断保留 A，批次 old/stale；回退旧制品、源码内容还原或重启不得清空 invalidatedGeneration。重新成功运行才产生 fresh 新 batch。
  5. `--base` 只提交基础并失效业务；显式分阶段 collect 维护下游状态；增量和 collect 对模型客户端调用计数必须始终为 0。
- 输入/输出/失败：当前扫描和依赖图 → 更新计划、受影响运行、新 Snapshot 和 freshness；不稳定来源/运行失败返回 partial 或错误，并附哪个阶段保留 stale。
- 验证与通过：`cargo test -p project-knowledge-core --test lifecycle incremental_`；同一变更后增量可见结果与干净全量重建（忽略 runId/generation）相同；覆盖所有 0/1/2 候选变化、删除/新增、篡改制品、失败后回退和宿主杀进程；stale 单向不变量始终成立。
- 保存证据：变更前后依赖闭包、运行次数、全量/增量 normalized diff、无模型计数、失败及恢复的 freshness 时序。

### T11 — 路由、前端、注释、模块和自定义入口插件

- 状态：`todo`；前置依赖：T10、T06。
- 完成能力：前端项目专属脚本能提取真实页面入口、标题、操作、事件、请求和业务词，并能在不同布局复用协议。
- 模块/文件：`collectors/examples/frontend-backend-a/`、`collectors/examples/frontend-backend-b/`、`collectors/templates/node/`、`fixtures/page-location/`、`crates/core/tests/business_scenarios.rs`。
- 步骤与约束：
  1. 使用锁定 TypeScript AST，按调查规则解析 route registration、hash/history、alias、懒加载、title/i18n 键及实际文案；输入 `/#/Other/purchaseLedger` 的规范化在 Core page 解析边界完成。
  2. 提取组件/方法、事件→handler、handler→wrapper→HTTP/GraphQL request；保留 baseURL、proxy、操作名/变量与动态目标缺口，禁止用函数名猜后端服务。TypeScript AST 的 UTF-16 offset 必须按原始文本映射到 UTF-8 字节，不能复用行列值作为字节位置。
  3. 注释/docstring 绑定实际符号，业务别名和模块有原始 evidence；自定义定时任务/消息/命令入口按当前样例真实注册解析，不把所有同名方法当入口。
  4. 至少两种目录/命名布局独立 manifest；项目约定可变化，但输出 kind/字段不变化。生成器不内置 NestJS/React/Vue/ORM 默认路径。
  5. 为支持模式写 manifest、锁依赖、positive/negative/candidate/gap case；不支持模板/框架构造明确 coverage partial。
- 输入/输出/失败：来源快照、基础符号和调查材料 → routes/frontend/comments/modules/entrypoints extract；语法未知保留 gap，不用正则硬连完整业务链。
- 验证与通过：`pnpm run test:plugins -- --capabilities routes,frontend,comments,modules,entrypoints`；`cargo test -p project-knowledge-core --test business_scenarios frontend_`；独立真值 expected 全命中、forbidden 零命中、动态目标有 gap；移动目录与业务改名后仍能按新身份检出。
- 保存证据：两布局制品/锁/hash、实际 AST 位置转换、case 报告、title/i18n 和事件请求的原始引用。

### T12 — 后端 HTTP/GraphQL、业务方法和数据库采集插件

- 状态：`todo`；前置依赖：T11。
- 完成能力：项目 API/数据库结构和已有 SQL/ORM 查询可采集，数据源、实例和复合键不混淆。
- 模块/文件：`collectors/examples/*/{api,database}/`、`fixtures/{page-location,purchase-ledger}/`、`crates/core/tests/business_scenarios.rs`。
- 步骤与约束：
  1. 按 A 的 TS 注册表/handler 和 B 的 Python AST 实际约定解析 HTTP method/path、GraphQL type/field/operation→resolver、业务方法调用；服务 ID 来自配置及绑定证据。
  2. 解析 DDL/schema 导出与已调查 ORM 模型/实例绑定，输出 dataSource→table→column、有序主外键和 declared cardinality；不同 dataSource 同名表隔离。
  3. 已有 SQL 使用 SQLGlot 显式方言解析 AST；ORM 仅实现 fixture 中被证实的映射模式，提取 queryImplementation、字段表达式、filter/group/join 源码，不执行目标业务代码。嵌入字符串/转义/模板到原始文件须有位置映射；无法精确到 token 时引用可证明的完整语句/字符串范围，不伪造列级 offset，动态拼接仍返回 gap。
  4. 保存复合关联键有序 pairs、left/right join、条件、nullable 与主外键声明来源；只见列名相同不能推导 FK 或 1:1。
  5. 动态 SQL/请求参数、动态实例和未支持 ORM 返回 candidate/gap；解析失败不补造表列、条件和关系基数。
- 输入/输出/失败：后端/DDL/SQL/ORM 来源、显式 service/dataSource → api/database extract 和 coverage；缺 datasource 或未知 dialect 拒绝 confirmed 跨源关系，其他已证实事实可 partial 保存。
- 验证与通过：`pnpm run test:plugins -- --capabilities api,database`；`cargo test -p project-knowledge-core --test business_scenarios backend_`；HTTP/GraphQL 两条链及业务方法与独立真值一致；不同源同名表、错误实例、复合键遗漏任一列均不得误连。
- 保存证据：两布局输出、SQL/ORM AST 与原文映射、dataSource binding、负例和 unsupported gap 报告。

### T13 — 跨插件、跨仓库与候选集合关联

- 状态：`todo`；前置依赖：T12。
- 完成能力：前后端、数据库和业务对象按真实绑定关联，歧义保持候选且可增量重算。
- 模块/文件：`collectors/examples/*/link/`、`crates/core/src/collect/validate.rs`、`crates/core/src/index/dependencies.rs`、`fixtures/truth/negative.json`。
- 步骤与约束：
  1. link 从 extract 后 staging 导出实体快照；引用既有 entityId，不重复制造“相似实体”。以 service+method+注册路径、GraphQL type/field、dataSource+实例+表映射确认目标。
  2. wrapper/baseURL/proxy 的组合只在原始配置可证明时绑定；跨仓调用必须有项目 bindings 或契约/注册证据；显式人工声明标 declared/human。
  3. 同名组件/方法/handler、同路由不同 app、同表不同数据源分别形成候选；唯一一个名字仍不能自动 confirmed，缺少绑定返回 requiredEvidence。
  4. 为每种关联记录 selector 定义和完整候选集摘要；新增同名实体时旧“唯一匹配”失效，关联可能从 confirmed 变 candidate，不复用历史 toId。
  5. 验证每条跨层关系有自身调用/绑定 evidence；禁止仅借实体存在或一条 AI 自述确认关系。
- 输入/输出/失败：staging entity/assertion 集合与真实绑定来源 → link 关系/candidate/gaps；候选导出不全返回 CANDIDATE_SET_INCOMPLETE，不能完整激活。
- 验证与通过：`cargo test -p project-knowledge-core --test business_scenarios linking_`；跨仓正确链路全部命中，负例零 confirmed；新增候选/修改代理/切 datasource 后全量与增量结果等价；每条 confirmed 边有独立证据。
- 保存证据：entity selectors/digests、逐边 provenance、歧义转换前后结果、禁止关系清单。

### T14 — 业务流程与台账口径的结构化采集

- 状态：`todo`；前置依赖：T13。
- 完成能力：采集业务操作/状态/前后单据与取数语义，不以代码调用或路径距离替代业务依据。
- 模块/文件：`collectors/examples/*/{workflow,report}/`、`crates/core/src/contract/{entity,relation}.rs`、`fixtures/{purchase-workflow,purchase-ledger}/`。
- 步骤与约束：
  1. workflowState/operation/documentType 与 transitions_to/precedes/relates_document 保存 trigger、guard、from/to state、单据 keyPairs、前后页面及实际流程文档/状态写入证据。
  2. 将源码调用顺序单独保留 calls；缺少业务顺序依据不生成 precedes。代码与原始流程资料冲突时返回两个断言及 CONFLICTING_EVIDENCE，不自行合并。
  3. queryImplementation 的 reportField 输出来源表达式、joinKeys、cardinality、grain、aggregates、validStates、时间字段、NULL/去重/单位处理；未知字段用 gap，不给默认口径。
  4. 识别一对多关联的预聚合和重复累加风险，记录现有 SQL/ORM 的实际统计位置；只给证据支持的基数，未声明/未验证时 unknown。
  5. 两个候选实现中即使某条链更短，也按目标业务范围、单据键、状态和粒度筛选；无法排除错误实现时同时返回候选并指出待确认条件。
- 输入/输出/失败：原始流程资料、源码状态逻辑、已有 SQL/ORM/DDL → workflow/report 扩展断言及统一关系；未知流程/口径为 partial，可观察代码事实照常保留。
- 验证与通过：`pnpm run test:plugins -- --capabilities workflow,report`；`cargo test -p project-knowledge-core --test business_scenarios semantics_`；调用顺序反例不产生流程边，错误粒度/丢 tenant 键/错误有效状态均被标识；缺任一必需口径产生对应 gap。
- 保存证据：逐状态 guard/单据映射、SQL 表达式与 reportField 对照、冲突和缺口报告、独立口径审阅。

### T15 — 通用查询、双向有界遍历、预算和分页

- 状态：`todo`；前置依赖：T13。
- 完成能力：所有客户端可用同一 DTO 查询断言、上下游和链路，明确完整性、分页和过期语义。
- 模块/文件：`crates/core/src/{query/mod.rs,search,graph,codemap,evidence}`、`crates/core/src/contract/query.rs`、`crates/cli/src/commands/query.rs`。
- 步骤与约束：
  1. 完成第 3.7 节公共 response、filters、basis/candidate/freshness 默认值、schema capabilities，必要范围的只读 freshness 核验不落库。
  2. 精确 route/path/identity 与多字段 FTS 合并去重、稳定排序；中文业务词、标题、注释、模块、符号与源码均有命中理由。
  3. 实现 in/out/both 邻居与有界 BFS、visited 去重、每跳方向/条件/证据，rootFilter 与 traversalFilter 分离；不按最短路径给业务正确性分数。
  4. 搜索/邻接实现 keyset continuation，generation/requestHash 不同拒绝；路径预算返回 frontier/nextQueries，budgetUsed 和预留 envelope 字符限制准确。
  5. 处理空完整结果与未覆盖空结果、stale 排除与 allowStale、跨 generation evidence；查询前后持久业务表/generation/活动制品/候选目录应无变化，以 SQL trace 和逻辑摘要验证；不把 SQLite 自身 WAL/SHM 协调文件的变化误认为执行维护。
- 输入/输出/失败：统一 QueryRequest → QueryResponse 或统一 error；预算返回 partial；超大非法请求、无库、过期 token、取消分别分类，不隐式刷新。
- 验证与通过：`cargo test -p project-knowledge-core --test indexing query_`；双向回环/高出度/跨层 rootFilter、分页无重无漏、预算上限、Unicode 字符预算、generation 变更失效均通过；EXPLAIN QUERY PLAN 确认邻接/精确路由使用预期索引，不要求编写 SQL 优化器。
- 保存证据：请求/响应 golden、预算与分页结果、只读数据库 hash/SQL trace、索引计划、无维护调用计数。

### T16 — 页面、流程、台账三个场景的 Core/CLI 闭环

- 状态：`todo`；前置依赖：T14、T15。
- 完成能力：三类业务输入可返回有证据的跨层事实和缺口，CLI 已具备全部业务查询能力。
- 模块/文件：`crates/core/src/query/{feature,page,workflow,report}.rs`、`crates/core/tests/business_scenarios.rs`、`scripts/verify-scenarios.mjs`。
- 步骤与约束：
  1. locateFeature 组合搜索、入口和有界关系；page 处理完整 URL/hash/history/alias，保留原始路径大小写和动态匹配歧义，不能只 substring 匹配。
  2. 页面场景输出 title/i18n、组件操作→事件→请求包装→HTTP/GraphQL→业务方法→表列，各缺失跳单独 gap。
  3. workflow 只组合带业务语义的关系，输出前后页面、状态、流转条件、关联单据；report 优先定位现有 SQL/ORM，对字段来源/关联键/基数/粒度/聚合/状态逐项回答，未知明确空值+gap，不编造 SQL。
  4. 按第 5 节冻结真值核对关系和数值；本地 fixture SQL 由测试 oracle 单独执行，Core 不执行用户业务 SQL。错误 join/最短误导路径必须作为反例。
  5. 创建 verify:scenarios 的 cli 入口和逐项报告，响应中每个结论都能回读匹配 source hash。
- 输入/输出/失败：路由、业务功能或台账问题 → 场景 response；歧义、缺流程资料、动态接口、口径不全返回 partial，不用猜测填满链路。
- 验证与通过：`cargo test -p project-knowledge-core --test business_scenarios`；`pnpm run verify:scenarios -- --scenario all --transport cli`；第 5 节全部 cli 项通过，独立数值与现有正确实现一致，错误实现有明显反证/缺口。
- 保存证据：场景矩阵逐项结果、来源回读、字段 lineage 对照、数值差异表、错误链路反例。

### T17 — 主题、版本绑定、知识失效和 Skill 输出

- 状态：`todo`；前置依赖：T16。
- 完成能力：可无模型导出主题上下文、导入外部产物并生成可核验的业务 Skill，旧文档能被判过期。
- 模块/文件：`crates/core/src/knowledge/`、`crates/core/templates/`、`crates/cli/src/commands/generate.rs`、`skills/{project-onboarding,project-query}/`。
- 步骤与约束：
  1. 实现 TopicArtifact/context 契约和预算选择，按主题选事实、原文、代码地图与 coverage；记录 inputDigest、原始 source 和候选集合依赖。
  2. import-topic 校验逐 claim evidence/basis、indexId/generation/inputDigest，拒绝循环 topic、generatedKnowledge 作为源码和无原始依据的派生约定；import-conventions 复用 T03 的 identityDelta CAS、原始证据校验与来源登记，返回变更后的快照并失效受影响索引。
  3. strict Handlebars 渲染第 3.9 节全部文件和 index-manifest；SKILL.md 写真实工具/bin 导航及 freshness 检查，不生成独立查询算法。
  4. 相关源/规则/插件/Prompt 变化标 topic/export stale；无关改动允许依赖复核但保留原生成版本；单纯重渲染不得刷新 LLM 解释。
  5. 原子输出完整目录/清单，导出中断保留上个完整版本并准确报告其状态；自动生成输出硬排除在采集范围。
- 输入/输出/失败：新鲜主题上下文或外部 TopicArtifact → 保存的 topic version、Skill 和绑定 manifest；引用过期或循环拒绝，partial 产物必须显式标缺口。
- 验证与通过：`cargo test -p project-knowledge-core --test indexing knowledge_`；开发 CLI 执行 context→导入固定外部主题→skill；校验所有 Markdown 引用/文件 hash；修改相关 SQL 后文档 stale，重渲染仍 stale，重新导入当前主题才 fresh；生成内容扫描回流测试为零。
- 保存证据：context/topic/schema hashes、Skill 输出清单、引用校验、过期/重生成时序、无循环证据报告。

### T18 — Rig 模型配置、Prompt 组合和预算工具包装

- 状态：`todo`；前置依赖：T17、T00。
- 完成能力：Rust Agent 可调用 Core 受控工具，接收结构化错误，模型/凭据/时间/轮数配置可验证。
- 模块/文件：`crates/agent/src/{config,prompts,tools}.rs`、`prompts/{shared,investigate,collector,link,repair,knowledge}.md`、`crates/cli/src/commands/agent.rs`、`crates/agent/tests/agent_lifecycle.rs`。
- 步骤与约束：
  1. 按 T00 确认的真实 Rig 0.45.0 API 建 Agent；实现 provider allowlist、apiKeyEnv 注入、baseUrl 校验、model 必填和工具调用 preflight；禁用 provider 不进入 capabilities。
  2. 实现 Prompt 顺序与项目补充，动态加载当前 schemaHash、工具 allowlist 和 scope；默认内容不绑定框架/ORM/业务名；源码文本只作为引用数据。
  3. Core 工具以最小参数 Schema 注册；validate 返回完整有界 issues 给 Rig 工具反馈，不能被框架默认泛化成“失败”。
  4. Hooks/包装器统一累计 24 次模型请求、80 次工具调用、2 次修复和 600 秒总时间；provider 重试计数；取消可到达 Core runner。
  5. Agent configure 不写密钥；预先 abort、超时、缺 key、401、429、无工具模型、非法 JSON 和 endpoint 不兼容均有结构化输出。
- 输入/输出/失败：agent 配置、goal/focus、项目约定与 Core 工具 → 可执行受控 Rig Agent；缺配置只影响 agent 命令，日志严格脱敏；不兼容端点直接失败，不偷偷换 provider/model。
- 验证与通过：`cargo test -p project-knowledge-agent --test agent_lifecycle config_`、同 suite 的 `tools_`；模拟 provider 精确验证轮数、错误 JSON Pointer 和取消。至少一个真实配置模型完成一次查询工具 smoke，记录 provider/model/日期但不记录 key；未验证端点保持不支持。
- 保存证据：脱敏配置、Prompt hash、工具列表/schema、mock 请求计数、真实 smoke 的请求 ID/耗时和响应摘要。

### T19 — 调查、生成、有限修复、激活和知识的 Agent 闭环

- 状态：`todo`；前置依赖：T18、T09。
- 完成能力：一个 Rig Agent 能完成增强索引与 Skill，失败能有限停止；外部 Agent 制品可走同样 Core 路线。
- 模块/文件：`crates/agent/src/{runner,lib}.rs`、`crates/agent/tests/agent_lifecycle.rs`、`skills/project-onboarding/`、`docs/agent-usage.md`。
- 步骤与约束：
  1. 实现第 3.8 节状态机和产物检查，调查先读取基础索引与原文，再 import-conventions；未知身份不自动写为确定 binding。
  2. 生成 manifest/脚本/锁/case 到唯一候选 run；validate 返回结构化 issue 后最多两次 repair，所有修订重算 hash 与验证；工具不能修改真值、业务源码或公共协议。
  3. 默认验证通过自动发布/登记/激活；autoActivate=false 返回 pending_activation；with-skill 只使用活动索引做主题生成/导入/Skill，失败整体 partial。
  4. 保存进度与累计预算，实现 resume 的来源复核和 activation 幂等；模型自行称完成、只生成文件、只过 schema 或未完成业务 case 均不能报 done。
  5. 用外部手写/外部 Agent 候选调用 submit→validate→activate，证明插件协议完全不含 Rig 类型；日常 index 运行不加载 Agent crate/model 客户端。
- 输入/输出/失败：goal/focus + 项目 → conventions、可运行插件、validation/activation、主题/Skill 和最终报告；两修复仍失败或预算耗尽保存 candidate/issues 并停止，旧批次 freshness 真实保留。
- 验证与通过：`cargo test -p project-knowledge-agent --test agent_lifecycle`；故意错误插件在预算内被修复或准确停止；一次真实模型在独立 fixture 完成全流程（按 T06 真值评估）；随后去掉 key 重跑增量成功、模型调用 0；autoActivate=false、冲突、中断、知识失败均有正确状态。
- 保存证据：每阶段产物/hash、issues→修订→新验证链、总预算、激活快照、真实业务验收与外部候选兼容性、无模型增量计数。

### T20 — Node binding、TypeScript SDK 与取消/关闭

- 状态：`todo`；前置依赖：T15、T17、T02。
- 完成能力：SDK 对同一个 SQLite/配置提供类型化查询及显式维护，不重复实现查询与存储。
- 模块/文件：`bindings/node/{Cargo.toml,build.rs,src/lib.rs,package.json}`、`packages/sdk/src/{index,client}.ts`、`packages/sdk/src/generated/`、`packages/sdk/test/`。
- 步骤与约束：
  1. binding 提供只读 open/execute/getCapabilities/close 和显式维护 open；统一 JSON DTO 传输以减少 N-API 类型差异，业务类型由生成 TS 保证；非法输入仍由 Core 校验。
  2. 异步执行在后台线程/任务，Connection 在 owner 线程创建/使用；SDK 不阻塞 Node event loop，不安装第二套 SQLite driver。
  3. SDK 提供第 3.7 节全部方法、统一 error 类与 snapshot/ref 透传；维护客户端只能调用 Core 已有入口，不需要或打包 Rig Agent。
  4. 请求 ID 绑定 Core 取消令牌；AbortSignal 已触发需立即拒绝，运行中取消中断查询或停止插件树；close 幂等、拒绝新任务并取消/等待已有任务，在超时内释放资源。
  5. capabilities/只读状态不做 migration/发现/采集；native 找不到匹配平台包时明确安装错误，不下载源码编译作为默认 fallback。
- 输入/输出/失败：ProjectKnowledge.open({configPath}) + 生成类型请求 → Promise<QueryResponse>；错误保留 code/issues，native panic 转 INTERNAL_ERROR 不崩溃宿主；关闭后调用明确 CLOSED。
- 验证与通过：`pnpm run build:native`、`build:sdk`、`typecheck`、`test:sdk`、`check:contracts`；同请求 normalized CLI/SDK 完全一致；大量查询时 event loop 仍可响应；pre-abort/运行中取消/重复 close/查询后文件锁释放通过，读操作无写 SQL。
- 保存证据：生成类型差异、跨端 golden、event loop 测量、取消/close/句柄测试、依赖树无 JS SQLite/Rig。

### T21 — stdio MCP 查询资源与显式维护模式

- 状态：`todo`；前置依赖：T20、T09。
- 完成能力：LLM 通过 MCP 直接查询与维护 Core，无需生成临时 TS 脚本，普通查询连接不具备生成/激活能力。
- 模块/文件：`packages/mcp/src/{index,tools,resources,schema-adapter}.ts`、`packages/mcp/test/`、`packages/mcp/package.json`。
- 步骤与约束：
  1. 使用锁定 MCP 2.4.0 `McpServer`、`StdioServerTransport`；bin `project-knowledge-mcp`；stdout 仅协议消息，进程日志 stderr。
  2. 实现 Standard Schema wrapper：`~standard.jsonSchema.input/output` 返回 bundled Core schema，`~standard.validate` 调 native/Core validator；支持的 draft 转换在 T00/T02 已验证，不手写平行 Zod 模型。
  3. query 模式注册 index_status/search/locate_feature/trace_links/get_code_map/read_source/locate_page/explain_workflow/explain_report，资源含 schema、索引说明、事实/证据和 Skill manifest；引用携带 indexId/generation，资源读取同样核 freshness。
  4. maintenance 模式显式加 export_snapshot/write_candidate/import_conventions/prepare_plugin/validate_plugin/run_plugin_cases/activate_plugin/update_index/export_topic/import_topic/generate_skill 等维护工具；不会给未启动的 Rust Agent 增设伪 agent 工具。
  5. 每个工具回传 structuredContent 的统一 envelope；Core 错误返回 `isError=true` 与 error，非法工具参数用协议参数错误；正常 partial 仍非工具执行错误，annotations 明确只读/破坏性特征。
  6. MCP 取消映射 SDK requestId/token，客户端断开关闭 native；只有 mode=maintenance 可写，prompt 注入和工具参数不能切 mode。
- 输入/输出/失败：MCP initialize/list/tools/call/read resources → 与 SDK 同源 response；无库不建库、unsupported protocol 按 SDK 协商/拒绝，断开不留下进程/句柄。
- 验证与通过：`pnpm run build:mcp`、`pnpm run typecheck`、`pnpm run test:mcp`；官方 client 实际启动 stdio server，核验协商版本、tools/list Schema、全部工具正常/partial/error、资源与取消；query 模式调用写工具失败且数据库无变化。
- 保存证据：stdio 协议 transcript（脱敏）、工具/Schema hash、一致性断言、模式隔离、取消与退出结果。

### T22 — 独立业务验收、跨端一致和故障回归

- 状态：`todo`；前置依赖：T19、T21、T16。
- 完成能力：所有需求有可执行验收，能评估准确性而非仅输出格式；失败和增量跨端行为一致。
- 模块/文件：`scripts/{verify-scenarios,verify-plan}.mjs`、`crates/core/tests/{business_scenarios,lifecycle}.rs`、`packages/{sdk,mcp}/test/`、`docs/validation/acceptance.md`。
- 步骤与约束：
  1. 同一 truthHash 驱动 CLI/SDK/MCP，比较去除 requestId/时间后的实体/关系/证据/缺口/complete 结果；不能复制算法实现作为“oracle”。
  2. 执行第 5 节全部矩阵、受控插件故障/中断、过期历史读取、候选集合变化、并发修改与生成材料回流负例。
  3. 统计预定 expected/forbidden 的分母、每类 precision/recall、关键链路缺失和数值差；所有 supported 必需项 100% 通过、forbidden confirmed=0；不以全图“约等于准确”代替断言。
  4. 校验本文每个需求映射至少一个任务/验收，所有依赖存在且无环；检查 CLI/Schema/SDK/MCP/Agent 术语字段和目标命令列表与实际脚本一致。
  5. 在至少一个真实多仓项目做独立抽样审阅，保留来源授权范围内的脱敏证据；无法审阅则仅报告 fixture 验收通过，真实项目准确性仍待验证，最终范围不能写已验证。
- 输入/输出/失败：冻结独立真值、构建产品、所有故障脚本 → 逐项 acceptance 报告与需求追踪；任何关键误连/错误数值/失效复活阻塞发布，不降低阈值。
- 验证与通过：`cargo test --workspace --locked`；`pnpm run check:contracts`、`pnpm run test:plugins`、`pnpm run test:sdk`、`pnpm run test:mcp`、`pnpm run verify:scenarios -- --scenario all --transport all`、`pnpm run verify:docs -- --plan ../plan/NEW-PROJECT-LONG-TASK.md`；全部需实际执行并记录，真实审阅与合成 smoke 分开统计。
- 保存证据：完整命令/退出码、各项分母与失败明细、数值 oracle/对照、跨端 normalized diff、需求映射检查和独立审阅记录。

### T23 — 分层安装、多平台制品和发布候选

- 状态：`todo`；前置依赖：T22。
- 完成能力：CLI/Agent/native/SDK/MCP 可在声明支持的平台安装运行，发布候选不依赖本机缓存或仓库源码路径。
- 模块/文件：`xtask/`、`bindings/node/npm/`、`scripts/{package,verify-packages}.mjs`、`.github/workflows/{ci,release}.yml`、`docs/{installation,releasing}.md`、各 Cargo/package manifest。
- 步骤与约束：
  1. 构建第 6 节四平台 CLI（默认含 Agent）、Core-only CLI 和 native 包；Prompt/模板/Schema 通过编译嵌入或随包资源定位，禁止依赖构建机绝对路径。
  2. native root loader+按平台 optional package，SDK→native、MCP→SDK 的版本及 npm os/cpu/libc 条件一致；绑定包打入 required build.rs/schema 资源，Rust CLI 不要求 Node。
  3. CI 从干净 checkout/lock 构建，收集 artifacts 后组装并核 hash/版本/目标平台；不复制当前旧 workflow 的 package 名和 scripts。Node 24 运行期与 Rust/C/C++/Python 构建期分别测试。
  4. 每个平台在干净 consumer 安装 tarball 和 CLI，移除 Rust/Node/Python 等非该能力所需工具后测试，四类插件运行时依赖由各自 manifest 检查；无匹配 native 包给明确 unsupported 错误。
  5. 写 release manifest（CLI/core/schema/native/sdk/mcp/protocol/SQLite/runtime 版本和 SHA-256）、license/SBOM、安装/卸载/升级步骤；核验 npm scope、crate/bin 冲突和发布权限，产生 dry-run/pack 制品。
  6. 本计划默认交付可审查发布候选；实际公开发布需要后续明确授权及可用渠道凭据，缺少时标记“候选已完成/公开发布未执行”，不伪称已发布。
- 输入/输出/失败：通过验收的版本+平台矩阵 → release artifacts、可安装包和验证报告；任一声明平台失败则阻塞该平台出包或经明确决策缩小支持矩阵，不静默降级。
- 验证与通过：各平台 `cargo run -p xtask -- package --target <triple>`、`pnpm run verify:packages -- --artifacts <dir>`；执行第 6 节安装矩阵和 `npm pack --dry-run`（各 npm package）；离开源码目录运行 init/base/query、Agent smoke、SDK/MCP 和插件 sample 均成功且无额外隐式下载。
- 保存证据：平台 CI run、制品 SHA-256/版本清单、干净 consumer 日志、optional package 实际选择、安装依赖排除测试、dry-run/渠道核验和未发布原因（如有）。

## 5. 三个业务场景的验收矩阵

### 5.1 真值与验收驱动约定

本节路径、名称、数据和预期均为**待创建且待独立核对的 fixture 设计**，不声称存在于当前仓库或任何真实业务项目。T06 必须完成独立来源/审阅记录才能冻结为真值；Agent 为自己生成的插件同时编造 expected 不满足准确性验收。

两套布局 A/B 使用不同文件位置和注册方式，至少一个前后端分仓，禁止把 fixture 路径硬编码进 Core。真值包包含来源文件 hash、逻辑 identity、expected/forbidden 关系、逐项 gap、数值和来源审阅记录。测试可以从稳定身份重算 ID，不能从待测输出产生 expected。某个不支持模式的正确结果应是 partial+具体 gap，不能把“全部未知”当作 supported 用例通过。

场景驱动目标命令：`pnpm run verify:scenarios -- --scenario 1|2|3|all --transport cli|sdk|mcp|all [--case <验收ID>]`。T16 实现 cli，T22 完成全部传输；负例 X 系列通过 `--scenario all --case Xxx` 选择。每个通过项保存：request、response、truthHash、source/artifact hashes、断言数量、失败明细、回读证据；表中“通过”都是实施验收要求，不是本次运行结果。

### 5.2 场景一：`/#/Other/purchaseLedger` 页面与跨层业务实现

fixture 中配置 `app.web` hash router、明确 service/dataSource，title 示例“采购台账”由 i18n 文案文件提供；包含一条 HTTP 和一条 GraphQL 调用分支。真实项目的 title、方法、接口和表名只能来自其实际来源。

| ID | 输入/扰动 | 必查输出与独立断言 | 通过条件 | 任务 |
| --- | --- | --- | --- | --- |
| S1-01 | route=`/#/Other/purchaseLedger`，指定 app.web | hash path `/Other/purchaseLedger`、原 route 声明、页面文件与 title/i18n key/文案 evidence | route/title/页面 identity 全匹配真值；每个片段 hash/字节可回读；不能只有文件名命中 | T11、T16、T22 |
| S1-02 | 同一路由查页面操作 | 组件、查询按钮/事件、handler、组件方法和调用点 | expected triggers/handles/calls 逐边完整；重复名称组件无误连 | T11、T13、T16、T22 |
| S1-03 | HTTP 分支，经 wrapper/baseURL/proxy | 前端请求封装→service+method+path→注册 handler→业务查询实现 | 每跳有实际绑定证据；能从业务方法反向回到正确页面；同 path 其他服务不连 | T12、T13、T15、T16、T22 |
| S1-04 | GraphQL 分支，与其他业务共用 `/graphql` | operation type/name、schema field、resolver、后端业务方法 | 按 type/field/operation 精确绑定；不能把所有 `/graphql` 请求连所有 resolver | T11、T12、T13、T16、T22 |
| S1-05 | 查询实现读写数据库 | 实际数据源、表/列、model 映射、复合键和字段 lineage | 必需表列全命中且与 datasource 正确绑定；其他源同名表零 confirmed | T12—T16、T22 |
| S1-06 | 去掉 routeHint，仅输入“采购台账增加供应商筛选” | 从业务词、title、注释、模块或 symbol 召回入口、业务实现及关联原因 | 真值中的目标进入首个默认 pageSize，title/注释命中有 evidence，未存在的测试/文件不编造 | T05、T11、T15、T16、T22 |
| S1-07 | 另一个 app 注册同 path；或请求地址动态拼接不可解析 | 多 root 候选、动态值及缺失 binding 的 gap | partial，candidate/requiredEvidence 明确；不默认挑第一个，不给伪完整链路 | T13、T15、T16、T22 |

### 5.3 场景二：功能操作、前后页面、状态与单据

fixture 原始业务说明与源码分别保存，并刻意设置“日志调用在保存之前”等调用顺序反例。目标流程包括提交、审批、接收/入库、撤销/退回与完成分支；实际允许边以独立审阅的流程材料为准。

| ID | 输入/扰动 | 必查输出与独立断言 | 通过条件 | 任务 |
| --- | --- | --- | --- | --- |
| S2-01 | 输入“采购台账的操作流程和前后流程页面” | 当前页面真实操作、前置审批页面、后续收货/退货页面及每个关联理由 | expected 页面/操作/业务边完整，结果能区分导航链接、代码调用与业务先后关系 | T11、T14、T16、T22 |
| S2-02 | 查看提交与审批条件 | DRAFT→SUBMITTED 的数量/供应商校验；SUBMITTED→APPROVED 的角色/审批条件；拒绝分支 | 每条转换含 from/to、trigger、guard 原文/evidence；不能将 guard 简化为无条件 | T14、T16、T22 |
| S2-03 | 部分收货至完成 | APPROVED→RECEIVING、RECEIVING→COMPLETED 的实际数量与有效单据条件 | 状态、分支、聚合判断与原始说明/源码分别有 basis；未知有效口径返回 gap | T14、T16、T22 |
| S2-04 | 查看采购单、收货单、退货单关联 | tenantId+采购单号+行号等有序 keyPairs、关联方向及前后页面 | 复合键全项匹配，关联到正确 documentType；不能只凭单号相同合并跨租户单据 | T12—T16、T22 |
| S2-05 | 只有代码调用顺序，没有流程材料/状态语义 | 可观察 calls 和 `WORKFLOW_EVIDENCE_MISSING` | partial，不生成 precedes/transitions_to，不宣称无后续业务流程 | T14、T16、T22 |
| S2-06 | 原始流程文档与源码状态条件冲突 | 两组独立断言、各自 basis/来源、CONFLICTING_EVIDENCE | 无静默覆盖，无 LLM 自定优先级；输出所需业务确认项 | T14、T16、T17、T22 |
| S2-07 | 删除审批页/新增新流程入口并增量 | 旧页面/操作/业务边过期或撤下，新入口重新采集 | 全量/增量同结果；主题/Skill 对应依赖 stale，旧流程不因采集失败继续显示 fresh | T10、T14、T17、T22 |

### 5.4 场景三：复杂台账取数与数值

fixture 默认报告对象为有效采购明细，每行粒度 `(tenantId,purchaseNo,lineNo)`，schema 显式声明复合键；dataSource `ds.erp` 与 `ds.analytics` 均有同名表，但报告只读取已绑定的 ds.erp。独立真值必须复核以下目标数据和口径：

| 采购明细（均在 ds.erp） | orderedQty | unitPrice | 有效采购状态 | 预期处理 |
| --- | --- | --- | --- | --- |
| A/P1/1 | 10 | 5 | APPROVED | 输出本租户行 |
| A/P1/2 | 4 | 20 | APPROVED | 输出本租户行 |
| A/P2/1 | 3 | 7 | APPROVED | 即使无收货仍保留 |
| A/P3/1 | 9 | 9 | CANCELLED | 排除 |
| B/P1/1 | 100 | 1 | APPROVED | 查询 tenant A 时不得混入 |

明细 A/P1/1 的收货记录：POSTED 且未删数量 3、2；另有 CANCELLED 数量 9、已删除数量 20，均排除；有效退货数量 1；两条有效发票金额 8、12。A/P1/2 有效收货 1，其他有效明细无收货/退货/发票。供应商表 `(tenantId,supplierId)` 为复合键，跨租户同 supplierId 设置不同名称。

计算约定需有独立口径依据：grossReceived=sum(有效收货)，netReceived=grossReceived-sum(有效退货)，outstanding=orderedQty-netReceived，purchaseAmount=orderedQty*unitPrice，invoiceAmount=sum(有效发票金额)。金额用定点十进制/整数最小货币单位核验，不以二进制浮点容差掩盖差异。

| tenant A 结果行 | grossReceived | netReceived | outstanding | purchaseAmount | invoiceAmount |
| --- | --- | --- | --- | --- | --- |
| A/P1/1 | 5 | 4 | 6 | 50 | 20 |
| A/P1/2 | 1 | 1 | 3 | 80 | 0 |
| A/P2/1 | 0 | 0 | 3 | 21 | 0 |
| 合计 | 6 | 5 | 12 | 151 | 20 |

fixture 同时包含：正确的按明细预聚合收货/退货/发票再关联的现有 SQL/ORM；错误的多明细直接 join 导致 fanout 的实现；缺 tenantId 或 lineNo 的关联；错误状态过滤；更短但统计粒度错误的路径。独立 oracle 从原始明细逐组计算，与执行本地正确 SQL 的结果交叉核验；不能调用待测插件或 Core report 算法生成 expected。

| ID | 输入/扰动 | 必查输出与独立断言 | 通过条件 | 任务 |
| --- | --- | --- | --- | --- |
| S3-01 | 输入“采购台账如何取已收、未收及金额” | 已有 SQL/ORM 文件、queryImplementation 和调用入口 | 精确定位真值中的现有实现与源码；不新造查询冒充已有实现 | T12、T14、T16、T22 |
| S3-02 | 追踪每个结果字段 | 来源表列、表达式、join keyPairs、基数、NULL 处理和单位 | 每个目标字段有完整 lineage 或明确 gap；supplierName 必含 tenant 绑定；不同源同名表不串 | T12—T16、T22 |
| S3-03 | 查粒度和多表聚合 | 采购明细复合 grain、收货/发票 1:N、预聚合位置、过滤时机 | 不能将单头/明细粒度混合；输出独立依据，未知基数返回 unknown+gap | T14、T16、T22 |
| S3-04 | 查询 tenant A 数值 | 上表三行与合计；有效状态/删除/无匹配行 | 本地正确 SQL 与独立 oracle 逐值相等，三行不丢不重；金额精确一致 | T06、T14、T16、T22 |
| S3-05 | 错误直接 join/fanout 或丢复合键 | 错误关联证据、重计风险、缺失键与不符数值 | 不把更短路径标为正确；能给明确风险/缺口，错误实现不通过正确性验收 | T13—T16、T22 |
| S3-06 | 缺有效状态/时间字段/业务 grain 材料 | 对应 unknowns 和 requiredEvidence | partial；不从字段名自动认定生效时间，不套默认状态，不输出伪确认口径 | T14、T16、T17、T22 |
| S3-07 | 修改 SQL 聚合或状态过滤后插件失败，再回退旧制品 | 旧 report 断言和主题 stale，错误/中断信息保留 | 失败或回退不能使旧值 fresh；当前来源成功重采并重新导入主题才恢复 | T09、T10、T17、T22 |

### 5.5 共用负例、异常与增量矩阵

| ID | 触发 | 必需行为/通过条件 | 任务 |
| --- | --- | --- | --- |
| X01 | 同名 method/component/endpoint/table，跨 scope 或仓库 | 不同 identity；无真实绑定零 confirmed；candidate 附歧义/所缺依据 | T02、T11—T13、T22 |
| X02 | ds.erp 与 ds.analytics 同名表；同 ORM 类型绑定不同实例 | dataSourceId 全程保留，默认不跨源；查询和证据表能区分 | T03、T12、T13、T22 |
| X03 | tenant+document+line 复合键漏任一列、顺序/类型冲突 | 不接受完整关系语义，返回 KEY_MAPPING_INCOMPLETE 或 candidate；不得靠单字段同名补齐 | T02、T12—T14、T22 |
| X04 | 动态 URL/GraphQL field/SQL/运行时实例 | 保存可观察表达式和 DYNAMIC_TARGET gap；confirmed 目标不猜测 | T11—T14、T22 |
| X05 | 分仓，代理/baseURL/服务名修改或无 binding | 正确配置才连接；变化使依赖闭包 stale，重采后反映新服务 | T03、T10、T13、T22 |
| X06 | 删除文件、删除全部匹配文件、成功空输出、源文件改名 | 旧批次事实撤下/过期，无可见悬空边；其他生产者事实保留 | T04、T08、T10、T22 |
| X07 | 新增文件、候选 0→1/1→2/2→1、同身份属性变化 | 匹配集合摘要变化，相关 extract/link 重新运行，全量与增量一致 | T03、T08、T10、T13、T22 |
| X08 | 插件非零退出/坏 JSON/超时/输出洪泛/缺依赖/非法 evidence | 统一错误、无部分导入；旧批次按真实新鲜度展示；没有泄露 key | T07、T08、T10、T22 |
| X09 | 候选验证中源码/配置/制品/运行时改变，或两候选并发 | 验证失效/ACTIVATION_CONFLICT；活动版本与事实一起切换，无混合 generation | T09、T10、T22 |
| X10 | 运行中杀宿主，发布后未 C，C 内，C 后未答复 | 重启后只有提交前或提交后完整状态；孤立制品不加载；旧失效不复活；进程清理有 owner | T07、T09、T10、T22 |
| X11 | index --base、插件失败、回退/源码内容还原 | 业务/知识 stale；只读核验不持久修改；只有新成功批次恢复 fresh | T09、T10、T17、T22 |
| X12 | 旧引用/分页 token、跨库 ID、预算超限、大图环 | SNAPSHOT_EXPIRED/引用错误或 partial+frontier；完整 JSON，时间/字符/visited 有界 | T15、T20、T21、T22 |
| X13 | 查询试图触发迁移/生成/插件发现；query MCP 调写工具 | 明确拒绝，磁盘/DB/活动指针无写入，模型/插件调用次数均 0 | T05、T15、T20、T21、T22 |
| X14 | generated Skill/主题/无依据 conventions 重新作为源码，主题互引 | 循环或生成来源拒绝；原始来源链可追；相关变更使导出 stale | T02、T08、T17、T19、T22 |
| X15 | Agent 连续修复失败、耗尽预算/取消、autoActivate=false | 准确 failed/partial/pending_activation；保存错误与累计预算，不靠删 case 过关，不生成当前正式 Skill | T18、T19、T22 |
| X16 | 缺 Node/Python、缺 native、错架构、没有模型 key | 只影响依赖该组件的能力；Core 基础索引独立可运行；错误指出安装项 | T07、T18、T20、T23 |

### 5.6 需求追踪与门禁

| 需求组 | 实施任务 | 必验项 |
| --- | --- | --- |
| 多仓/范围/忽略/身份/基础索引 | T02—T05 | S1-06、X01、X02、X05—X07、X16 |
| 协议/位置/ID/扩展/错误/事实分类 | T02、T04、T08 | 全场景 evidence 回读、X01—X04、X08、X12、X14 |
| SQLite 约束/owner/替换/快照/事务/新鲜度 | T04、T08—T10 | X06—X11、S3-07 |
| 四类采集、注释/模块/自定义入口、extract/link | T07、T08、T11—T14 | S1 全部、S2 全部、S3-01—S3-03、X01—X08 |
| 业务词/路由/title/注释/模块/符号、双向有界查询 | T05、T15、T16 | S1-01—S1-07、S2、S3、X12 |
| SDK/MCP 同源、分页预算/partial、只读 | T02、T15、T20—T22 | 全场景三端对照、X12、X13、X16 |
| Rig/通用 Prompt/凭据/工具/有限修复/外部 Agent | T18、T19、T21 | S1—S3 真实模型生成后的独立验收、X08—X11、X15 |
| 自动登记/不可变制品/原子激活/回滚/并发中断 | T09、T10、T19 | X07—X11、X15 |
| 主题/源码引用/Skill/版本绑定/过期/防循环 | T17、T19 | S2-07、S3-07、X11、X14、X15 |
| 安装/构建/平台/发布 | T00、T01、T20、T21、T23 | X16、第 6.1—6.3 节 |
| 独立真值、数值和误关联 | T06、T11—T16、T22 | 每类 expected/forbidden 分母完整；S3-04 数值；合成 smoke 不计独立准确率 |

门禁：冻结 supported fixture 的所有必需 expected 召回率 100%，所有 confirmed 结果精确率 100%，forbidden confirmed=0，所有预定 gap 存在且原因正确，S3 数值逐项精确相等。候选集范围、覆盖范围、分母和样本来源必须随报告保存；不能把 fixture 指标外推为所有项目准确率。

## 6. 安装、发布和最终完成条件

### 6.1 安装依赖与交付关系

| 安装目标 | 最终用户需要 | 不应被隐式要求 | 安装验收 |
| --- | --- | --- | --- |
| CLI 基础索引/查询 | 本机平台 CLI 二进制、可读源码目录、可写索引目录（维护时）；SQLite 随 bundled 构建 | Rust/Node/Python、LLM、业务 DB、SCIP、Repomix | 干净环境 init→index --base→search→readSource→只读 status；不访问模型和包源 |
| CLI 内置 Agent | 含 Agent 的 CLI、provider/model/baseUrl（如有）、apiKeyEnv，模型网络连接 | Node.js Agent 进程、Pi/Goose 安装 | 带工具的真实模型 smoke、有限失败及脱敏；缺 key 时基础 CLI 仍可运行 |
| Node native + SDK | Node 24.x、对应 OS/arch/libc 的预编译 native、可读现有配置/库 | Rust 编译器、C/C++ linker、Python、SQLite npm driver、LLM | 离开源码目录由 npm pack 包安装并查询同一库；读 open 不创建或迁移库 |
| stdio MCP | Node 24.x、MCP 包及其 SDK/native 依赖；MCP 客户端配置启动命令 | Rust CLI 安装、内置 Agent、HTTP 服务器或某个 Web 框架 | 官方 client stdio initialize→tools/list→call/resource→cancel→close |
| Node 插件 | manifest 要求的 Node（示例 24.x）、已准备 npm 锁依赖 | 业务项目依赖安装、LLM、目标应用运行 | 外部提交→验证激活，之后离线重复采集 |
| Python 插件 | manifest 要求的 Python（示例 3.12.x）、已准备 venv/hash-lock 依赖 | Node、Rust 编译工具、LLM、在线业务 DB | 同上；缺 Python 只失败该插件，基础查询仍运行 |
| 其他插件运行时 | 显式 runtime ID/版本/绝对 executable 路径，统一 JSON 协议 | 为新语言新增 Core 查询/存储逻辑 | 同一协议向量和错误/超时/取消测试通过；不对未验证 runtime 宣称支持 |
| 从源码构建 | 锁定 Rust 1.97.1/Node 24.14.0/pnpm 10.x、平台 linker/C 工具；测试 Python 示例需 3.12.x | 不要求业务数据库或用户模型密钥运行普通测试 | `--locked`/`--frozen-lockfile` 全构建；模型 smoke 单独记录，不能静默跳过后算通过 |

依赖图：CLI(Core-only)→Core；CLI(default)→Core+Agent→Rig；native→Core；SDK→native；MCP→SDK；外部插件为 Core 子进程，不成为 SDK 算法依赖。npm 包名均为待核验命名权限的目标名称；变更公开名称时一次更新 loader、optionalDependencies、docs 和验收 consumer，不触碰协议。

### 6.2 首版平台矩阵

| OS/架构 | Rust target / native 后缀 | CLI/Core/Agent | SDK/MCP | 插件/进程验证 |
| --- | --- | --- | --- | --- |
| Windows x64 | `x86_64-pc-windows-msvc` / `win32-x64-msvc` | 必须原生构建与运行；路径/原子发布/Job Object | Node 24，预编译包实际加载 | Node/Python 模板、中文空格路径、进程树取消、中断恢复 |
| Linux x64 glibc | `x86_64-unknown-linux-gnu` / `linux-x64-gnu` | 固定构建 runner/container 并记录实际 glibc 版本，不承诺更低版本 | Node 24，libc 条件正确 | 进程组清理、WAL/backup、只读、干净安装 |
| macOS x64 | `x86_64-apple-darwin` / `darwin-x64` | 原生或明确可复现交叉构建后在 x64 运行 | Node 24，实际 consumer | Node/Python 模板、cancel/close、资源路径 |
| macOS arm64 | `aarch64-apple-darwin` / `darwin-arm64` | 原生构建和运行 | Node 24，不能误加载 x64 包 | 同上，制品架构核验 |

最低 OS/glibc 不是本次已经确认的事实，由 T23 在发布 manifest 写入实际构建与干净安装验证边界；未测环境不列入支持。首版不发布 Windows arm64、Linux arm64/musl、浏览器或 WASM。CI runner 的可用名称在 T23 核对，不将旧 workflow 的 runner 名称视为仍可用。

### 6.3 发布候选清单与顺序

1. 固定产品版本、protocolVersion、schemaHash、Cargo/package/lock hash；四平台通过 T22/T23。打包资源路径在安装后可定位；无 workspace 相对路径泄漏。
2. 产生各平台 CLI 压缩包、SHA-256、license；默认含 Agent，另提供 Core-only 构建说明或制品，命令 capabilities 明确区别。
3. 汇总四平台 `.node`，生成 optional platform packages，核验 loader/os/cpu/libc/版本；然后 pack native root、SDK、MCP。无正确平台包时立即报告，不在用户安装时编译或下载未核验二进制。
4. 每平台 fresh consumer 装包并执行安装矩阵；打包文件检查 Prompt/Schema/模板和必要资源齐全，密钥、候选 work、fixture 数据库和本地缓存不进入发布包。
5. 如后续明确授权公开发布：先平台包→native root→SDK→MCP，再 CLI release；每步只发布已核验 hash，任一步失败保存状态并停止依赖包发布，禁止重发不同内容的同版本。公开渠道凭据只在 CI secret 使用。
6. 数据库 schema 升级只在显式维护命令进行，先保存可恢复备份和版本；不做旧项目数据库迁移，也不允许旧 SDK 静默降级新库。安装/升级文档说明当前可支持版本及重建索引方式。

### 6.4 最终完成条件

- T00—T23 全部 `done` 且证据可复现；历史审查若仍缺失，单列未取得资料及已覆盖回归，不能写“已阅读”。公开发布未获授权时，T23 可按“可安装发布候选交付”完成，但必须明确未公开发布。
- 全部协议 DTO、Schema、类型生成、错误、CLI、SDK、MCP、Agent 工具字段一致；契约生成检查无差异，任务依赖有效无环，需求追踪无空项。
- 无 LLM 基础索引、手写外部插件提交、Rig 生成激活、无 LLM 日常增量、三端业务查询、主题/Skill 生成与过期重建分别有真实执行记录。
- 第 5 节 supported 真值指标、数值和 forbidden 门禁全通过；独立来源审阅与真实项目抽样不能用 Agent 自生 case 替代。真实项目缺资料时明确限制已验收范围，不声称未知业务已覆盖。
- 正常、失败、取消、中断、并发、文件新增/删除、候选集合变化、过期、回退、制品篡改均通过；旧失效批次不会错误恢复 fresh；只读调用零维护/模型/插件执行。
- 四个平台按声明边界通过构建和安装，基础 CLI/Agent/SDK/MCP/插件运行时各自依赖明确；不存在“在开发机 node_modules/target 缓存可运行”作为唯一交付证据。
- 未完成或未验证内容以精确任务/失败证据标出，不能通过改状态、删验收或宣称框架自动具备能力完成本计划。

## 7. 执行记录模板

每次执行追加一条；任务状态在第 4 节对应任务同步更新。恢复执行时先读最后记录、当前依赖和已保存产物，不重做已完成工作，不把旧命令成功视为新版本成功。

```markdown
### <日期时间 Asia/Shanghai> — <任务ID> — <执行者/运行ID>

- 状态：todo → doing → done / blocked（只写实际发生转换）
- 前置依赖：<ID、完成记录路径/证据hash>
- 工作目录/源码版本：<绝对路径；有 Git 则 commit，无 Git 则文件清单hash>
- 输入：<配置/来源快照/制品/Schema/truthHash/目标与范围>
- 本次具体能力：<可观察结果>
- 修改文件：<实际路径及目的>
- 决策变更：<D/U编号、原因、官方版本来源、影响任务；无则“无”>
- 命令与结果：
  | 命令（含工作目录） | 工具版本 | 退出码 | 匹配/通过断言数 | 日志路径 |
  | --- | --- | --- | --- | --- |
  | ... | ... | ... | ... | ... |
- 输出：<Snapshot、batch/run/validation/activation ID、制品hash、文档manifest>
- 正常与异常验证：<具体验收ID、结果和证据路径>
- 新鲜度/覆盖：<base/collectors/knowledge；fresh/stale/unknown；未覆盖项>
- 独立真值：<来源/审阅记录；合成smoke单列；不能填写模型自评>
- 失败/中断/回滚：<发生阶段、正式库是否提交、旧批次状态、恢复动作>
- 预算：<模型请求/工具调用/修复次数/总耗时；普通采集模型调用必须0>
- 未验证项或阻塞：<缺失资料、外部条件、失败日志；无则“无”>
- 下一可执行任务：<依赖已完成的ID及第一条实际命令>
- 完成证据：<docs/evidence/<任务ID>/... 的清单与hash>
```

本文编制时的自检范围：资料可用性、官方依赖资料、目标契约、任务依赖和需求覆盖；不是实现完成、准确率通过、平台支持通过或任何产品命令已成功执行的证据。
