# T20 — Node native 与 TypeScript SDK

2026-10-29，Windows x64。本轮保留已有未提交工作，不提交 Git，不调用模型或连接业务数据库。T15/T17/T02 均已完成。

native 使用一个独立 owner 线程创建、使用并释放 Core SQLite 连接；查询始终只读，维护必须显式打开。请求 ID 在返回 Promise 前登记，与 Core 查询响应 ID 一致，取消传到查询/SQLite VM、基础扫描及事务提交和 owned 插件进程树。AsyncBlock 异步等待响应，不占 libuv 线程池；最多 64 个排队请求，重复 ID/超量拒绝。close 幂等、立即拒绝新任务，取消并等待已有工作与资源释放；10 秒异常超时返回 BUSY 且继续清理，不承诺强制打断不返回的 OS 文件调用。相对配置路径在打开时固定。

SDK 提供十类查询、同源 MaintenanceRequest 的显式维护便捷方法及统一错误；保留 snapshot/ref、partial/gaps/coverage 和分页。维护入口不包含 Agent/Rig，不引入 JS SQLite driver。Core Schema 由 xtask 和 capabilities 共用，同源导出新的维护与能力类型；见计划 D29 和 [SDK 用法](../../sdk-usage.md)。构建包装器显式传当前 Rust target 与 --locked，避免 metadata 拉取无关平台制品。

验收入口：

```powershell
python docs/evidence/T20/verify.py
python docs/evidence/T20/verify.py --group finalize
```

`commands.json` 保存实际命令、退出码、耗时和完整日志哈希；重复执行保留带编号的日志。`verification-report.json` 仅在所有必需门禁及产物断言通过后生成。

最终通过 Core 131（business 18、client 3、collectors 36、indexing 57、lifecycle 17）、native 1、SDK 7，共 139 个测试，0 failed/ignored。Core 整组回归本轮已完成，不等于全工作区回归；T19 全工作区超时记录未更改。native/SDK/CLI 的最终构建、Clippy 全目标 -D warnings、fmt、契约和类型边界/typecheck、依赖隔离、50 份冻结来源及独立整数 oracle、计划/源码/diff 均 exit 0。192 份源码零字节 0。

- `cli-sdk-golden.json`：同库十类 CLI/SDK 请求逐项比较，保留两端完整响应。规范化仅移除 requestId、checkedAt、elapsedMs 和随这些元数据长度变化的 responseChars；实体/断言/关系/原始引用/缺口/分页/完整性均比较。另用 128 字节请求 ID、Unicode 原文和 2400 字符预算核对 native 原始 JSON 的实际 Unicode scalar 计数。
- `event-loop.json`：40 个查询期间计时器仍响应、pre-abort/在途取消/同一 ID/重复 close/关闭后拒绝、查询后 Windows SQLite 文件改名。
- `plugin-lifecycle.json`：SDK submit→prepare→validate→activate→重复 activate→collect；等真实插件写入父/后代 PID 标记后 abort 或 close，确认两 PID 均消失。排队取消、重复 ID、队列满及排队期间异步 fs 仍响应也在同一综合测试中检查。
- `readonly-sql.json`：Core 实际 SQL trace 无写表/建表语句、单读事务和数据库字节不变；在 SQLite 查询语句开始后触发取消，返回 CANCELLED 并退出事务。另验证基础写事务和激活事务中取消均保留原 generation/活动版本。
- `source-audit.json`：源码清单、零字节与未实现项。MCP/T23 和范围外 SCIP 仍分别登记，未通过空导出充当实现。

早期检查中两处 SDK 测试假定了错误的 schema 版本存储/搜索结果结构、一处 SQL trace 测试未考虑语句前空格，均修正测试后重验；未修改业务预期使错误结果通过。最初 napi metadata 的跨平台下载被停止，改用显式 host target，未改外部锁定版本。T19 的全工作区超时记录保持原样，本轮验证范围以当前命令日志为准。

证据汇总器的首次依赖断言曾把 Rust 的 `libsqlite3-sys` 子串误判为 JS `libsql`；已改为按完整包名核验。该次汇总没有生成通过报告，实际依赖树始终是 Core 的 rusqlite，无 JS SQLite driver 或 Rig。

本轮只验 Windows x64。三业务场景完整 CLI/SDK/MCP 对照继续由 T22 负责，四平台干净安装与发布继续由 T23 负责；MCP 为下一项 T21。

实现所依据的版本 API 已核对本地锁定 napi 3.15.0 源码；官方参考为 [async functions](https://github.com/napi-rs/website/blob/main/pages/en/docs/concepts/async-fn.md) 和 [NAPI 属性](https://napi.rs/docs/concepts/napi-attributes)。未升级 API 或依赖版本。
