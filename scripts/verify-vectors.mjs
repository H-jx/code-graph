/**
 * 校验 `fixtures/protocol/identity-vectors.json` 中的稳定身份测试向量。
 *
 * 用法：
 *   node scripts/verify-vectors.mjs                 # 校验并打印摘要
 *   node scripts/verify-vectors.mjs --write         # 重新生成向量文件
 *
 * 退出码：0 = 全部通过；1 = 有不一致；2 = 用法错误。
 */

import { readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import {
  computeEntityId,
  computeEvidenceId,
  computeRelationId,
  computeSourceId,
} from "./identity.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const vectorPath = resolve(here, "..", "fixtures", "protocol", "identity-vectors.json");

const args = process.argv.slice(2);
if (args.some((arg) => arg !== "--write")) {
  console.error("用法：node scripts/verify-vectors.mjs [--write]");
  process.exit(2);
}
const write = args.includes("--write");

const vectors = JSON.parse(readFileSync(vectorPath, "utf8"));
const results = [];
let failures = 0;

for (const vector of vectors.vectors) {
  let actual;
  try {
    if (vector.type === "entity") {
      actual = computeEntityId(vector.projectId, vector.kind, vector.slots);
    } else if (vector.type === "relation") {
      actual = computeRelationId(
        vector.projectId,
        vector.kind,
        vector.fromId,
        vector.toId,
        vector.qualifiers ?? {},
      );
    } else if (vector.type === "source") {
      actual = computeSourceId(vector.projectId, vector.repositoryId, vector.path);
    } else if (vector.type === "evidence") {
      actual = computeEvidenceId(
        vector.sourceId,
        vector.contentHash,
        vector.startByte,
        vector.endByte,
        vector.role,
      );
    } else {
      throw new Error(`unknown vector type: ${vector.type}`);
    }
  } catch (error) {
    actual = `ERROR: ${error.message}`;
  }
  const ok = actual === vector.expectedId;
  if (!ok) {
    failures += 1;
  }
  if (write && actual !== vector.expectedId && !actual.startsWith("ERROR:")) {
    vector.expectedId = actual;
  }
  results.push({ id: vector.id, ok, expected: vector.expectedId, actual });
}

const summary = {
  total: results.length,
  passed: results.length - failures,
  failed: failures,
  results,
};

if (write) {
  vectors.generatedBy = "scripts/verify-vectors.mjs";
  writeFileSync(vectorPath, `${JSON.stringify(vectors, null, 2)}\n`, "utf8");
  console.log(`已重写 ${vectorPath}（${results.length} 条向量）`);
} else {
  console.log(JSON.stringify(summary, null, 2));
}

process.exit(failures === 0 ? 0 : 1);
