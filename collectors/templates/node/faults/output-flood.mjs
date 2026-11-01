/** 故障脚本：输出洪泛（超过 stdout 上限）。 */
import { once } from "node:events";
const chunk = "x".repeat(64 * 1024);
for (let index = 0; index < 1024; index += 1) {
  // 等待管道实际排空，不能用 process.exit 截断尚未写出的洪泛数据。
  if (!process.stdout.write(chunk)) await once(process.stdout, "drain");
}
