# Windows x64 候选安装

首版范围为 Windows x64 MSVC。Linux/macOS 转后续验证；本轮不公开发布 npm 或 GitHub Release。下载/传递完整候选目录后，先核对 `release-manifest.json` 与 `SHA256SUMS`；不要混用不同候选的包或二进制。

CLI 压缩包内有默认含 Agent 的 `project-knowledge.exe`、无 Agent 的 `project-knowledge-core.exe` 和许可证。解压到固定工具目录即可运行，基础索引无需 Rust、Node、Python、LLM 或业务数据库；SQLite 与 Prompt/模板资源已编译进入产品。

```powershell
tar -xzf project-knowledge-0.1.0-win32-x64-msvc.tar.gz -C C:/tools/project-knowledge
C:/tools/project-knowledge/project-knowledge.exe init --root C:/workspace/my-repo
C:/tools/project-knowledge/project-knowledge.exe index --config C:/workspace/my-repo/.project-knowledge/project.json --base
C:/tools/project-knowledge/project-knowledge.exe query --config C:/workspace/my-repo/.project-knowledge/project.json --operation search --text purchase
```

SDK/MCP 需 Node 24，不需 Rust 或 C 编译器。在独立 consumer 目录安装候选的四个 tarball，路径须指向同一候选目录：

```powershell
npm init -y
npm install --ignore-scripts C:/candidate/project-knowledge-native-win32-x64-msvc-0.1.0.tgz C:/candidate/project-knowledge-native-0.1.0.tgz C:/candidate/project-knowledge-sdk-0.1.0.tgz C:/candidate/project-knowledge-mcp-0.1.0.tgz
```

四个包必须一起安装，因为当前 npm scope 尚未作为本项目公开发布。保存生成的 package-lock.json 和 tarball 路径；依赖第一次安装可能访问 npm，缓存就绪后 `npm ci --offline --ignore-scripts` 可离线重装。运行 SDK/MCP 时不下载或编译 native；缺失平台包返回 `RUNTIME_MISSING`。用法见 [SDK](sdk-usage.md) 和 [MCP](mcp-usage.md)。

内置 Agent 需另配置 provider/model/凭据及模型网络；不配置也能使用基础索引和查询。Node/Python 插件只在显式准备、验证、激活或增量运行时需要各自 manifest 声明的运行时和依赖。详见 [agent-usage.md](agent-usage.md) 与 [collectors.md](collectors.md)。

卸载时删除 CLI 解压目录或在 consumer 中 `npm uninstall @project-knowledge/mcp @project-knowledge/sdk @project-knowledge/native @project-knowledge/native-win32-x64-msvc`。项目配置和索引不会自动删除。升级先备份 `.project-knowledge`，只用显式维护命令升级或重建索引；只读 SDK/MCP 不自动迁移，不支持旧项目数据库迁移或旧 SDK 静默降级新库。

隔离安装验收在临时目录创建独立源码和 npm 锁文件，运行 PATH 排除 Rust/C/Python，CLI 另排除 Node，验证 tarball 实际安装、离线重装、SDK/MCP 查询、缺 native 和句柄释放。它不等同于全新 Windows 虚拟机或最低 OS 兼容性认证；实际主机版本记录在 manifest。
