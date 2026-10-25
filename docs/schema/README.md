# 契约 JSON Schema

由 `cargo run -p xtask -- export-contracts` 从 `crates/core/src/contract/` 导出（JSON Schema 2020-12），提交入库。

用途：

1. `scripts/generate-types.mjs` 据此生成 `packages/sdk/src/generated/*.ts`；
2. `crates/core/src/collect/validate.rs` 用 `jsonschema` 校验提取器输出与候选 manifest；
3. MCP 的 schema 资源读取同一份内容，保证查询方与实现方看到的协议一致；
4. 评审协议变化时直接 diff 本目录（`pnpm run check:contracts` 会在临时目录重生成并比较）。

预期文件（实现后由 T02 生成）：

```text
docs/schema/
├── protocol.json               # 版本、枚举、错误 envelope
├── entity.json
├── relation.json
├── evidence.json
├── collector-manifest.json     # 插件 manifest
├── collector-io.json           # CollectorInput / CollectorOutput
├── query.json                  # QueryRequest / QueryResponse
└── identity-vectors.json       # 稳定 ID 的跨语言测试向量
```

约定：不加载远程 `$ref`；只允许本目录内已登记的 Schema 相互引用。

> 骨架阶段本目录为空（仅 `.gitkeep`），因为 `xtask export-contracts` 尚未实现。
