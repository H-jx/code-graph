# T00 — 依赖与工具链核对

2026-10-26：done。历史 tools.txt 保留；当前以 probe-report.json 的真实命令、退出码及输入/锁文件哈希为准。

- Rust 1.97.1；napi 3.15.0 / derive 3.6.13 / build 2.6.0；Node 24.14.0 实际加载原生库，直接调用产品 Core 校验器并核对 Schema/JSON Pointer。
- MCP server/client 2.4.0：真实 stdio initialize → tools/list → tools/call，legacy 2025-11-25 与 pin 2026-07-28 各通过；合法与非法调用均验证。
- Rig 0.45.0：官方 MockCompletionModel 驱动两轮工具调用、结构化工具错误、max_turns、hook 取消，4 项通过；显式 key/base URL/provider 构造通过，真实模型请求为 0。
- TypeScript 6.0.3（独立 package-lock.json，真实 npm ci）：Compiler API 中文/emoji UTF-16 → UTF-8 转换通过。根工具链 TypeScript 5.9.3 保留。
- Python 3.12.13 可由 uv 解析，AST 字节定位、真实 Python 插件模板、独立身份向量通过。原“无 Python 3.12”阻塞已解除。
- SQLite 与四类 Tree-sitter grammar 沿用 T04/T05 的有效实测。

入口：`node scripts/verify-integration-probes.mjs`。CI 先安装 probe/node 中的锁文件依赖；Python 可由 PROJECT_KNOWLEDGE_PYTHON、PATH 上的 3.12 或 uv 解析。

证据：native-build.log、rig-tests.log（4 passed）、node-tests.log（5 passed）、npm-ci.log、python-vectors.json、probe-report.json。精确依赖/MSRV/license 见 dependency-matrix.json；独立 probe workspace 与锁文件在 probe/。

未实测真实 provider 网络、多平台发布或远程 Actions。缺失的历史架构审查不伪造。
