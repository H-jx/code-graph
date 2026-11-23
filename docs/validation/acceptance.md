# 首版验收范围

Windows x64 候选已通过隔离安装、离线重装和 CLI、SDK、MCP 查询验收。Linux 和 macOS 已列入构建矩阵，支持状态以各自的安装验收为准。完整结果见 [MCP 验收](../evidence/T21/README.md)、[三端对照](../evidence/T22/README.md) 和 [Windows 候选验收](../evidence/T23/README.md)。

验收人口分开记录：

- 真实 synes 来源：原先授权只读的前后端项目，11 份冻结文件和 12 条源码/文档声明事实；本轮直接回查来源哈希与逐行引用。它们是实现中声明的事实，未取得人工签认，也不代表真实业务要求正确。
- A/B 布局：源码独立审阅的合成场景与补充绑定，验证实体、关系、原始证据、歧义/动态缺口及 CLI/SDK/MCP 一致性。expected/forbidden 分母和命中范围在 `T22/acceptance-report.json`，不外推整体生产准确率。
- 数值审计：本地独立整数累加 oracle 与 SQLite 正确预聚合逐项比较，三行及合计精确一致；fanout 和 WHERE 子表过滤两个错误查询必须被检出。不是生产财务口径。
- Agent：T19 既有真实模型业务闭环与本轮 mock provider 故障回归分别记录，本轮不新调用模型或业务数据库。

`scripts/acceptance-cases.mjs` 将 21 个业务条目与 16 个 X 负例映射到真实测试函数。`verify-plan.mjs` 检查全部 ID、依赖 DAG、命令与测试位置；`verify-acceptance.mjs` 只接纳本轮工作区日志中实际通过的函数，并核对三端完整响应。归一化只移除 requestId、checkedAt、elapsedMs、responseChars；实体、关系、引用、缺口、partial 和分页内容保留。

复现：

```powershell
python scripts/verify-final.py
node scripts/verify-acceptance.mjs
pnpm run verify:scenarios -- --scenario all --transport all --case X12
```

`--case` 执行映射的相关测试；无 `--case` 的三端场景覆盖 A/B 页面、流程、台账，以及已有自然语言、分页和负例请求。生命周期/故障测试由完整工作区运行统一验收，不把共用 Core 的三个入口计成三份独立真值。

安装报告记录 release profile、实际 Windows 版本、平台包选择、排除构建工具后的 PATH、独立 consumer 和离线重装结果。最低 Windows 版本、全新虚拟机、物理断电及第二个真实多仓库项目均待验证。原始业务资料的作者签认和部分历史审查材料仍缺失。
