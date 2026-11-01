# 采集与候选维护（当前实现）

所有维护命令需要先执行基础索引。Core 暂存链路已支持 Node 24、Python 3.12、锁定依赖环境和 manifest 登记的扩展类型，验收范围为合成协议/事务 smoke。

## 外部候选

```sh
project-knowledge plugin --config project.json submit --path candidate-directory
project-knowledge plugin --config project.json prepare --candidate candidate_ID
project-knowledge plugin --config project.json validate --candidate candidate_ID
project-knowledge plugin --config project.json activate --candidate candidate_ID
project-knowledge collect --config project.json
project-knowledge collect --config project.json --phase extract
project-knowledge collect --config project.json --phase link
project-knowledge plugin --config project.json rollback --plugin plugin.id --version 1.0.0
```

也可直接 `plugin ... validate --path candidate-directory`，响应会包含新 candidateId。
无依赖可跳过 prepare。带依赖时先 submit → prepare → validate；缺失缓存会明确报错。
`collect --manifest manifest.json --validate-only` 可验证尚未激活的候选，不修改正式库 generation。
正式 collect 只使用活动版本；显式 manifest 必须与活动制品哈希一致。

`collect` 默认 `--phase all`，在 backup API 制作的暂存 SQLite 中移除目标阶段的旧指针，
按 DAG 执行 extract、重新导出候选、执行 link，然后运行 acceptanceCases。
任何运行、Schema、证据、依赖或 case 错误都会阻止整组正式提交。
显式 extract 提交会使依赖旧批次的 link stale；空输出同样替换旧事实。

## 发现与激活策略

真实项目接入以 AI 调查并生成项目插件为主：读取该项目实际源码与配置，生成独立的 manifest、提取/关联脚本、锁文件和 case，再走现有准备、验证、登记和激活入口。
框架注册、目录约定、ORM 映射与业务状态保存在项目制品及带引用的调查材料中；SDK/native/Core 提供通用协议、验证、执行、存储与查询。
接入新项目时重新生成插件，公共 SDK/Core 无需项目专属修改或重新发布。示例目录只用于协议和验收，不作为所有项目的默认适配逻辑。
内置 Rig 的 configure/build/repair/resume 与 `.env` 接入见 [agent-usage.md](agent-usage.md)。外部 AI 生成的候选继续使用同一 submit/prepare/validate/activate 路径；独立业务验收、派生约定导入及主题/Skill 输出仍按 T06/T17/T19 的未完成项核验。

日常更新使用 `project-knowledge index --config project.json --incremental`。
Core 重扫文件/ignore/配置、核验活动制品与运行时/依赖缓存，比较完整来源集合和候选断言集合，
先提交受影响批次及下游的失效，再按稳定 DAG 在暂存库运行受影响的 extract/link，最终一次提交业务组。
响应的 `executed` 列出实际运行的阶段，`invalidated` 列出失效范围。未变化的阶段和仓库保留 batchId；无变化更新不推进正式 generation。
新增、删除、成功空输出、候选从零匹配到唯一/歧义、属性变化都参与比较。
普通增量不会发现新候选、联网安装依赖或调用模型；需要新制品时显式提交、准备和验证。

`--base` 只刷新基础层并使业务批次 stale，不要求 Node/Python。显式 collect 执行前也会登记已确认的输入变化，
基础来源过期时需先更新索引。失败、取消或宿主退出保留已提交的失效状态；内容改回或重启不会让旧 batch 变 fresh，
下次成功运行生成新批次。分阶段 extract 提交会使下游 link stale，之后可增量补齐。

`plugin --config project.json discover` 是显式维护入口：按 collectors 和 plugins.directories 发现候选，
按 ID/version/hash 去重，再验证，并遵循 autoActivate。
autoLoad=false 停止自动发现；已经激活的版本仍可 collect。
autoActivate=false 将自动处理停在 validated；显式 activate 仍可主动激活。
普通查询和 status 不发现、不修复、不运行插件。

多个相互依赖的新候选可重复传入 `validate --candidate candidate_A --candidate candidate_B`。
同组共用 validationId，激活任一成员会一次提交全组及受影响活动阶段；任一成员失败则全组失败。
Core 暂存 API 的单独提交不能冒充活动版本切换。

## 绑定、发布与恢复

显式 prepare 使用配置的 Node 执行 npm CLI（ci --ignore-scripts），或 Python 创建 venv 并执行 pip install --require-hashes --only-binary=:all:。依赖缓存位于宿主用户 cache/project-knowledge/dependencies，隔离业务 node_modules；其键包含 OS/arch、运行时路径/版本、制品与锁摘要。ESM 入口使用缓存中的制品副本，Python 使用 venv，Windows 的长路径包目录由 Core 设置。

