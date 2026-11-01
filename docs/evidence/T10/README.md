# T10 无模型增量闭环

2026-10-27：done。依赖 T09 已复核闭环，保留既有扫描、依赖、staging、发布与事务实现。

## 实现

- `index/incremental.rs` 按完整来源集合、候选断言集合、配置、运行时及依赖缓存比较输入；依赖反向闭包只含当前批次，另补显式 DAG 下游。失效 A 在执行前持久提交，业务结果在暂存库逐阶段执行后整组提交。
- `index/build.rs` 按仓库输入摘要复用未变基础批次；无关仓库变化不再更换该仓库断言。被替换/移除基础批次也保留单向失效历史。
- 阶段输入移除自身与显式下游旧指针，避免自引用旧结果；新增/零匹配/歧义/属性变化均比较完整集合。分阶段 case 只重放本阶段 case。
- CLI index 接入异步 Core 与取消令牌；`executed` 返回实际运行阶段。collect 运行前持久登记输入变化，过期来源明确要求更新索引。`--base` 保留不依赖插件运行时的基础索引能力。
- 单项目维护锁、最终输入复核、generation CAS；失败/取消/宿主中断保留 A，恢复只改诊断，旧 batch 从不直接恢复 fresh。

## 验证

| 命令/日志 | 结果 |
| --- | --- |
| `cargo test --workspace --locked -j 1 -- --test-threads=1` / workspace-verified.log | exit 0；103 passed（CLI 5、collectors 36、indexing 45、lifecycle 17），无 ignored |
| `cargo clippy --workspace --all-targets --locked -j 1 -- -D warnings` / clippy-serial.log | exit 0，无 lint 告警 |
| `cargo fmt --all -- --check` / fmt.log | exit 0，无差异 |
| Core-only CLI build / core-only.log | exit 0 |
| Rust export-contracts --check / contracts.log | exit 0，5 文件一致 |
| generate-types --check / types.log | exit 0，同源 TS 一致 |
| verify-core-dependencies / core-dependencies.log | exit 0，406 节点，无 Agent/Rig |
| Linux Core / linux-core.log | exit 0，99 passed，1 ignored（满文件系统专用用例已在 T09 独立执行） |
| Linux Core-only CLI / linux-cli.log | exit 0，3 passed |
| Linux 最终源码 incremental_ / linux-incremental-final.log | exit 0，4 passed；源码摘要与 verification-report.json 一致 |
| verify-plan / plan.log | 任务状态、DAG 与映射检查；见最终报告 |

`comparisons/comparison-*.json` 保存五次变更的 incremental/clean 完整规范化结果、失效闭包、执行阶段、相等断言和模型调用边界。
覆盖候选 0→1→2→1→0、无原关系引用的新候选、改名、文件移动、同身份属性变化、成功空输出与删除；比较实体、关系、来源选择、证据、搜索投影和覆盖。
另有无变化不重跑/不增代、未变仓库 batchId 不变、ignore 变化、制品篡改、运行时缺失、失败后内容还原、失败回退、取消、base-only、分阶段 collect 与真实 Windows 增量 CLI 宿主强杀/两级 owned 后代清理/恢复 interrupted。

所有 fixture 均为合成生命周期 smoke。模型请求数为 0：Core 不构造模型客户端，依赖检查也确认没有 Agent/Rig；这不是独立业务准确率或真实 provider 验证。

## 失败与证据边界

- `workspace-final.log` 与 `clippy.log` 保留并行跨平台验证时 Windows OS 1455（提交内存不足）失败，未降低断言；释放本任务 WSL 干净缓存文件页、限制为串行后，workspace-verified.log / clippy-serial.log 全部通过。
- Linux 旧 output-flood fixture 的 process.exit 有机会截断管道，实际收到 SCHEMA_INVALID 而非 OUTPUT_LIMIT。摘录在 linux-output-flood-initial-failure.txt；脚本改为等待 drain，原输出超限断言保留，后续 Windows/Linux collectors 均通过。
- WSL 独立 Clippy 初次安装产生零字节 cargo-clippy/clippy-driver；空程序退出 0 不能证明 lint 运行。linux-gates-result.json 已标 accepted=false，修复安装仍有遗留文件冲突，不计入通过。Windows 实际全工作区 Clippy 是本任务有效 lint 证据。linux-gates.sh 已加工具非空和版本输出检查，防止重放时报绿。
- 真实 npm/pip 安装与物理满文件系统沿用 T07/T09 有效证据；本轮未重复联网安装、未做物理断电或 macOS/ARM64 发布验收。
- 下一项 T11 仍依赖 blocked 的 T06。truth/三个业务目录仅有 .gitkeep，没有独立资料、审阅记录与 truthHash，不能由本轮 smoke 替代。

最终命令、退出码、哈希和边界见 verification-report.json；执行记录见 ../../execution-log.md。
