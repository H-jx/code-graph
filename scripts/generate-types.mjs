/**
 * 从 Rust 导出的 JSON Schema 生成 TypeScript 类型（计划 T02）。
 *
 * 目标流程：
 * 1. `cargo run -p xtask -- export-contracts`
 *    → 从 `crates/core/src/contract` 导出 JSON Schema（2020-12）到 `docs/schema/`；
 * 2. `node scripts/generate-types.mjs`
 *    → 用 json-schema-to-typescript 生成 `packages/sdk/src/generated/contract.ts`。
 *
 * SDK / MCP 只消费生成类型，不在仓库里另写一套手写模型。
 *
 * `--check` 模式在内存中重新生成并与磁盘内容比较；不一致时以非零码退出，
 * 避免“生成后再声称一致”。
 */

import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { compile } from "json-schema-to-typescript";

const here = dirname(fileURLToPath(import.meta.url));
const repositoryRoot = resolve(here, "..");
const schemaPath = resolve(repositoryRoot, "docs", "schema", "contract.schema.json");
const outputPath = resolve(
  repositoryRoot,
  "packages",
  "sdk",
  "src",
  "generated",
  "contract.ts",
);

const args = process.argv.slice(2);
const unknown = args.filter((arg) => arg !== "--check");
if (unknown.length > 0) {
  console.error(`未知参数：${unknown.join(", ")}`);
  console.error("用法：node scripts/generate-types.mjs [--check]");
  process.exit(2);
}
const check = args.includes("--check");

let schema;
try {
  schema = JSON.parse(readFileSync(schemaPath, "utf8"));
} catch (error) {
  console.error(`无法读取 Schema ${schemaPath}：${error.message}`);
  console.error("请先运行 `cargo run -p xtask -- export-contracts`。");
  process.exit(2);
}

const banner = `// 由 scripts/generate-types.mjs 从 docs/schema/contract.schema.json 生成。
// 请勿手工编辑：改动 Rust 契约后重新运行 \`pnpm run build:contracts\`。
`;

const generated = await compile(schema, "Contract", {
  bannerComment: "",
  additionalProperties: false,
  style: {
    singleQuote: false,
    semi: true,
    printWidth: 100,
  },
});

const content = `${banner}${generated.trim()}\n`;

if (check) {
  let existing = "";
  try {
    existing = readFileSync(outputPath, "utf8");
  } catch {
    existing = "";
  }
  if (existing !== content) {
    console.error(`生成的类型与 ${outputPath} 不一致；请运行 \`pnpm run build:contracts\`。`);
    process.exit(1);
  }
  console.log(`生成的类型与 ${outputPath} 一致。`);
} else {
  mkdirSync(dirname(outputPath), { recursive: true });
  writeFileSync(outputPath, content, "utf8");
  console.log(`wrote ${outputPath}`);
}
