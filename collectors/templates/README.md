# 提取脚本模板

这里放可直接复制进目标项目 `.project-knowledge/collectors/<plugin-id>/<version>/src/` 的最小脚本模板。
形态参考 `../manifest.example.json`（真实字段以 `schema/` 导出的 JSON Schema 为准）。

## 约定（Node.js / Python 相同）

- 输入：一次 JSON 从 stdin 读入 —— 来源快照（仓库相对路径、内容哈希、行范围）、
  项目配置、已登记约定、可选的已有实体（link 阶段）。
- 输出：一次 JSON 写到 stdout —— `entities` / `relations` / `evidence` / `dependencies` / `gaps`。
- 失败：非 0 退出码 + stderr 诊断；结构化错误由 Core 组装，脚本不要打印人类长篇日志到 stdout。
- 禁止：扫描工作区之外的路径、联网、写数据库、读环境变量里的凭据。

## node/

- 目标文件：`extract.mjs`、`link.mjs`
- 运行时：Node.js 22.13+（与 SDK/MCP 一致）
- 依赖：只在 `deps/` 声明并锁定，由 Core 准备运行环境

## python/

- 目标文件：`extract.py`、`link.py`
- 运行时：Python 3.12+
- 依赖：`deps/requirements.txt` 或 `deps/pyproject.toml` + 锁文件

> 骨架阶段未附带可执行脚本模板：具体解析方式取决于目标项目实际语法与框架约定，
> 由 Agent 按 `prompts/collector.md` 生成后经验证激活。
