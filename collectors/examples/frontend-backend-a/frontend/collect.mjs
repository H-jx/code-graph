import { readFileSync } from 'node:fs';
import { collect } from './frontend-ast.mjs';
const rules=JSON.parse(readFileSync(new URL('./conventions.json',import.meta.url),'utf8'));
let text='';
for await(const chunk of process.stdin) text+=chunk;
process.stdout.write(JSON.stringify(collect(JSON.parse(text),rules)));
