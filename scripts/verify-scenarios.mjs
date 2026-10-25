/**
 * 场景验收驱动（骨架，计划 T16/T22）。
 *
 * 目标调用：
 *   pnpm run verify:scenarios -- --scenario all --transport cli
 *   pnpm run verify:scenarios -- --scenario 3 --case S3-04 --transport sdk
 *
 * 目标行为：用同一冻结真值（fixtures/truth）驱动 CLI/SDK/MCP，比较去除 requestId 与时间后的
 * 实体、关系、证据、缺口与 completeness；数值项与独立 oracle 逐值核对。
 */

console.error('TODO(pk) T16/T22: verify-scenarios 未实现（需 CLI 场景命令与真值包）。');
process.exitCode = 2;
