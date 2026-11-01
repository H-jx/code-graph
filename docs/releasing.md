# 生成可安装候选

首版 Windows x64，版本 `0.1.0`，尚未公开发布。源码构建需锁定 Rust 1.97.1、Node 24、pnpm 10.34.8 和 MSVC；从仓库根执行：

```powershell
pnpm install --frozen-lockfile
python scripts/verify-final.py
cargo run -p xtask -- package --target x86_64-pc-windows-msvc
pnpm run verify:packages -- --artifacts <本次输出的候选目录>
```

pack 默认使用 release profile，构建默认/无 Agent CLI、当前平台 native、SDK/MCP。每次创建独立 `target/release-candidate/0.1.0/<target>/candidate-*`，保留命令日志、CLI 压缩包、四个 npm tarball、版本/锁文件/Schema/二进制哈希与依赖清单。`release-manifest.json` 的构建状态不因“打包成功”变成安装通过；以独立 `installation-report.json` 为验收证据。`--profile dev` 仅用于调试打包流程，manifest 会明确记录，不将其冒充 release 产物。

`.github/workflows/release.yml` 只准备和验证 Windows 候选。CI 配置存在不代表已执行；当前工作区无 Git remote，不能虚构远程 run。其他三平台由用户明确列为后续，须分别在目标主机完成构建、安装和进程生命周期验收才加入支持声明。runner 名称核对依据为 [GitHub 官方清单](https://github.com/actions/runner-images/blob/main/README.md)。

本轮不执行公开发布。后续明确授权后，核验 npm scope 身份/命名权限、公共仓库位置及公开包内容；依次发布平台包→native→SDK→MCP，再发布 CLI。只发布已验收哈希，不覆盖同版本不同内容；凭据只在渠道 secret 使用。当前仓库 `repository` 占位元数据未作为真实公开仓库或权限证明。

依赖清单用于追溯实际锁定组件，不是漏洞或许可合规认证。历史缺失审查资料、物理断电、最低 Windows 版本和未提供第二真实仓库的验证限制继续保留在 [acceptance.md](validation/acceptance.md)。
