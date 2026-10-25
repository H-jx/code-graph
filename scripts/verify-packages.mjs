/**
 * 发布制品与干净安装验收（骨架，计划 T23）。
 *
 * 目标调用：pnpm run verify:packages -- --artifacts <dir>
 *
 * 目标行为：在临时干净目录里安装各 npm 包与平台产物、CLI 压缩包，移除不该需要的工具
 * （Rust/C/Python/模型凭据），执行 init → index --base → status → search → readSource；
 * 无匹配平台 native 包时给出明确 unsupported 错误，不触发源码编译或未知下载。
 */

console.error('TODO(pk) T23: verify-packages 未实现（需 xtask package 产出制品目录）。');
process.exitCode = 2;
