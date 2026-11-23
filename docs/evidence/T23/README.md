# T23 — Windows 候选构建与安装验收

> **2026-11-11 更新**：2026-10-30 暂缓的 consumer `0xC0000409` 异常已定位并修复——根因是
> Node v24.14.0 / libuv 在 Windows 上 `fs.cpSync` 对非 ASCII 路径的内存损坏缺陷（验收脚本
> 缺 native 检查的文件复制），**非产品缺陷**；产品在中文空格路径下全部通过。修复以等价手动
> 递归复制替换 `cpSync`，不削弱验收。完整隔离安装验收（含离线重装）已通过。详见
> [continuation-2026-11-11/](continuation-2026-11-11/)（根因、复现、最终候选与验收报告）。
> 下方 2026-10-30 记录保留为历史。

## 历史记录（2026-10-30）

2026-10-30：用户要求先忽略安装，停止推进本项，不将 T23 标为完成，不继续重现或修复安装验收异常。

已产生的候选为 `target/release-candidate/0.1.0/x86_64-pc-windows-msvc/candidate-qfYMrM/`：release 默认 CLI、Core-only CLI、Windows x64 native 平台包、native root、SDK、MCP。包含 SHA256SUMS、release-manifest.json、source-manifest.json、锁文件、依赖清单与实际命令日志，未公开发布。

已有成功记录：CLI 初始化/基础索引/状态/查询/证据读取、关闭 Agent feature 的拒绝行为、运行 PATH 排除构建工具、独立 npm tarball 安装与 `npm ci --offline`。SDK/native 的 capabilities/open/status/close 单独调用亦成功。

未完成的门禁：完整 `sdk-mcp-consumer` 进程退出码为 3221226505（0xC0000409），stderr/stdout 日志为空，未产生通过的 installation-report.json 或 consumer-report.json。临时配置已有本地 Agent 检查设置，但这不足以证明整个流程或退出清理通过。异常发生的准确阶段及根因尚未确认，不将它说成已修复，也不认定它只属于安装器或必然属于 native。

原始记录在候选目录的 `install-commands.json`、`install-sdk-mcp-consumer.log`，构建记录在 `commands.json`。首轮候选 `candidate-KHLp8m` 因上游 MCP 包未导出 package.json 子路径而打包失败；版本读取改从已安装入口查找元数据后，qfYMrM 候选构建成功。失败记录保留。

功能回归另见 [T22](../T22/README.md)：Rust 159、SDK 7、MCP 3，以及 14 组三端同库对照。通过功能回归不替代本项的交付运行验收。
