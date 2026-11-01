# T16 Core/CLI 场景闭环

四个场景 execute 已替换搜索骨架：page 按已登记 router mode/原路径/alias 选根并沿实际调用、请求、注册和读写扩展；locateFeature 区分根与遍历过滤；workflow 保留业务状态、guard、页面和单据关系；report 按生产者保留已有实现、字段来源与口径。全部走同一只读入口、快照、预算、取消和原始引用校验。旧 scenario_skeleton 已删除。

## 实际证据

`node scripts/verify-scenarios.mjs --scenario all --transport cli`（也可用 pnpm verify:scenarios）实际构建 CLI，再由同一 A/B 临时索引对照 Core/CLI。cli-report.json 记录 6 个主响应、命令/退出码/日志与 CLI SHA-256；另外保存自然语言词项召回、反向页面路径、跨入口分页、动态路由和缺流程资料反例。没有由 TypeScript 重写图或业务算法。

CLI 在测试前复制到 target/verified-cli/<SHA-256>/，并针对该副本验收；后续 Cargo 测试重建 debug 文件不会替换已验证的制品。实际副本路径在 cliArtifactPath 中。

- 每布局页面：实际标题/i18n、组件/事件/操作、HTTP 与 GraphQL 目标、业务方法，以及正确 ds.erp 的 7 个 purchase_detail 列。反向可回到原 route；alias 等价，路径 suffix 不匹配，编码分隔符不变成目录分隔，大小写保留。
- 冻结 service 原文缺少 erp import，先验证 PAGE_DATA_SOURCE_MISSING；再在自有临时目录使用单独审阅的 T16-bindings 显式 import，才确认数据库链。未修改 50 个冻结文件，未凭同名变量补边。
- 每布局流程：6 状态、14 个独立原文/代码转换断言，gross/net 冲突及 CONFLICTING_EVIDENCE，两个含 3 键对的单据关系。去掉文档后，codeCalls 保留而 transitions/precedes 不出现。动态路由多 root 保持歧义，不按框架优先级猜一个。
- 每布局报表：9 字段完整列集合与 T14 独立审阅相等，4 个有序 join、3 键 grain、状态/NULL 等原始口径保留。时间、未声明单位和不能证明的基数仍 unknown；fanout/WHERE 子表过滤作为已有错误候选与具体 gap，不按路径长短选正确实现。
- 六个主响应的 108 个原始引用逐一回读 current（A 28+26+4；B 26+20+4）。CLI 与 Core 规范化后相等，搜索首游标也可跨两种入口使用。
- numeric-sql.json 保存本地 fixture SQL 的实际/预期逐行与合计、空差异列表及两个错误 SQL 的反证。独立 Python 按键整数累加也得到 `[6,5,12,15100,2000]`，不以浮点容差比较。Core/CLI 查询自身不执行业务 SQL、不连接业务数据库。

## 修复与边界

查询用到了原先未被索引的嵌套 statement.text，已在通用搜索投影保留原始嵌套字符串。SQL AST 对完整同数据源 DDL 的 `*`/`alias.*` 保留列 reads；重复输出名称不能冒充唯一 CTE 列。无查询参数的 CodeMap 现在返回有界文件/声明地图。

跨端对照发现 Rig 启用 serde_json preserve_order，数组长度并列时的 Map 遍历顺序使裁剪不同，请求 hash 也会受影响。D25 使用显式字段名并列规则和 Core canonical_json，另外预留时钟/计数宽度；完整失败 JSON、诊断日志和两种 cargo feature 图保留，不通过放宽比较消除差异。字符预算包含最终信封；序列化阶段也检查取消/超时，截断保留有界后续查询。

`python docs/evidence/T16/verify.py` 复核上述 CLI 证据和二进制 hash，再运行全部 business_scenarios、全部 indexing、Agent/CLI 回归、全目标 Clippy/fmt、Schema/类型、五组制品一致性、冻结真值、源码与计划检查。实际结果和源码/日志/快照 hash 见 verification-report.json。

本轮 Windows x64；无真实远程模型调用。自然语言 fallback 仅显式词项召回并标 partial，不声称解释任意业务问题。生产准确率、知识/Skill、SDK/MCP、四平台交付分别由后续任务验收。
