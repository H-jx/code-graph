# TS API example (T12 API subloop)

The standalone manifest parses the configured `http.METHOD(path, handler)` and
`graphql.register(type, field, resolver)` registrations with TypeScript 6.0.3.
Only a proven local or named relative import binds a handler. Service ownership
must be unique in the explicitly configured source scope. Duplicate names in
other files are separate operations; dynamic paths/handlers return gaps.

Prepare/check packaging with `node scripts/package-api-examples.mjs [--check]`.
Run the Core-driven suite with `pnpm run test:plugins -- --capabilities api`.
The bundled case copies the separate source-reviewed synthetic API supplement;
Core marks candidate-supplied cases as candidate. The original frozen T06 package
is unchanged. SDK/native/Core contain no project-specific API registration rules.

Database/model/SQLGlot/queryImplementation collection is pending in T12.
This example does not support arbitrary decorators, classes, external package
re-exports or dynamic runtime registration. It never imports or executes business code.
