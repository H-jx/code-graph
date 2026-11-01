import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
const root = fileURLToPath(new URL('../', import.meta.url));
const args = process.argv.slice(2).filter((arg) => arg !== '--');
const requested = (
  args[args.indexOf('--capabilities') + 1] ?? 'routes,frontend,comments,modules,entrypoints'
).split(',');
const frontend = ['routes', 'frontend', 'comments', 'modules', 'entrypoints'];
const supported = [...frontend, 'api', 'database', 'link', 'workflow', 'report'];
if (requested.some((capability) => !supported.includes(capability))) {
  console.error(
    'Requested capabilities are not implemented yet: ' +
      requested.filter((c) => !supported.includes(c)).join(','),
  );
  process.exit(2);
}
const commands = [];
for (const [selected, packaging, filter] of [
  [
    requested.some((c) => frontend.includes(c)),
    'scripts/package-example-collectors.mjs',
    'frontend_',
  ],
  [requested.includes('api'), 'scripts/package-api-examples.mjs', 'backend_api_'],
  [requested.includes('database'), 'scripts/package-database-examples.mjs', 'backend_database_'],
  [requested.includes('link'), 'scripts/package-link-examples.mjs', 'linking_'],
  [
    requested.some((capability) => ['workflow', 'report'].includes(capability)),
    'scripts/package-semantic-examples.mjs',
    'semantics_',
  ],
])
  if (selected)
    commands.push(
      ['node', [packaging, '--check']],
      [
        'cargo',
        [
          'test',
          '-p',
          'project-knowledge-core',
          '--test',
          'business_scenarios',
          filter,
          '--locked',
          '-j',
          '1',
          '--',
          '--test-threads=1',
        ],
      ],
    );
for (const [command, parameters] of commands) {
  const result = spawnSync(command, parameters, { cwd: root, stdio: 'inherit', env: process.env });
  if (result.error) {
    console.error(result.error.message);
    process.exit(1);
  }
  if (result.status !== 0) process.exit(result.status ?? 1);
}
