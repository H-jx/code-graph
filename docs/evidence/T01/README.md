# T01 — 工作区、CLI 与统一验证入口

- cli-help.txt / cli-version.txt：开发版 CLI 的实际输出。
- 真实存在的调度入口：
  - pnpm run build:contracts → cargo run -p xtask -- export-contracts + 
ode scripts/generate-types.mjs
  - pnpm run check:contracts → 同样的两个命令加 --check（在内存/临时位置比较，不写盘）
  - pnpm run verify:docs → 
ode scripts/verify-plan.mjs
- 未完成：新项目独立 CI workflow；cargo build -p project-knowledge-cli --no-default-features --locked
  的单独验证记录（本次仅验证了默认构建）。
