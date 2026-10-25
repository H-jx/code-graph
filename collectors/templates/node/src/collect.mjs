/**
 * 采集脚本模板（Node.js）。
 *
 * 协议（计划 §3.5）：
 * - stdin 收到一个 UTF-8 JSON 对象（CollectorInput），读取到 EOF 表示输入结束；
 * - stdout 只能输出一个 JSON 对象（CollectorOutput）；
 * - 调试日志写 stderr，退出码非零表示整阶段失败。
 *
 * 模板只演示协议骨架：真实插件要按项目调查结论解析源码，并附原始证据。
 */

import { readFileSync } from "node:fs";

/** 读取 stdin 全部内容。 */
function readStdin() {
  try {
    return readFileSync(0, "utf8");
  } catch (error) {
    process.stderr.write(`cannot read stdin: ${error.message}\n`);
    process.exit(2);
  }
}

/** 解析 CollectorInput。 */
function parseInput() {
  const raw = readStdin();
  if (!raw.trim()) {
    process.stderr.write("stdin is empty; expected one CollectorInput JSON object\n");
    process.exit(2);
  }
  try {
    return JSON.parse(raw);
  } catch (error) {
    process.stderr.write(`stdin is not valid JSON: ${error.message}\n`);
    process.exit(2);
  }
}

/** 构造空的 CollectorOutput。 */
function emptyOutput(input) {
  return {
    protocolVersion: input.protocolVersion,
    runId: input.runId,
    stageId: input.stageId,
    inputSnapshotId: input.inputSnapshotId,
    entities: [],
    relations: [],
    evidence: [],
    dependencies: [],
    coverage: [],
    gaps: [],
  };
}

function main() {
  const input = parseInput();
  const output = emptyOutput(input);
  process.stderr.write(
    `collector ${input.pluginId}@${input.version} stage ${input.stageId}: ` +
      `${input.sources.length} sources, ${input.entities.length} entities\n`,
  );
  process.stdout.write(`${JSON.stringify(output)}\n`);
}

main();
