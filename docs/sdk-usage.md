# TypeScript SDK

首版按 D31 交付 Windows x64 可安装候选；安装见 [installation.md](installation.md)。Linux/macOS 后续。仓库开发使用 `pnpm run build:native`、`pnpm run build:sdk`，Node 24。native 缺失或不匹配时返回 `RUNTIME_MISSING` 和安装提示，不在使用时编译或下载。

```ts
import { ProjectKnowledge } from "@project-knowledge/sdk";

const capabilities = await ProjectKnowledge.getCapabilities(); // 无需配置或索引
const configPath = "C:/workspace/my-repo/.project-knowledge/project.json";
const client = await ProjectKnowledge.open({ configPath });
try {
  const result = await client.search({ query: "采购", pageSize: 20 });
  // result.snapshot / sourceReferences / completeness / gaps 由 Core 原样返回。
  if (result.continuation) {
    await client.search({ query: "采购", pageSize: 20, continuation: result.continuation });
  }
} finally {
  await client.close();
}
```

查询方法为 `status`、`search`、`locateFeature`、`trace`、`neighbors`、`getCodeMap`、`readSource`、`locatePage`、`explainWorkflow`、`explainReport`，也可传完整 `QueryRequest` 给 `execute`。参数类型从 Rust Schema 生成；无命中、歧义、覆盖不足、过期和预算截断都保留 Core 的语义，调用方须检查 `status` 与 `completeness`。只读打开不会创建库、迁移、发现或运行插件。

```ts
const configPath = "C:/workspace/my-repo/.project-knowledge/project.json";
const pluginManifest = "C:/workspace/my-collector/manifest.json";
const maintenance = await ProjectKnowledge.openMaintenance({ configPath });
try {
  await maintenance.index({ mode: "base" }); // 显式创建/迁移并建立基础索引
  const candidate = await maintenance.submitPlugin({ path: pluginManifest });
  if (!candidate || typeof candidate !== "object" || !("id" in candidate) || typeof candidate.id !== "string") throw new Error("Missing candidate ID");
  const candidateId = candidate.id;
  await maintenance.preparePlugin({ candidateId }); // 显式准备锁定依赖，允许联网
  await maintenance.validatePlugin({ candidateIds: [candidateId] });
  await maintenance.activatePlugin({ candidateId });
  await maintenance.index({ mode: "incremental" }); // 无模型，运行已激活制品
} finally {
  await maintenance.close();
}
```

`maintain` 接受同源 `MaintenanceRequest`，其便捷方法还包括 `discoverPlugins`、`rollbackPlugin`、`collect`、`exportContext`、`importTopic`、`importConventions`、`generateSkill`。`discoverPlugins` 只登记候选，后续准备、验证、激活必须显式调用；`collect` 运行全部活动阶段。知识工具只导入外部生成内容和调用 Core 渲染，不包含 Rig 或模型客户端。维护结果沿用各 Core 入口的 JSON，除 `exportContext` 的既有 DTO 外暂为 `unknown`；调用方须按实际结果检查，不能以 Promise 成功推断覆盖完整。

所有请求可传第二个参数 `{signal, requestId}`；省略 ID 时 SDK 生成 UUID。已触发的 signal 立即以 `CANCELLED` 拒绝，运行中取消传递到 Core 检查点、SQLite VM 和 owned 插件进程树；同一客户端在途 ID 不可重复。响应 `requestId` 与注册 ID 一致，所有响应字符预算仍在 Core 计算。`cancel(requestId)` 可显式取消。

`close()` 立即拒绝新请求、取消排队及运行任务并等待 owner 释放连接和子进程；重复调用共享同一个 Promise。清理等待上限为 10 秒，异常超时返回可重试 `BUSY`，客户端仍保持关闭并继续清理；文件系统或 OS 调用若不返回，宿主无法保证强制打断。支持 `await using` / `Symbol.asyncDispose`。每个 native 客户端有一个连接 owner 线程和最多 64 个排队请求，超量返回 `BUSY`；Promise 等待不会占用 Node 的 libuv 线程池。

错误均为 `ProjectKnowledgeError`，保留 `code`、`retryable`、`issues`、`runId`、`phase`，不回显非法输入内容；native panic 映射为 `INTERNAL_ERROR`。native JSON 通道同样接受 Core Schema 校验，TypeScript 类型不会替代运行时验证。没有额外 SQLite driver、项目框架解析规则或专属业务映射。
