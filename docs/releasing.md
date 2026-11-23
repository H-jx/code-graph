# 生成可安装候选

构建 Windows x64 候选需要 Rust 1.97.1、Node 24、pnpm 10.34.8、MSVC、ripgrep 和 tar。从仓库根运行：

```powershell
pnpm install --frozen-lockfile --ignore-scripts
cargo run --locked -p xtask -- package --target x86_64-pc-windows-msvc
if ($LASTEXITCODE -ne 0) { throw '候选构建失败' }
$candidate = Get-ChildItem -LiteralPath 'target/release-candidate/0.1.0/x86_64-pc-windows-msvc' -Directory |
    Sort-Object LastWriteTime -Descending |
    Select-Object -First 1
pnpm run verify:packages -- --artifacts $candidate.FullName
```

打包默认使用 release profile，构建含 Agent 和 Core-only 的 CLI，以及当前平台 native、SDK 和 MCP。每次创建独立的 `target/release-candidate/0.1.0/<target>/candidate-*`，内含 CLI 压缩包、四个 npm tarball、构建日志、版本和哈希清单。用 `installation-report.json` 确认候选通过隔离安装验收。`--profile dev` 用于调试打包流程。

完整项目验收需要 Python 3.12 和 ripgrep，可运行 `python scripts/verify-final.py` 与 `node scripts/verify-acceptance.mjs`。当前验收范围见 [首版验收](validation/acceptance.md)。

`.github/workflows/release.yml` 当前包含 Windows/Linux/macOS 构建矩阵及公开发布步骤。日常本地交付使用上面的候选构建命令。公开发布需另行核验 npm scope 权限、仓库元数据和目标平台验收结果。

依赖清单用于追溯实际锁定组件，不是漏洞或许可合规认证。历史缺失审查资料、物理断电、最低 Windows 版本和未提供第二真实仓库的验证限制继续保留在 [acceptance.md](validation/acceptance.md)。
