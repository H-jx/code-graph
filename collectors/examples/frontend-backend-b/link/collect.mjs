import { linkFacts } from './link-facts.mjs';
let text=''; for await (const chunk of process.stdin) text+=chunk;
process.stdout.write(JSON.stringify(linkFacts(JSON.parse(text))));
