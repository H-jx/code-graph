# T23 consumer 0xC0000409 根因定位与修复

日期：2026-11-11（Asia/Shanghai）· 执行者：opencode · Windows x64 · Node v24.14.0 · Rust 1.97.1

## 症状（与 2026-10-30 记录一致）

`scripts/verify-packages.mjs` 的 `sdk-mcp-consumer` 步退出码 **3221226505（0xC0000409，
STATUS_STACK_BUFFER_OVERRUN）**，stdout/stderr 日志为空，不产生 consumer-report.json /
installation-report.json。其余 11 步（CLI 解包/init/base/status/query/source、agent-help、
core-agent-disabled、runtime-exclusions、npm-install、npm-ci-offline）全部按预期退出。

## 定位方法

1. **dev 对照**：用既有 dev profile native 二进制，在等价隔离环境（中文空格临时目录、复制
   node.exe、PATH 仅 runtime+System32、净化 env、完整插件生命周期 + MCP stdio + 缺 native 检查）
   运行带阶段标记的诊断 consumer。dev 下全部通过 → 崩溃非环境/路径/流程的必然结果。
2. **release 复现 + 阶段标记**：对 release 候选重跑 `verify-packages`，崩溃复现（exit
   3221226505、日志 0 字节）。把带 `writeSync(2, "STAGE n")` 同步标记的诊断脚本复制进残留
   consumer 目录运行，标记停在 **`26-missing-native`** 之后、`27-report` 之前。stage 01–25
   （SDK 查询、MCP stdio、submit/prepare/validate/activate/collect、Agent configure --check）
   全部通过。
3. **隔离 stage 26**：单独运行 missing-native 检查（**未加载任何产品 native**），子标记停在
   `26.3-copied-pkgjson sdk` 之后、`26.4-cpSync-sdk-dist` 之前 → 崩溃发生在
   `fs.cpSync(installedSdkDist, …, {recursive:true})`，源路径位于中文 consumer 目录下。
4. **最小复现（零产品代码）**：新鲜 node 进程直接 `fs.cpSync` 含中文的源路径 → 进程退出
   **-1073740791（0xC0000409）**。

## 根因

**Node v24.14.0 / libuv 在 Windows 上 `fs.cpSync` 对非 ASCII 路径的内存损坏缺陷**，与本产品
（Rust Core / native / SDK / MCP / Agent）无关。受控矩阵（每例独立进程，确定性复现）：

| 源路径 | 目标路径 | 结果 |
| --- | --- | --- |
| ASCII | ASCII | 正常复制 |
| 含中文 | ASCII | **崩溃 0xC0000409** |
| ASCII | 含中文 | 静默不复制（返回成功，目标缺失） |
| 含中文 | 含中文 | **崩溃 0xC0000409** |

触发依赖精确路径字节/布局（栈缓冲区溢出特征）：等长、同含「中文 空格」的两个目录可一崩一不崩，
按具体路径确定性复现。产品自身在中文空格路径下完全正常——stage 01–25 全通过，含插件 staging
中文路径、MCP 子进程、Agent CLI；Core/native/SDK/MCP 源码无任何 `cpSync`。

崩溃点是验收脚本 `scripts/package-consumer.mjs` 的缺 native 检查：它把已安装 SDK 的 `dist`
从中文 consumer 目录 `cpSync` 到一个临时目录，再验证缺平台包时返回 `RUNTIME_MISSING`。这是
**测试夹具的文件复制操作**，不是产品功能。

## 修复

`scripts/package-consumer.mjs`：以等价的手动递归复制 `copyTree`（`mkdirSync` + `copyFileSync`，
二者已证明正确处理中文路径）替换 `fs.cpSync`，并从 import 移除 `cpSync`。

- **不削弱验收**：缺 native 检查逻辑不变（仍复制 SDK dist + native loader 到独立目录并断言
  `RUNTIME_MISSING`）；中文空格路径的产品测试完整保留（consumer 目录仍为 `pk-installed-中文 空格-*`）。
- 复制语义更正确：原 `cpSync` 在非 ASCII 目标下还会静默漏复制。
- 属打包/验收辅助脚本改动，符合收尾核对约定的变更范围；产品二进制不变。

`scripts/package.mjs:253` 也用了 `cpSync`，但其源/目标恒为 ASCII（仓库根 + target/release-candidate），
不受影响，未改动；若将来从非 ASCII 构建路径打包需同样规避（已知环境限制）。

## 验证

修复后对 release 候选重跑 `node scripts/verify-packages.mjs --artifacts <candidate>`：
**`{"passed":true}`**。`sdk-mcp-consumer` 退出 0；consumer-report `passed:true`、
`nativeMissing:"RUNTIME_MISSING"`、插件激活 `active`、Agent `complete`（2 轮/1 工具）、
`databaseHashUnchanged`、`externalModelRequests:0`、`localProviderRequests:2`；
`buildProfile:"release"`、`platform:"win32"`、`architecture:"x64"`。

最终候选在修复后重建，使 source-manifest 绑定修复后的脚本哈希；官方 installation-report 以
最终候选为准（见同目录 verification-report.json）。

## 复现脚本

`cpsync-repro.mjs`（本目录）：零产品依赖，演示 Node `fs.cpSync` 非 ASCII 源崩溃 / 非 ASCII 目标静默漏复制。
运行：`node cpsync-repro.mjs`（在 Windows + Node 24.14.0 下；含中文源用例会使该进程以 0xC0000409 退出）。