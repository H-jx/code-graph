# T13 跨插件与跨仓库关联

前置 T12 已满足。四制品 frontend/api/database/link 经 Core 登记、整组验证、原子激活；所有关联引用已有实体，不在 SDK/Core 放置项目规则。源码/制品/真值/命令/退出码/日志 hash 在 verification-report.json，完整复现命令为 `python docs/evidence/T13/verify.py`。

已验证 A/B 两布局 HTTP client → 显式 service → 实际注册，以及 GraphQL 字段 → 实际 resolver；query → dataSource 来自带作用域的表列 reads。每条边复用其原始调用、配置或注册证据。缺绑定、多 resolver、缺调用证据均不产生 confirmed 关联；人工声明保留 declared/human；不完整 selector 返回 CANDIDATE_SET_INCOMPLETE。

四组增量/干净重建比较在 normalized-{a,b}-{ambiguous,proxy-datasource}-{incremental,clean}.json，comparison-report.json 保存逐份 hash 与相等断言。覆盖新增 resolver 后旧确认关系变为候选、消除歧义、代理变化、切换 dataSource 与同名表隔离。比较保留全部实体/关系/覆盖记录和重复数量，只规范化运行/批次标识及无语义的记录顺序。

| 验证 | 结果 |
| --- | --- |
| cargo test --workspace --locked -j 1 -- --test-threads=1 | 131 passed，0 failed，0 ignored |
| cargo clippy --workspace --all-targets --locked -j 1 -- -D warnings | exit 0 |
| cargo fmt --all -- --check | exit 0 |
| Rust 契约 5 文件 / TS 生成 / TS 正负边界 | exit 0 |
| pnpm run typecheck | exit 0 |
| 50 冻结文件 / Python 独立整数 oracle / 24 项计划 | exit 0 |

新增 CollectorInput.evidence 为可省略数组，仅导出本次所选来源版本中的证据。candidateSets 同时绑定相关关系断言，不能沿用旧 resolver 唯一性；见 D23。三端 DTO 仍从 Rust 生成，没有平行协议。

原始失败完整保留：contracts-export-failed.log（rusqlite 的 u64 读取）、linking-test-query-failed.log（测试取错 producer_key 所在表）、linking-coverage-order-failed.log（相同记录的覆盖排序）。左右完整结果保存在 normalization-{left,right}-failed.json；确认实体/关系相同、覆盖记录多重集合相同后采用稳定排序，未放宽任何内容断言。

本轮 Windows x64，无产品模型请求、无业务数据库连接。T06 truthHash 保持不变，独立补充原文审阅见 fixtures/truth-supplements/T13-link.json。结果限于支持的合成样例，不声称生产准确率或其他平台已验收。下一任务 T14：流程/台账业务语义。
