# CI runner 名称核对（T23 步骤 3 / §6.2）

- 核对日期：2026-11-11（Asia/Shanghai）
- 依据：GitHub 官方 actions/runner-images README（main 分支，当日抓取）
  https://github.com/actions/runner-images/blob/main/README.md

## 当时可用标签（节选）

| 标签 | 架构 | 状态 |
| --- | --- | --- |
| `windows-2022` | x64 | 可用 |
| `windows-latest` / `windows-2025` | x64 | 可用（VS2026） |
| `ubuntu-24.04` / `ubuntu-latest` | x64 | 可用 |
| `macos-15-intel` / `macos-15-large` | x64 | 可用 |
| `macos-15` / `macos-15-xlarge` | arm64 | 可用 |
| `macos-26` / `macos-latest` | arm64 | 可用 |
| `macos-14` | x64 | **2026-11-02 已从 GitHub Actions 退役**（#15226；Azure DevOps 延至 2026-11-23，不适用本仓库） |

## 本仓库处置

- `.github/workflows/release.yml` 的 `aarch64-apple-darwin` 原用 `macos-14`，已改为 `macos-15`（arm64）。
- `windows-2022`、`ubuntu-24.04`、`macos-15-intel` 与 ci.yml 的 `macos-15` 均仍可用，未改动。
- 本仓库无 Git remote，本轮未触发远程 Actions；runner 核对是配置正确性检查，不是远程 run 证据。
- 其余三平台（Linux/macOS x64/arm64）按 D31 仍为后续范围，须在目标主机实际构建+安装验收后才加入支持声明。

## 附带观察（未修改，发布授权前需核对）

- release.yml `publish` job 的 `candidates/*/*.tgz` glob 与 upload-artifact 的目录层级
  （`target/release-candidate/**` 的最长公共祖先）可能不匹配；本轮不公开发布，publish job 未执行，
  留待授权发布前与 npm scope 权限一并核验。