缓存完整文件树、链接目标和 seal 绑定 integrity。validate/collect/activate 不安装依赖，执行前后与提交前核验；新增、删除、改写缓存文件都会拒绝。失败安装不发布 seal；跨进程 OS 锁防止同时安装。每次插件 cwd 使用独立临时目录，宿主不会继承 NODE_PATH/PYTHONPATH/PYTHONHOME/VIRTUAL_ENV；模型 key 不透传。并发、完整 I/O 超时、两个输出流上限与 Job/process group 清理受 Core 控制。这些措施不构成 OS 沙箱。

`pnpm run verify:dependencies` 对真实 npm/PyPI 锁进行显式联网安装，再验证复用、坏锁和篡改拒绝。它以安装日志/mtime 不变证明后续未运行安装器，没有使用网络命名空间断网。

manifest、入口、artifacts、case、inputFixture 和 truthRef 都纳入哈希；路径拒绝越界、目录链接和重解析点。
运行时使用配置的绝对路径，未配置 Node/Python 时从 PATH 解析为绝对路径并检查版本。
versionRange 当前接受空格分隔的数字比较条件，例如 `>=24 <25`，其他写法会明确拒绝。

验证结果绑定 generation、来源快照、配置、活动版本、运行时、集合、case 和暂存结果。
发布在目标父目录创建临时目录、逐文件 sync 后 rename 到
`.project-knowledge/collectors/<pluginId>/<version>/<artifactHash>/`。
发布后不同哈希必须使用新 version；数据库事务一次切换活动版本、整组事实、搜索投影及知识状态。
同 generation 下并发验证的两个候选至多一个能通过 CAS。

单项目 OS 文件锁保护维护入口，进程退出自动释放。维护恢复将旧 prepared 候选和 running 记录标为失败或 interrupted，
并结束未提交的 pending 激活；已经提交的激活可幂等查询确认。未被数据库引用的目录不会自动加载。
暂存库位于 `.project-knowledge/work/staging/`，每个目录另持独立 OS 锁和所有权标记。
恢复只回收已释放锁的本项目目录，并复核最终绝对路径；并发 validate-only 仍持锁的目录会保留。
源码或运行时变化造成的持久 stale 不因候选失败、内容改回或回退而清空；回退会重跑并生成新批次。

## 验收证据边界

acceptanceCases 的 inputFixture 是相对制品根的 CollectorInput JSON，按其中 stageId 重放插件；
expected 支持必需实体/关系、禁止关系及必需 gapCodes，失败返回 JSON Pointer 和 repairHint。
truthRef 如存在，必须是制品中的本地文件，并与 truthHash 相等。
候选自带 case 始终标为 candidate 来源，不因此升级为独立业务真值。

`node scripts/verify-collection.mjs` 保存可复现的合成 CLI 闭环、命令退出码、响应和哈希。
生命周期测试覆盖五个可控故障点和实际目录发布 I/O 错误；Windows 还实际终止验证宿主，
断言插件与孙进程退出、OS 锁释放、正式 generation 不变以及维护重启恢复。
同一测试还覆盖并发维护保留活动 validate-only 暂存目录、宿主退出后回收该目录。
T09 现已在 Windows/Linux 事务 C 内真实强杀 owned 宿主，并验证 SQLite FULL；Linux 还在独立 4 MiB tmpfs 上验证真实 ENOSPC 发布失败与恢复。
T10 包含增量与干净重建的实体/关系/来源/证据/搜索/覆盖比较，以及 Windows 增量 CLI 强杀后的后代清理与失效恢复。
物理断电与完整四平台矩阵仍未验证，详见各任务证据索引。
T06 已冻结独立来源包与合成样本，truthHash 和来源审阅见 docs/evidence/T06；样例通过不代表生产业务准确率。

## 数据库示例插件（T12）

`collectors/examples/frontend-backend-{a,b}/database/manifest.json` 提供带 hash-lock 的 Python 插件：A 用 TypeScript Tree-sitter AST，B 用 Python AST，两者用 SQLGlot 30.24.0 解析 SQLite DDL/SQL。只识别明确 connect/model/query/findAll 模式，不执行源码或 SQL、不连接业务数据库；日常采集不安装依赖、不调用模型。

`node scripts/package-database-examples.mjs --check` 核对自包含制品；`node scripts/verify-plugins.mjs --capabilities api,database` 经 Core prepare→run→case→validate→commit 运行验收。原始样本缺少 import 的 erp 调用仍是 UNBOUND_INSTANCE，带完整导入的补充样本单独冻结于 fixtures/truth-supplements/T12-database。

