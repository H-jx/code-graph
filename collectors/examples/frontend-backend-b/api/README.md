# Python API example (T12 API subloop)

Requires Python 3.12; uses stdlib `ast` without importing business modules.
The configured patterns are `app.METHOD(path)(handler)` and
`graphql.register(type, field, resolver)` with top-level functions and relative
imports. Python AST byte columns map to exact UTF-8 evidence spans. Unknown paths,
handlers or service ownership yield gaps. Same-named unimported functions are not bound.

Packaging: `node scripts/package-api-examples.mjs [--check]`.
Core execution/negative cases: `pnpm run test:plugins -- --capabilities api`.
The protocol and identity specification are copied into the standalone artifact;
Python identity encoding is the existing independently verified T02 implementation.
No project node_modules, Python application dependencies or business runtime are loaded.

Database, ORM instances, SQL and complex language constructs remain T12 work.
The API fixture is synthetic smoke and does not establish production accuracy.
