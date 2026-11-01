# T01 — 工作区、CLI 与统一验证入口

2026-10-26：done。复核并修正已有 `.github/workflows/ci.yml`，未创建重复 workflow。

- rust-checks.json 与 fmt/clippy/check/tests/core-only.log：本地门禁全部 exit 0，完成时 87 项测试通过。T02 后续回归日志独立存放，历史记录不覆盖。
- core-dependencies.log：Core 正常依赖 406 节点，无 Rig/Agent；core-deps-negative.log：缺 Cargo 明确失败，避免管道吞掉错误。
- pnpm-install/typecheck/build-ts/contracts/plan/probes.log：锁定安装、TS 编译、同源比较、计划检查与真实 native/MCP/Rig/Python 探针通过。
- CI Rust matrix 为 Windows/Linux/macOS；Node job 配置 Node 24.14.0、Python 3.12.10、Rust 1.97.1。官方 actions/python-versions manifest 确认该 Python patch 的三平台制品；本地为 3.12.13。
- Rust toolchain action 固定完整 commit。产品 binding 尚为空时，以直接调用真实 Core 的 native 探针验收。

cli-help.txt / cli-version.txt 保留开发 CLI 输出。未触发远程 Actions，不把配置存在或本地通过表述为远程 CI 成功。
