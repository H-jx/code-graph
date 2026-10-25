/** 故障脚本：输出洪泛（超过 stdout 上限）。 */
const chunk = "x".repeat(64 * 1024);
for (let index = 0; index < 1024; index += 1) {
  process.stdout.write(chunk);
}
process.exit(0);
