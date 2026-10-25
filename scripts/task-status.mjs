// 打印任务文档中的任务状态。
import { readFileSync } from "node:fs";

const path = process.argv[2] ?? "plan/NEW-PROJECT-LONG-TASK.md";
const text = readFileSync(path, "utf8");
const tasks = [...text.matchAll(/###\s+(T\d{2})\s+—/g)];
const out = [];
for (let index = 0; index < tasks.length; index += 1) {
  const start = tasks[index].index;
  const end = index + 1 < tasks.length ? tasks[index + 1].index : text.length;
  const section = text.slice(start, end);
  const status = section.match(/- 状态：`(\w+)`/);
  out.push(`${tasks[index][1]}=${status ? status[1] : "?"}`);
}
console.log(out.join(" "));
