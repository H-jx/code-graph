# T21 — stdio MCP

2026-10-29，Windows x64，MCP server/client 锁定 2.4.0。未调用外部模型或业务数据库，保留工作区原有未提交成果。

实现十类查询、14 个显式维护工具、三个静态资源与两个带快照引用的资源模板。query 是默认模式；只有启动 `--mode maintenance` 才登记维护工具。所有业务算法、存储、Schema 校验与插件生命周期都委托已有 SDK/native/Core；不增加 JS SQLite、项目框架解析或伪 Agent 工具。

Standard Schema 从 Core tagged union 投影工具参数；native 新增无项目/无数据库的 Core 请求校验入口。MCP 2.4 高层 handler 会把参数错误转成工具错误，因此通过公共 `tools/call` handler 保留协议 InvalidParams；合法请求的执行失败仍返回统一 Core error 与 isError，partial 保留正常 structuredContent。JSON-RPC ID（含类型）通过 SHA-256 固定长度映射至 SDK ID，取消、断开、stdin EOF 和关闭均归会话 owner 处理。D30/D31 已登记。

复现：

```powershell
pnpm run build:native
pnpm run build:mcp
pnpm run typecheck
$env:PK_MCP_EVIDENCE_DIR = (Resolve-Path docs/evidence/T21).Path
pnpm run test:mcp
```

3 个综合测试、0 failed/ignored：

- 官方 client 分别以 legacy、固定 `2026-07-28` 建立真实 stdio 连接。10 个查询逐项与同库 SDK 比较，保留完整工具请求/响应、Schema hash、分页及正常 partial；核对全部 tools/list Schema、三静态/两模板资源、模式隔离、非法参数不泄露输入。数据库主文件哈希不变，关闭后可在 Windows 改名。
- 无库仍可协商、列工具与读 Schema；查询返回错误，不创建数据库。
- maintenance 的提交/准备/验证/cases/激活/重复激活/collect/增量/回退、主题上下文/导入/Skill 渲染实际成功；非法约定返回 Core 错误。等真实插件写出父与后代 PID 后取消或断开，两个 PID 都消失，取消后连接可继续查询。

`query-resources.json` 保存查询 transcript 与只读断言；`maintenance-cancellation.json` 保存实际维护结果和 PID 清理。当前完整命令、日志与退出码在 [T22/commands.json](../T22/commands.json)，MCP 与 SDK 的业务三场景对照由 T22 单独保存。资源不直接读取任意文件路径，只接受 Core 已校验的 snapshot/reference。

早期失败保留在执行说明中：修复 Node 24 不支持 `node --test test/` 的测试入口；将错误的 search `items` 结构改为已有 `results`；初始 30 文件/默认两秒预算会产生时序相关截断，改用与 SDK 生命周期测试相当的小 fixture，并保留独立预算/partial 用例。没有放宽业务 expected/forbidden 或改冻结真值。

首版平台按用户确认只交付 Windows x64。接口用法见 [mcp-usage.md](../../mcp-usage.md)；API 依据为本地锁定类型及 [MCP 官方 v2 文档](https://ts.sdk.modelcontextprotocol.io/v2/)。
