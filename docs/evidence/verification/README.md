# 全量验证记录

本目录保存本轮实施结束时的统一质量门禁输出，全部为**实际执行**的命令结果。

| 文件 | 命令 | 退出码 |
| --- | --- | --- |
| `01-fmt.log` | `cargo fmt --all -- --check` | 0 |
| `02-clippy.log` | `cargo clippy --workspace --all-targets -- -D warnings` | 0 |
| `03-test.log` | `cargo test --workspace --locked` | 0 |
| `04-vectors.json` | `node scripts/verify-vectors.mjs` | 0 |
| `05-verify-docs.log` | `node scripts/verify-plan.mjs --plan ../plan/NEW-PROJECT-LONG-TASK.md` | 0 |
| `06-contracts.log` | `cargo run -p xtask -- export-contracts --check` | 0 |
| `07-types.log` | `node scripts/generate-types.mjs --check` | 0 |
| `08-typecheck.log` | `pnpm run typecheck` | 0 |

测试数量：`collectors` 21 条 + `indexing` 43 条 = 64 条，全部通过，0 失败。

额外验证：`cargo build -p project-knowledge-cli --no-default-features --locked` 通过，
且 `cargo tree -p project-knowledge-core` 中没有 `rig`（Core 不依赖 Agent 栈）。
