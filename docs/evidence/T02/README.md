# T02 — 统一类型、稳定身份、Schema 与协议向量

- contract.schema.json：由 crates/core/src/contract 经 schemars 导出的 JSON Schema
  （2020-12），是 SDK / MCP / 插件类型的唯一来源。
- entity-kinds.json / elation-kinds.json：类型注册表（实体身份槽位、关系允许端点）。
- error-codes.json / protocol.json：错误码、缺口码与协议版本信息。
- schema-hash.txt：契约 Schema 的 SHA-256。
- identity-vectors.json：共享测试向量（18 条），覆盖同名跨作用域/跨仓库/跨数据源、
  复合键顺序、Unicode、CRLF、匿名符号与重载碰撞。
- identity-vectors-node-report.json：Node 侧独立实现对全部向量的校验结果（18/18 一致）。
- contract-tests.log：cargo test -p project-knowledge-core --test indexing contract_ 的输出。
- 跨语言一致性：Rust（crates/core/tests/indexing.rs 的
  contract_identity_vectors_match_shared_fixture）与 Node（scripts/verify-vectors.mjs）
  对同一份向量文件给出相同 ID。
