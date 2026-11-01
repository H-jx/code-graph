# T12 API subloop — doing

Reproduce from the root: `python docs/evidence/T12/verify-api.py`.

Both standalone API artifacts execute through Core. The TS and Python layouts
each match 4 expected entities and 3 registration relations with zero forbidden
decoy bindings. Dynamic path/handler cases remove confirmed bindings and report gaps.
Combined frontend/API staging retains both assertions of the same GraphQL field
under its configured service scope. API snapshots include original evidence bytes.

127 workspace Rust tests, workspace Clippy -D warnings, fmt, frozen source audit and
plan checks pass. Source/artifact/lock/log hashes and exact commands are in
api-verification-report.json. These are synthetic smoke checks, not production accuracy.
The frozen T06 truthHash is unchanged; the API supplement has its own recorded hash.

T12 is incomplete: data source/ORM instance binding, DDL/SQLGlot, queryImplementation,
ordered composite keys and joins still need implementation and acceptance. The
plugin verifier rejects `database`; T13 remains ineligible. No missing gate is waived.
