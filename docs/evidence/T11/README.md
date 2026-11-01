# T11 — 2026-10-28

Reproduce: `python docs/evidence/T11/verify.py` from the repository root.
The report records exact commands, exit codes, source/artifact/lock hashes;
tool-versions.json records Rust/Cargo 1.97.1, Node 24.14.0, pnpm 10.34.8 and the audit Python.

125 workspace tests passed. Both standalone Core-executed frontend collectors match
26 frozen expected entities and 20 relations; forbidden hits are zero. Current snapshots,
case results, original source text and evidence references are saved alongside the report.
The frozen truth identifies the A/B layouts as synthetic smoke. No production accuracy
or human sign-off is inferred. Ordinary extraction makes no model requests.

Negative tests cover lexical shadowing, decoys, dynamic client/proxy/URL/title/component,
ambiguous titles, malformed syntax, nested entrypoints and unsupported registrations.
Movement/renaming, lazy imports, inline/i18n titles, aliases and UTF-8 spans are checked.

workspace-map-order-failed.log and plugins-coverage-count-failed.log retain intermediate
verification failures, corrected without changing frozen expected values or product validation.
