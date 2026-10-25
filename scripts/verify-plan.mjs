/**
 * 校验任务文档中的任务 ID、依赖 DAG、需求映射与目标命令登记（计划 §4.1 / T01 / T22）。
 *
 * 用法：
 *   node scripts/verify-plan.mjs --plan ../plan/NEW-PROJECT-LONG-TASK.md
 *
 * 退出码：0 = 全部通过；1 = 存在问题。
 */

import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const repositoryRoot = resolve(here, "..");

const args = process.argv.slice(2);
const planIndex = args.indexOf("--plan");
const planPath = resolve(
  here,
  planIndex >= 0 && args[planIndex + 1] ? args[planIndex + 1] : "../plan/NEW-PROJECT-LONG-TASK.md",
);

const problems = [];
const notes = [];

let plan;
try {
  plan = readFileSync(planPath, "utf8");
} catch (error) {
  console.error(`无法读取任务文档 ${planPath}：${error.message}`);
  process.exit(1);
}

// ---------------------------------------------------------------- 任务与依赖

const taskPattern = /^###\s+(T\d{2})\s+—\s+(.+)$/gm;
const tasks = [];
let match;
while ((match = taskPattern.exec(plan)) !== null) {
  tasks.push({ id: match[1], title: match[2].trim(), index: match.index });
}

if (tasks.length === 0) {
  problems.push("任务文档中没有找到 `### Tnn — 标题` 形式的任务标题");
}

const statusPattern = /-\s*状态：`(\w+)`；前置依赖：([^\n。]*)。/;
const statuses = new Map();
const dependencies = new Map();
for (let index = 0; index < tasks.length; index += 1) {
  const start = tasks[index].index;
  const end = index + 1 < tasks.length ? tasks[index + 1].index : plan.length;
  const section = plan.slice(start, end);
  const statusMatch = section.match(statusPattern);
  if (!statusMatch) {
    problems.push(`任务 ${tasks[index].id} 没有找到「状态 / 前置依赖」行`);
    continue;
  }
  statuses.set(tasks[index].id, statusMatch[1]);
  const raw = statusMatch[2].trim();
  dependencies.set(
    tasks[index].id,
    raw === "" || raw === "无" ? [] : [...raw.matchAll(/T\d{2}/g)].map((entry) => entry[0]),
  );
}

const allowedStatuses = new Set(["todo", "doing", "blocked", "done"]);
for (const [taskId, status] of statuses) {
  if (!allowedStatuses.has(status)) {
    problems.push(`任务 ${taskId} 的状态 \`${status}\` 不合法（允许 todo/doing/blocked/done）`);
  }
}

const known = new Set(tasks.map((task) => task.id));
for (const [taskId, deps] of dependencies) {
  for (const dependency of deps) {
    if (!known.has(dependency)) {
      problems.push(`任务 ${taskId} 依赖了不存在的任务 ${dependency}`);
    }
    if (dependency === taskId) {
      problems.push(`任务 ${taskId} 依赖了自己`);
    }
  }
}

// 环检测（DFS）。
const visiting = new Set();
const visited = new Set();
function detectCycle(taskId, stack) {
  if (visiting.has(taskId)) {
    problems.push(`依赖存在环：${[...stack, taskId].join(" → ")}`);
    return;
  }
  if (visited.has(taskId)) {
    return;
  }
  visiting.add(taskId);
  for (const dependency of dependencies.get(taskId) ?? []) {
    if (known.has(dependency)) {
      detectCycle(dependency, [...stack, taskId]);
    }
  }
  visiting.delete(taskId);
  visited.add(taskId);
}
for (const task of tasks) {
  detectCycle(task.id, []);
}

// 依赖必须指向更早的任务（文档顺序即实施顺序）。
const order = new Map(tasks.map((task, index) => [task.id, index]));
for (const [taskId, deps] of dependencies) {
  for (const dependency of deps) {
    if (!known.has(dependency)) {
      continue;
    }
    if (order.get(dependency) >= order.get(taskId)) {
      problems.push(
        `任务 ${taskId} 的前置依赖 ${dependency} 在文档中不是更早的任务，实施顺序不成立`,
      );
    }
  }
}

// ---------------------------------------------------------------- 目标命令登记

const requiredScripts = [
  "build:contracts",
  "check:contracts",
  "build:native",
  "build:sdk",
  "build:mcp",
  "typecheck",
  "test:sdk",
  "test:mcp",
  "test:plugins",
  "verify:scenarios",
  "verify:docs",
  "verify:packages",
];

let packageJson;
try {
  packageJson = JSON.parse(readFileSync(resolve(repositoryRoot, "package.json"), "utf8"));
} catch (error) {
  problems.push(`无法读取 package.json：${error.message}`);
}
const scripts = packageJson?.scripts ?? {};
for (const script of requiredScripts) {
  if (!scripts[script]) {
    problems.push(`package.json 缺少目标脚本 \`${script}\`（计划 §4.1 登记的命令族）`);
  }
}

// 计划正文里出现的 `pnpm run <script>` 必须已登记。
for (const match2 of plan.matchAll(/pnpm run ([a-z:]+)/g)) {
  const script = match2[1];
  if (!scripts[script]) {
    problems.push(`任务文档引用了未登记的脚本 \`pnpm run ${script}\``);
  }
}

// ---------------------------------------------------------------- 需求映射

const requirementRows = plan
  .split("\n")
  .filter((line) => /^\|\s*[^\s|]+\s*\|/.test(line) && /\|\s*T\d{2}/.test(line));
if (requirementRows.length === 0) {
  problems.push("任务文档中没有找到需求追踪表（§5.6）");
}
for (const row of requirementRows) {
  const cells = row.split("|").map((cell) => cell.trim());
  const referenced = [...row.matchAll(/T\d{2}/g)].map((entry) => entry[0]);
  if (referenced.length === 0) {
    problems.push(`需求追踪行没有引用任何任务：${row.trim().slice(0, 60)}`);
  }
  for (const taskId of referenced) {
    if (!known.has(taskId)) {
      problems.push(`需求追踪引用了不存在的任务 ${taskId}`);
    }
  }
  if (cells.length >= 2 && cells[1].length === 0) {
    problems.push(`需求追踪行缺少需求组名称：${row.trim().slice(0, 60)}`);
  }
}

// ---------------------------------------------------------------- 场景矩阵

const scenarioIds = [...plan.matchAll(/^\|\s*(S[123]-\d{2})\s*\|/gm)].map((entry) => entry[1]);
if (scenarioIds.length < 15) {
  problems.push(`场景矩阵条目过少（找到 ${scenarioIds.length} 个 Sxx-xx 编号）`);
}
const negativeIds = [...plan.matchAll(/^\|\s*(X\d{2})\s*\|/gm)].map((entry) => entry[1]);
if (negativeIds.length < 10) {
  problems.push(`负例矩阵条目过少（找到 ${negativeIds.length} 个 Xxx 编号）`);
}

// ---------------------------------------------------------------- 输出

notes.push(`任务数量：${tasks.length}`);
notes.push(`带状态的任务：${statuses.size}`);
notes.push(`场景条目：${scenarioIds.length}，负例条目：${negativeIds.length}`);
notes.push(`目标脚本登记：${requiredScripts.length}`);

for (const note of notes) {
  console.log(`info: ${note}`);
}
if (problems.length > 0) {
  console.error(`verify:docs 失败，共 ${problems.length} 个问题：`);
  for (const problem of problems) {
    console.error(`  - ${problem}`);
  }
  process.exit(1);
}
console.log("verify:docs 通过：任务 ID、依赖 DAG、需求映射与目标命令登记均一致。");
