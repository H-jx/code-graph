# 契约 JSON Schema

由 `cargo run -p xtask -- export-contracts` 从 `crates/core/src/contract/` 导出（JSON Schema 2020-12），提交入库。

用途：

1. `scripts/generate-types.mjs` 据此生成 `packages/sdk/src/generated/*.ts`；
2. Core 从同一 Rust 契约构建 Schema，用于校验提取器输出与候选 manifest；
3. MCP 的 schema 资源通过 Core 能力接口提供协议；
4. `pnpm run check:contracts` 检查导出 Schema、生成类型和类型边界是否一致。

生成文件：

```text
docs/schema/
├── contract.schema.json       # 查询、采集、配置和知识工件的统一 Schema
├── protocol.json             # 协议版本
├── entity-kinds.json         # 实体类型
├── relation-kinds.json       # 关系类型
└── error-codes.json          # 错误码
```

约定：不加载远程 `$ref`；只允许本目录内已登记的 Schema 相互引用。

跨语言稳定 ID 测试向量在 [fixtures/protocol](../../fixtures/protocol/)。
