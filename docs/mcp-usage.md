# stdio MCP

Node 24，先安装同版本 native 平台包、native、SDK 和 MCP。首版只交付 Windows x64 候选；包安装见 [installation.md](installation.md)。

```json
{
  "mcpServers": {
    "project-knowledge": {
      "command": "node",
      "args": ["C:/tools/project-knowledge/node_modules/@project-knowledge/mcp/dist/index.js", "--config", "C:/workspace/.project-knowledge/project.json"]
    }
  }
}
```

默认 query 模式提供 `index_status`、`search`、`locate_feature`、`trace_links`、`neighbors`、`get_code_map`、`read_source`、`locate_page`、`explain_workflow`、`explain_report`。字段与 SDK 一致，工具名选择 operation；工具参数不能改变模式。查询不会创建或升级索引、发现或执行插件、调用模型。

工具的 `structuredContent` 保留 Core 查询信封。`status=partial`、gaps、snapshot、分页和完整性必须一起读取；partial 不是执行错误。合法参数的 Core 执行失败返回 `isError=true` 与结构化 error；非法参数返回 MCP InvalidParams，错误不回显输入值。

维护连接须在启动参数显式增加 `--mode maintenance`。它额外提供候选提交、发现、准备、验证/cases、激活、回退、collect、基础/增量更新、主题上下文导出、主题/约定导入与 Skill 渲染；`tools/list` 公告每个工具的 Core 同源 Schema。`export_snapshot` 和 `export_topic` 都导出有收据的主题上下文；`validate_plugin` 和 `run_plugin_cases` 都使用完整 Core 候选验证。维护结果保留既有 Core JSON，不提供未运行的 Rust Agent 伪工具。

资源：

- `project-knowledge://schema`：完整 capabilities / Core Schema，无索引可读。
- `project-knowledge://index`、`project-knowledge://skill-manifest`：Core status 信封，包含覆盖、主题/导出 manifest 及新鲜度核验。
- `project-knowledge://facts/{reference}`、`project-knowledge://evidence/{reference}`：`reference` 是 EntityRef / SourceReference JSON 的 UTF-8 base64url，必须保留 indexId/generation。Core 拒绝跨库、过期或伪造引用，读取资源不写库。

取消通知传入 SDK AbortSignal 与 Core 请求令牌；stdin 关闭、客户端断开或 SIGINT/SIGTERM 均关闭会话和 native owner，等待 owned 插件子进程清理。stdout 只写协议，诊断写 stderr。单连接原生队列与关闭上限见 [sdk-usage.md](sdk-usage.md)。

实现依据锁定 MCP 2.4.0 的本地类型及[官方 v2 文档](https://ts.sdk.modelcontextprotocol.io/v2/)。验收使用官方 client 的 legacy 与固定 `2026-07-28` 两种协商；详见 [T21](evidence/T21/README.md)。