每种布局验收两个数据源、10 张表、62 列、16 个有序键和已写出的 SQL/ORM 查询；列 nullable 未有明确 NOT NULL 时保持 unknown。SQL joins 保存类型、完整条件与有序 keyPairs，不能确认完整键时返回 gap；即使存在外键，实际查询的 cardinality 仍为 unknown。业务粒度/状态/时间/NULL 策略/单位留待 T14，不把 T12 的语法结果当作完整台账口径。

`node scripts/audit-source.mjs --output <report.json>` 生成零字节、注释文件、空导出模块、显式占位及源码哈希清单；自动扫描不能代替任务验收。当前 SDK/native/MCP、图遍历、知识输出和场景组合仍有未完成实现，按 T15—T21 状态继续处理。

## 跨仓库关联插件（T13）

`collectors/examples/frontend-backend-{a,b}/link/manifest.json` 在同组 frontend/api/database extract 后执行，直接引用已有实体 ID，不重复制造方法、端点、表或数据源。它只从输入中完整的候选断言、关系及原始 evidence 进行关联：HTTP 需要显式 clientToService、静态客户端配置和后端注册；GraphQL 需要实际字段选择和 resolver 注册；查询的数据源取自已证明的、带数据源身份的表列读取。

缺绑定时，即使只有一个同名目标也保留 candidate；多 resolver 不生成 confirmed calls。代理还需要显式 proxyToService 与可证明的前缀改写；未知代理不能沿用旧服务目标。人工原始断言参与的关系保留 declared/human。每条关联都有其实际调用、配置或注册证据，不能只凭端点存在确认。

`node scripts/package-link-examples.mjs --check` 检查自包含制品；`node scripts/verify-plugins.mjs --capabilities link` 通过 Core 的登记、整组验证、激活和无模型增量验证两布局。候选缺失、重复、截断或与导出实体不一致均返回 CANDIDATE_SET_INCOMPLETE。候选摘要包含相关关系断言，新增 resolver 或更改代理/数据源后重新判定；四组规范化增量/干净重建比较保存在 docs/evidence/T13。

## 流程和台账语义采集（T14）

`collectors/examples/frontend-backend-{a,b}/semantics/manifest.json` 包含 workflow extract 和 report link 两阶段。依赖现有数据库、前端/API 和 link 制品，在同一 Core 维护链路运行；`node scripts/package-semantic-examples.mjs --check` 检查完整制品，`node scripts/verify-plugins.mjs --capabilities workflow,report` 执行验收。

workflow 解析明确配置的状态参数/属性、原始文档转换、guard、单据有序键和业务页面先后。状态写入核对未被重新绑定的参数所有者；其他对象返回 WORKFLOW_STATE_OWNER_UNKNOWN。文档缺失时仍保存有源码依据的操作、状态写入和 calls；业务 transitions/precedes 保持缺口。gross/net 条件冲突保留各自断言，不自动裁决。

report 保存已有 SQLite SELECT/CTE 的字段表达式、聚合及列来源，追踪预聚合、完整粒度、必要状态/删除过滤和实际零回填。标记 JOIN_FANOUT_RISK、CHILD_FILTER_AFTER_LEFT_JOIN、PREAGGREGATION_GRAIN_MISMATCH；OR 条件或相同列名不能证明必需过滤成立。台账粒度/状态/NULL/单位均须有独立原始 policy；没有依据则 unknown+gap。货币字段只声明原文可证明的 cent 单位，未声明的物理数量单位保留缺口。只有两侧键均有完整唯一性依据时声明 1:1；其余基数未知。

业务规则留在独立插件的 conventions.json 和原始来源；Core/SDK 没有采购专用判断，也不执行用户 SQL。当前是两布局合成样本的源码审阅验收，范围为支持的静态语法；Core 的页面/流程/台账场景组合仍由 T16 实施，不把采集完成等同于场景查询完成。
# T16 场景查询接入后的采集补充

数据库采集对有完整同数据源声明的 `*`/`alias.*` 保存真实列 reads；重复输出名称不形成唯一的下游 CTE 绑定。未知范围继续 gap，不扩成其他数据源的同名表。Core 的搜索投影保留嵌套属性中的原始字符串，因此 queryImplementation.statement.text 可用于定位已有实现。

冻结 A/B service 原文没有导入 erp，不能据同名变量连接数据库。T16 先验收该缺口，再在自有临时目录使用 fixtures/truth-supplements/T16-bindings 中单独审阅的显式 import；原始 50 个冻结文件未改动。源码、补充理由、预期列与 hash 见该目录 review.json，不能将补充样本外推为生产 ORM 支持率。
