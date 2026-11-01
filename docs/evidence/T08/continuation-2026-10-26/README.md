# T08 续实施证据

工作区：`E:/Project/my-project/code-graph`，用户指定从已有 T08 成果继续。
既有验证和事务 B 未重写；补齐暂存编排、选择集合、自动依赖、case 与 CLI。

## 本次覆盖

- backup SQLite → 清除本组旧指针 → 稳定 DAG extract → 再导出实体与完整候选 → link → case → 短事务提交。
- 自动 config/artifact/runtime/source/sourceSet/candidateSet/upstreamBatch 依赖；插件不能覆盖或伪造宿主摘要。
- sourceSet 跨仓库合并、绑定内容与选择状态；candidateSet 包含完整断言，支持零匹配和同 ID 属性变化。
- 输出必需数组、关系/证据 ID、UTF-8 边界、生成来源拒绝、scope 一致性、已定义的业务属性 Schema。
- acceptance case 重放 inputFixture，比对 required/forbidden/gapCodes；case/fixture/truth 文件参加制品哈希。
- validate-only 不改正式库；坏 link/case 不提交 extract；空 extract 替换并失效下游；其他生产者保留。
- Windows canonicalize 的 verbatim 路径在传入 Node 前转换为其可接受的绝对路径。

## 命令与结果

| 日志 | 实际命令 | 退出码 | 结果 |
| --- | --- | --- | --- |
| `workspace-tests.log` | `cargo test --workspace --locked` | 0 | 87 passed：CLI 3、collectors 29、indexing 44、lifecycle 11；0 failed/ignored |
| `clippy.log` | `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 | 无 lint 告警 |
| `fmt.log` | `cargo fmt --all -- --check` | 0 | 无差异 |
| `core-only.log` | `cargo build -p project-knowledge-cli --no-default-features --locked` | 0 | Core-only 构建通过 |
| `contracts.log` | `cargo run -p xtask --locked -- export-contracts --check` | 0 | 5 个同源契约文件一致 |
| `types.log` | `node scripts/generate-types.mjs --check` | 0 | TS 类型一致 |
| `typecheck.log` | `pnpm run typecheck` | 0 | SDK/MCP 编译通过 |
| `vectors.json` | `node scripts/verify-vectors.mjs` | 0 | 18/18 Rust 对照向量的 Node 独立实现一致 |
| `plan.log` | `node scripts/verify-plan.mjs --plan ../plan/NEW-PROJECT-LONG-TASK.md` | 0 | 24 项状态/DAG/需求映射完整 |

`validation.log` 是较早一次本轮定向验证；最终结果以 workspace-tests.log 为准。
测试数据及实现哈希在 `source-hashes.json`；工具版本与命令清单在 `verification.json`。
更多真实 CLI 响应见 [T09 证据](../../T09/README.md)。

## 边界

本轮闭环使用无依赖 Node 24 合成插件；不代表 Python、带依赖缓存或独立业务真值通过。
不支持的扩展 kind 被拒绝；扩展 Schema 正向登记仍依赖 T02。
T07 仍未 done，因此计划 T08 不虚标 done；此前未完成事项继续保留。
