# T05 — 可运行的基础索引、搜索、代码地图与证据读取

- ase-tests.log：cargo test -p project-knowledge-core --test indexing base_ 的输出。
- e2e/：开发版 CLI 的端到端记录（commands.log 为完整命令与退出码，编号 JSON 为每次输出，
  rtifact-hashes.txt 为产物 SHA-256）。
- 端到端步骤：init → index --base → status --check-freshness → search（中文）→
  search（标识符）→ query --request（请求文件）→ collect（明确失败）→ index --incremental。
