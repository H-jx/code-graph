# T22 — Windows x64 最终回归与三端业务验收

2026-10-29。用户明确确认首版收尾范围为 Windows x64 可安装候选，其他平台后续。保留旧成果与所有失败日志，不提交 Git、不调用新的外部模型或业务数据库。

复现：

```powershell
python scripts/verify-final.py
node scripts/verify-acceptance.mjs
```

验证器各组可单独执行（`--group build|tests|plugins|node|static|scenarios|finalize`）；不同组使用独立命令 journal，汇总保留每次尝试。`verification-report.json` 只在全部必需命令 exit 0、日志哈希和源码清单一致、MCP 记录与实际三端对照都通过后产生。`acceptance-report.json` 根据当前 workspace 日志核对 21 个 S 条目与 16 个 X 条目的实际测试名，不用计划状态代替运行证据。

本轮工作区 Rust 159 passed（Agent 21、CLI 6、Core 131、native 1），SDK 7、MCP 3；重复插件入口和独立场景重跑不加到独立测试总数。旧 T19 的 900 秒超时保留不变；本轮 workspace 命令在 1142.611 秒完成，退出 0，不以旧记录代替当前回归。仅验本 Windows x64 主机。

三端驱动在 Rust fixture 仍存活时，对同一库、同一快照与同一请求同步调用 CLI/SDK/MCP，保存完整响应。只去除 requestId/checkedAt/elapsedMs/responseChars；实体、关系、源码引用、缺口、partial 与分页字段全部比较。每次场景运行写独立 run 目录，最终报告只读取本轮成功命令绑定的目录，不累加旧尝试。

真值来源分开记录：50 份冻结来源及整数 oracle 原 hash 不变；A/B 源码审阅的 expected/forbidden 属合成场景；真实 synes 11 文件、12 条声明事实由 `source-review.json` 直接回查，当前来源逐文件 hash 11/11 相同。没有拿模型输出建立 expected，也没有人工签认、第二真实仓库或生产业务准确率声明。金额比较只限冻结本地审计口径。

早期失败和修复：Node 测试入口/搜索结构和两秒小预算在 T21 记录；首次 Clippy 检出 xtask 删除占位后遗留的 unused import，删除后重验；插件总入口的逗号参数被 PowerShell 当数组重写，修复命令逐参数引用；错误输出打印又暴露 Python 控制台 GBK，验证器现显式 UTF-8。失败日志保持，不作为通过统计，未修改业务期望或冻结真值。

可安装 release 包、隔离 consumer、离线重装和平台包选择由 [T23](../T23/README.md) 保存；MCP 同源 Schema、模式隔离与真实进程取消由 [T21](../T21/README.md) 保存。物理断电、最低 Windows 版本、Linux/macOS、原始作者签认和未恢复历史资料仍按 [验收边界](../../validation/acceptance.md) 保留。
