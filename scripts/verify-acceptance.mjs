import assert from 'node:assert/strict';
import { readFileSync, writeFileSync, readdirSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { resolve, join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { acceptanceCases } from './acceptance-cases.mjs';
const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const out = join(root, 'docs/evidence/T22');
const records = JSON.parse(readFileSync(join(out, 'commands.json')));
const workspace = records.filter((r) => r.name === 'workspace').at(-1);
assert.equal(workspace.exitCode, 0);
const log = readFileSync(join(out, workspace.log), 'utf8');
const cases = Object.entries(acceptanceCases).map(([id, checks]) => {
  for (const check of checks)
    assert.ok(
      log.includes(check.name + ' ... ok'),
      `${id}: no passed current-workspace result for ${check.name}`,
    );
  return { id, passed: true, checks };
});
const scenarios = records.filter((r) => r.name === 'scenarios').at(-1);
assert.equal(scenarios.exitCode, 0);
const scenarioDirectory = resolve(root, scenarios.evidenceDirectory);
const comparisons = readdirSync(join(scenarioDirectory, 'transports'))
  .filter((f) => f.endsWith('.json'))
  .map((file) => {
    const bytes = readFileSync(join(scenarioDirectory, 'transports', file));
    const value = JSON.parse(bytes);
    assert.equal(value.passed, true);
    return {
      file,
      operation: value.request.operation,
      snapshot: value.cli.snapshot,
      sha256: createHash('sha256').update(bytes).digest('hex'),
    };
  });
assert.ok(comparisons.length >= 6);
const truth = JSON.parse(readFileSync(join(root, 'fixtures/truth/frontend.json')));
const frontendTest =
  'frontend_two_layouts_match_frozen_identities_attributes_relations_and_evidence';
const cohorts = truth.layouts.map((layout) => ({
  layout: layout.layout,
  expectedEntities: layout.expectedEntities.length,
  expectedRelations: layout.expectedRelations.length,
  forbiddenRelations: layout.forbiddenRelations.length,
  forbiddenConfirmed: 0,
  expectedEntityRecall: 1,
  expectedRelationRecall: 1,
  precisionWithinExpectedForbiddenPairs: 1,
  assertionSource: frontendTest,
}));
// Counts describe the frozen expected/forbidden population, not every possible edge in a project.
const passedFrontend = log.includes(frontendTest + ' ... ok');
assert.ok(passedFrontend, 'Original frontend expected/forbidden acceptance must have run');
const sourceReview = JSON.parse(readFileSync(join(out, 'source-review.json')));
assert.equal(sourceReview.passed, true);
const scenarioReport = JSON.parse(readFileSync(join(scenarioDirectory, 'cli-report.json')));
const report = {
  passed: true,
  platform: process.platform,
  cases,
  caseCount: cases.length,
  comparisonCount: comparisons.length,
  comparisons,
  cohorts,
  originalSourceReview: {
    facts: sourceReview.facts.length,
    files: sourceReview.files.length,
    currentMatchingFrozen: sourceReview.currentFilesMatchingFrozen,
  },
  oracle: scenarioReport.oracle,
  truthHash: scenarioReport.truthHash,
  limitations: [
    'Expected/forbidden cohorts cover source-reviewed synthetic layouts; no whole-project precision percentage',
    'Real-source citations sampled separately; no production accounting claim',
    'Host tests only; process/filesystem behavior on other platforms requires its own run',
  ],
};
writeFileSync(join(out, 'acceptance-report.json'), JSON.stringify(report, null, 2) + '\n');
console.log(
  JSON.stringify({
    passed: true,
    cases: cases.length,
    comparisons: comparisons.length,
    originalFacts: sourceReview.facts.length,
    cohorts,
  }),
);
