# T09 候选与激活证据

T09 已完成当前任务的候选、原子激活与中断恢复验收（2026-10-27 复核）。

实现入口：`crates/core/src/collect/registry.rs`、`crates/core/src/storage/plugins.rs`、
`crates/core/migrations/0002_candidates.sql`、`crates/cli/src/commands/plugin.rs`。
使用方法与边界见 `docs/collectors.md`。

## 真实执行

- 全工作区 87 项测试全部通过，日志见 T08 续实施的 `workspace-tests.log`。
- lifecycle 的 11 项测试覆盖提交/验证/发布、同代候选 CAS、WAL 旧读快照、五处故障注入、
  实际发布 I/O 失败、不可变版本、回退新批次、源码改回不复活、活动阶段闭包、
  autoLoad/autoActivate，以及两个新候选整组成功或整组失败。
- `host-termination.log`：真实 Windows CLI 验证进程被终止，断言 Node 插件及孙进程退出，
  文件锁释放，正式 generation 不变，只读检查不恢复，维护重启将 prepared 标为 failed。
  暂存目录有独立 OS 锁与标记；同一 Windows 综合用例也断言活动 validate-only 的目录不被并发维护删除，宿主终止后能够安全回收。
  该文件为较早定向运行记录；最终两种宿主模式及目录断言以最新 workspace-tests.log 的同名用例和源码哈希为准。
- `cli/run-*/report.json`：`node scripts/verify-collection.mjs` 的真实 8 步 CLI 合成闭环；
  每步保留 argv、退出码、JSON 响应与 SHA-256，含基础索引、validate-only、submit、validate、activate、collect、坏 link 和 rollback。
  最后一次运行目录由 `latest-cli.json` 指向。
- v1→v2 只在维护入口迁移；保留 indexId/generation，旧 Schema 只读打开会明确拒绝。

## 续实施复核

- `rechecked-lifecycle.log`：Windows 13 passed；`linux-probe/rechecked-lifecycle.log`：Linux 14 passed，1 ignored（该用例有单独真实执行日志）。
- `activation_real_host_kill_inside_c_preserves_old_snapshot_and_retries` 在 C 写入第一个与第二个 batch 后真实终止本次 owned 子宿主，检查 WAL 读者只见旧整组、generation/搜索/活动映射不变、integrity/外键、维护恢复与重新激活。
- SQLite `max_page_count` 产生引擎真实 FULL，不能称为物理磁盘满；`linux-probe/full-filesystem-capacity.log` 与 `full-filesystem.log` 是独立 mount namespace 的 4 MiB tmpfs 100% 满额、发布真实 ENOSPC 与恢复 1/1 通过。
- `linux-probe/recheck.sh` 可重放上述 Linux 验证；完整源码与日志哈希、命令退出码见 `recheck-report.json`。首次手工 WSL 命令 PATH 转义失败（cargo not found），改用脚本后通过。
- T07 的 Python/npm/pip 接线已完成；`linux-probe/dependencies/report.json` 也记录 27 步真实 Linux CLI 依赖闭环。
- 修复了 Linux fork 短暂继承锁描述符导致锁释放延后的问题；维护锁显式 unlock，有独立 Linux 回归用例。

物理断电和 macOS/ARM64 尚未实测，不能由进程强杀或可控错误代替；U03/T23 继续约束平台发布。T06 独立业务真值仍 blocked，本任务仅证明合成采集生命周期，不声明业务准确率。T09 闭环后继续 T10。
