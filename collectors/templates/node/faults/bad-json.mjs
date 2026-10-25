/** 故障脚本：stdout 不是合法 JSON。 */
process.stderr.write("fault: writing invalid JSON\n");
process.stdout.write("this is not json\n");
process.exit(0);
