import { dirname, join } from 'node:path';
import { existsSync, readFileSync } from 'node:fs';

// Some dependencies export their entry point but deliberately hide package.json.
export function installedPackageMetadata(require, name) {
  let directory = dirname(require.resolve(name));
  while (true) {
    const file = join(directory, 'package.json');
    if (existsSync(file)) {
      const metadata = JSON.parse(readFileSync(file, 'utf8'));
      if (metadata.name === name) return metadata;
    }
    const parent = dirname(directory);
    if (parent === directory) throw new Error(`Cannot find installed metadata for ${name}`);
    directory = parent;
  }
}
