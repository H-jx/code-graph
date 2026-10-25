/**
 * 故障脚本：超时（父进程与子进程都不退出）。
 *
 * 用于验证宿主的超时与进程树终止：脚本派生一个孙进程并让两者都睡眠，
 * 宿主超时后必须把整棵进程树结束掉。
 */

import { spawn } from "node:child_process";

const child = spawn(process.execPath, ["-e", "setTimeout(() => {}, 600000)"], {
  stdio: "ignore",
});
child.unref();
process.stderr.write("fault: sleeping (grandchild alive)\n");
setTimeout(() => {}, 600000);
