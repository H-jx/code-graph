# T15 通用查询与空实现补齐

原有 graph/{neighbors,paths}、codemap/{rank,render} 四个注释占位均已替换成实际逻辑，并由统一查询入口调用。查询组合不执行插件、维护命令、模型或业务 SQL。Rust 新增 neighbors DTO（D24），Schema/TypeScript 同源生成；CLI 通过 `query --request` 使用同一契约。

## 验收范围

- 搜索合并 FTS、精确路由、实体 ID/规范身份，逐断言保留 basis/origin/evidence。支持字段过滤、仓库隔离及中文注释；按精确等级、BM25、实体/断言 ID 稳定排序。30 个独立构造的同类符号分页逐项一致。
- 单根邻接 in/out/both 分页与多层 BFS 分开。58 条有回环的独立压力关系无重无漏；每跳包含实际方向、限定条件、生产者、断言及原始引用。根快照跨代/跨库拒绝，改请求不能沿用游标。文件根可以继续访问符号，不把根类型当作所有后继类型。
- 7,000 字符预算下，搜索与邻接连续翻页仍逐项一致；裁掉尾部完整投影后游标只推进到最后返回项。完整紧凑 JSON（含 envelope）的 Unicode scalar 数等于 responseChars，未裁断 JSON。最小信封容纳不了返回 BUDGET_EXHAUSTED。
- 多层路径遵守实体/关系/visited/深度/时间预算，去重排队，截断返回有界 frontier/nextQueries，不提供 BFS 恢复 token。visited 计实际检查的候选、关系与来源扫描项；去重实体另见 data.entities，不混称为唯一实体数。
- 查询先固定读事务快照，开启 query_only，Core 循环和 SQLite VM 共用取消令牌。默认 verify 使用相同来源选择规则和有界扫描；不完整核验为 unknown/partial，发现源码集合变化后默认排除旧事实。indexed/allowStale 均不宣称磁盘 fresh。
- readSource 校验原样快照及存储中的原始字节范围；篡改范围拒绝。删除文件后默认片段为空，显式 allowStale 才返回带 storedVersion 的历史片段。Agent 复用该 Core 校验和取消，不另行维护查询校验规则。
- CodeMap 按入口、直接依赖和同文件原始证据选择文件与声明轮廓，保留签名、位置、来源与作用域；不拼函数体。不支持轮廓的语言列入 uncoveredLanguages，并标记 partial。

## 证据与复现

`python docs/evidence/T15/verify.py` 运行索引全回归、Agent/CLI、扩展采集后搜索回归、工作区 Clippy、fmt、契约/类型/边界、冻结真值与独立整数 oracle、源码扫描和计划核对。verification-report.json 保存实际命令、退出码、源码/日志/快照 SHA-256；tools.json 是本轮实际工具版本。

search.json 保存分页结果、查询前后完整持久表逻辑摘要、数据库主文件不变和未展开参数的 SQL trace；没有 DML/DDL。实际搜索与双向邻接 SQL 的 EXPLAIN QUERY PLAN 保存在 search.json/graph.json，确认使用路由和正反向索引。SQLite 自身 WAL/SHM 协调文件不作为业务写入证据。

早期失败如实保留：u64 ToSql 编译错误、测试压力记录复用了 payload hash、测试前缀也命中已有 PurchaseLedgerPage、旧字节读取用例未登记实际证据范围等。已修正实现/测试输入，未放松证据或断言唯一性校验。

本轮仅 Windows x64。普通查询/采集无真实远程模型请求、无业务数据库连接。T06 的 50 个冻结文件及 truthHash 不变。场景组合仍归 T16，知识/native/SDK/MCP 和四平台发布按后续任务验收；此前任务的全工作区/插件报告保留为历史证据，本轮不冒充重跑全部插件。
