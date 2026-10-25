# T00 — 依赖与工具链核对

- 工具版本与锁定依赖：见 	ools.txt。
- 已实测通过：
  - Rust 1.97.1（ust-toolchain.toml 固定 1.97.1，与 rig 0.45.0 的 MSRV 一致）。
  - ig = "=0.45.0" 实际编译通过（crates/agent 依赖该 crate 并随 workspace 一起构建）。
  - SQLite：FTS5 / JSON1 / 外键 / WAL / synchronous=FULL / Online Backup API 均实测通过
    （见 ../T04/storage-tests.log 与 ../T02/contract-tests.log）。
  - Tree-sitter：TS / TSX / JS / Python 四类 grammar 均能加载并解析（基础索引用例覆盖）。
- 未完成（阻塞项）：
  - napi v3 实际加载最小例。
  - MCP 2.4.0 stdio initialize / 	ools/list / 	ools/call 最小例。
  - Rig 工具循环、max_turns、结构化错误与取消的最小例。
  - Python 3.12：本机 python 为 WindowsApps 占位，无法执行任何 Python 侧验证。
