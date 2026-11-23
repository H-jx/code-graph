# Windows x64 候选安装

已验收的候选适用于 Windows x64 MSVC。使用 [构建指南](releasing.md) 在本机生成候选，或从构建方取得完整候选目录。CLI 压缩包、四个 npm tarball、`release-manifest.json` 和 `SHA256SUMS` 应来自同一次构建。验收记录见 [Windows 候选报告](evidence/T23/README.md)。

在 PowerShell 中设置候选目录并验证校验和：

```powershell
$candidate = (Resolve-Path -LiteralPath 'C:/candidate').Path
Get-Content -LiteralPath (Join-Path $candidate 'SHA256SUMS') | ForEach-Object {
    $expected, $file = $_ -split '\s+', 2
    $actual = Get-FileHash -LiteralPath (Join-Path $candidate $file) -Algorithm SHA256
    if ($actual.Hash -ne $expected) { throw "SHA-256 mismatch: $file" }
}
```

CLI 压缩包内有默认含 Agent 的 `project-knowledge.exe`、无 Agent 的 `project-knowledge-core.exe` 和许可证。解压到固定工具目录即可运行，基础索引无需 Rust、Node、Python、LLM 或业务数据库；SQLite 与 Prompt/模板资源已编译进入产品。

```powershell
$toolDir = 'C:/tools/project-knowledge'
New-Item -ItemType Directory -Force -Path $toolDir | Out-Null
tar -xzf (Join-Path $candidate 'project-knowledge-0.1.0-win32-x64-msvc.tar.gz') -C $toolDir
C:/tools/project-knowledge/project-knowledge.exe init --root C:/workspace/my-repo
C:/tools/project-knowledge/project-knowledge.exe index --config C:/workspace/my-repo/.project-knowledge/project.json --base
C:/tools/project-knowledge/project-knowledge.exe query --config C:/workspace/my-repo/.project-knowledge/project.json --operation search --text purchase
```

SDK/MCP 需 Node 24，不需 Rust 或 C 编译器。在独立 consumer 目录安装候选的四个 tarball，路径须指向同一候选目录：

```powershell
$consumer = 'C:/tools/project-knowledge'
New-Item -ItemType Directory -Force -Path $consumer | Out-Null
Set-Location -LiteralPath $consumer
npm init -y
$packages = @(
    'project-knowledge-native-win32-x64-msvc-0.1.0.tgz'
    'project-knowledge-native-0.1.0.tgz'
    'project-knowledge-sdk-0.1.0.tgz'
    'project-knowledge-mcp-0.1.0.tgz'
) | ForEach-Object { Join-Path $candidate $_ }
npm install --ignore-scripts @packages
```

四个包一起从候选目录安装。保留生成的 `package-lock.json` 和 tarball 路径。依赖第一次安装可能访问 npm，缓存就绪后可用 `npm ci --offline --ignore-scripts` 离线重装。缺失平台包时，SDK/MCP 会返回 `RUNTIME_MISSING`，可从同一候选目录重新安装。用法见 [SDK](sdk-usage.md) 和 [MCP](mcp-usage.md)。

内置 Agent 需另配置 provider/model/凭据及模型网络；不配置也能使用基础索引和查询。Node/Python 插件只在显式准备、验证、激活或增量运行时需要各自 manifest 声明的运行时和依赖。详见 [agent-usage.md](agent-usage.md) 与 [collectors.md](collectors.md)。

卸载时删除 CLI 解压目录或在 consumer 中 `npm uninstall @project-knowledge/mcp @project-knowledge/sdk @project-knowledge/native @project-knowledge/native-win32-x64-msvc`。项目配置和索引不会自动删除。升级先备份 `.project-knowledge`，只用显式维护命令升级或重建索引；只读 SDK/MCP 不自动迁移，不支持旧项目数据库迁移或旧 SDK 静默降级新库。

隔离安装验收在临时目录创建独立源码和 npm 锁文件，运行 PATH 排除 Rust/C/Python，CLI 另排除 Node，验证 tarball 实际安装、离线重装、SDK/MCP 查询、缺 native 和句柄释放。它不等同于全新 Windows 虚拟机或最低 OS 兼容性认证；实际主机版本记录在 manifest。
