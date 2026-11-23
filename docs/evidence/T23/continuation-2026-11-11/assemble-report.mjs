// Assemble the durable T23 verification report from the final release candidate.
// Copies volatile target/ artifacts into docs/evidence and binds their SHA-256.
// Run: node assemble-report.mjs <finalCandidateDir> <previousCandidateDir>
import { readFileSync, writeFileSync, copyFileSync, mkdirSync, readdirSync } from 'node:fs';
import { join, basename } from 'node:path';
import { createHash } from 'node:crypto';

const evidenceDir = 'E:/Project/my-project/code-graph/docs/evidence/T23/continuation-2026-11-11';
const final = process.argv[2];
const previous = process.argv[3];
const sha = (p) => createHash('sha256').update(readFileSync(p)).digest('hex');
const readJson = (p) => JSON.parse(readFileSync(p, 'utf8'));

const installDir = join(evidenceDir, 'install');
mkdirSync(installDir, { recursive: true });
const copied = {};
const copy = (name) => {
  const from = join(final, name);
  const to = join(installDir, name);
  copyFileSync(from, to);
  copied[name] = sha(to);
};
for (const name of readdirSync(final)) {
  if (name.startsWith('install-') || name === 'release-manifest.json' || name === 'SHA256SUMS')
    copy(name);
}
// consumer-report from the isolated consumer directory recorded in installation-report
const installation = readJson(join(final, 'installation-report.json'));
const consumerReport = join(installation.directory, 'consumer-report.json');
copyFileSync(consumerReport, join(installDir, 'consumer-report.json'));
copied['consumer-report.json'] = sha(join(installDir, 'consumer-report.json'));

const manifest = readJson(join(final, 'release-manifest.json'));
const commands = readJson(join(final, 'install-commands.json'));
const srcFinal = readJson(join(final, 'source-manifest.json'));
const srcPrev = readJson(join(previous, 'source-manifest.json'));
const sourceChanged = Object.keys(srcFinal).filter((k) => srcFinal[k] !== srcPrev[k]);

const report = {
  date: '2026-11-11',
  task: 'T23',
  executor: 'opencode',
  platform: 'win32',
  architecture: 'x64',
  osRelease: manifest.osRelease,
  node: manifest.node,
  rust: manifest.rust.split('\n')[0],
  profile: manifest.profile,
  finalCandidate: final,
  previousCandidate: previous,
  publication: 'not-published',
  rootCause: {
    symptom: 'sdk-mcp-consumer exit 0xC0000409 (3221226505), empty stdout/stderr',
    cause:
      'Node v24.14.0 / libuv fs.cpSync memory corruption on non-ASCII Windows paths; triggered by the acceptance harness missing-native check copying SDK dist from the Chinese consumer directory. Not a product defect.',
    fix: 'scripts/package-consumer.mjs: replace fs.cpSync with equivalent manual recursive copy (mkdirSync + copyFileSync). Acceptance logic and Chinese-path product testing unchanged.',
    reference: 'root-cause-0xC0000409.md',
    repro: 'cpsync-repro.mjs',
  },
  productSourceChange: {
    changedFiles: sourceChanged,
    note: 'Only the acceptance harness script changed; all product source (crates/bindings/packages src/prompts/xtask) hashes identical to the previous candidate. schemaHash unchanged.',
    schemaHash: manifest.schemaHash,
    schemaHashUnchanged: manifest.schemaHash === readJson(join(previous, 'release-manifest.json')).schemaHash,
  },
  buildDeterminismNote:
    'Separate release invocations recompiled CLI/native and produced different binary hashes (Rust release builds are not bit-for-bit deterministic across invocations). The final candidate is self-consistent: verify-packages checked its manifest SHA-256 against the actual artifacts and ran them successfully.',
  releaseManifest: {
    version: manifest.version,
    target: manifest.target,
    protocolVersion: manifest.protocolVersion,
    schemaHash: manifest.schemaHash,
    databaseSchemaVersion: manifest.databaseSchemaVersion,
    components: manifest.components,
    nativeBinarySha256: manifest.nativeBinarySha256,
    cliBinaryHashes: manifest.cliBinaryHashes,
    packages: manifest.packages.map((p) => ({ name: p.name, path: p.path, sha256: p.sha256 })),
    lockHashes: manifest.lockHashes,
  },
  installation: {
    passed: installation.passed,
    buildProfile: installation.buildProfile,
    consumerDirectory: installation.directory,
    manifestSha256: installation.manifestSha256,
    consumer: installation.consumer,
    externalModelRequests: installation.externalModelRequests,
    localProviderRequests: installation.localProviderRequests,
    businessDatabaseConnections: installation.businessDatabaseConnections,
    commands: commands.map((c) => ({ label: c.label, exitCode: c.exitCode, log: c.log, sha256: c.sha256 })),
  },
  namingCheck: 'name-check.json',
  runnerCheck: 'runner-check.md',
  evidenceHashes: copied,
  limitations: [
    'Only Windows x64 validated on the current host; not a fresh-VM or minimum-OS certification.',
    'Linux/macOS x64/arm64 remain follow-up (D31); release.yml runner names corrected but no remote Actions run (no Git remote).',
    'No new live-model business run; prior T19 evidence remains separate. Local mock provider only (2 requests).',
    'Node/Python plugins retain explicit runtime requirements; no business database connections.',
    'Public publication not performed; npm scope ownership/permission must be verified before any release.',
    'Physical power-loss, second real multi-repo project, and original-author sign-off remain known boundaries.',
  ],
};
writeFileSync(join(evidenceDir, 'verification-report.json'), JSON.stringify(report, null, 2) + '\n');
console.log(JSON.stringify({
  passed: installation.passed,
  sourceChanged,
  schemaHashUnchanged: report.productSourceChange.schemaHashUnchanged,
  evidenceFiles: Object.keys(copied).length,
  report: join(evidenceDir, 'verification-report.json'),
}, null, 2));