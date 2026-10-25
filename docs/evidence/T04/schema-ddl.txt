-- Project Knowledge Core：初始 schema（计划 §3.4）。
--
-- 约定：
-- - 一个项目一个库；SDK/MCP 不建立平行库。
-- - 所有 `*_json` 列都带 `CHECK(json_valid(...))`。
-- - 枚举列用 CHECK 约束；引用一律显式外键。
-- - `assertions` 是实体断言与关系断言的公共父表；子表 id 为 PK 且外键指向父表，
--   `batch_id` 必须与父表一致（由触发器保证）。

PRAGMA foreign_keys = ON;

CREATE TABLE schema_migrations (
  version     INTEGER PRIMARY KEY,
  checksum    TEXT    NOT NULL,
  applied_at  TEXT    NOT NULL
);

CREATE TABLE index_meta (
  id                        INTEGER PRIMARY KEY CHECK (id = 1),
  index_id                  TEXT    NOT NULL UNIQUE,
  project_id                TEXT    NOT NULL,
  generation                INTEGER NOT NULL DEFAULT 0 CHECK (generation >= 0),
  current_source_snapshot_id TEXT,
  config_hash               TEXT    NOT NULL,
  created_at                TEXT    NOT NULL
);

CREATE TABLE repositories (
  id   TEXT PRIMARY KEY,
  root TEXT NOT NULL UNIQUE
);

CREATE TABLE scopes (
  id   TEXT PRIMARY KEY,
  kind TEXT NOT NULL CHECK (kind IN ('application', 'module', 'service'))
);

CREATE TABLE scope_roots (
  scope_id      TEXT NOT NULL REFERENCES scopes(id),
  repository_id TEXT NOT NULL REFERENCES repositories(id),
  path          TEXT NOT NULL,
  PRIMARY KEY (scope_id, repository_id, path)
);

CREATE TABLE services (
  id              TEXT PRIMARY KEY,
  scope_id        TEXT NOT NULL REFERENCES scopes(id),
  protocol        TEXT NOT NULL,
  base_urls_json  TEXT NOT NULL CHECK (json_valid(base_urls_json))
);

CREATE TABLE data_sources (
  id           TEXT PRIMARY KEY,
  scope_id     TEXT NOT NULL REFERENCES scopes(id),
  dialect      TEXT NOT NULL,
  catalog      TEXT,
  schema_name  TEXT
);

CREATE TABLE project_bindings (
  id           TEXT PRIMARY KEY,
  kind         TEXT NOT NULL CHECK (kind IN ('clientToService', 'proxyToService', 'ormInstanceToDataSource')),
  scope_id     TEXT NOT NULL REFERENCES scopes(id),
  from_key     TEXT NOT NULL,
  to_key       TEXT NOT NULL,
  evidence_json TEXT NOT NULL CHECK (json_valid(evidence_json))
);

CREATE TABLE sources (
  id               TEXT PRIMARY KEY,
  repository_id    TEXT NOT NULL REFERENCES repositories(id),
  path             TEXT NOT NULL,
  current_hash     TEXT,
  selection_status TEXT NOT NULL CHECK (selection_status IN ('selected', 'excluded', 'skipped')),
  UNIQUE (repository_id, path)
);

CREATE TABLE source_versions (
  source_id    TEXT    NOT NULL REFERENCES sources(id),
  content_hash TEXT    NOT NULL,
  content      BLOB    NOT NULL,
  byte_length  INTEGER NOT NULL CHECK (byte_length >= 0),
  line_map_json TEXT  NOT NULL CHECK (json_valid(line_map_json)),
  source_class TEXT    NOT NULL CHECK (source_class IN ('code', 'config', 'schema', 'documentation', 'derivedConvention', 'generatedKnowledge')),
  created_at   TEXT    NOT NULL,
  PRIMARY KEY (source_id, content_hash)
);

CREATE TABLE source_snapshots (
  id             TEXT PRIMARY KEY,
  config_hash    TEXT NOT NULL,
  source_set_hash TEXT NOT NULL,
  created_at     TEXT NOT NULL
);

CREATE TABLE snapshot_sources (
  snapshot_id      TEXT NOT NULL REFERENCES source_snapshots(id),
  source_id        TEXT NOT NULL REFERENCES sources(id),
  content_hash     TEXT,
  selection_status TEXT NOT NULL CHECK (selection_status IN ('selected', 'excluded', 'skipped')),
  PRIMARY KEY (snapshot_id, source_id)
);

CREATE TABLE plugin_versions (
  plugin_id     TEXT NOT NULL,
  version       TEXT NOT NULL,
  artifact_hash TEXT NOT NULL,
  manifest_json TEXT NOT NULL CHECK (json_valid(manifest_json)),
  runtime_digest TEXT NOT NULL,
  artifact_path TEXT NOT NULL,
  validation_id TEXT,
  created_at    TEXT NOT NULL,
  PRIMARY KEY (plugin_id, version)
);

CREATE TABLE active_plugins (
  plugin_id          TEXT PRIMARY KEY,
  version            TEXT NOT NULL,
  activation_revision INTEGER NOT NULL,
  FOREIGN KEY (plugin_id, version) REFERENCES plugin_versions(plugin_id, version)
);

CREATE TABLE runs (
  id                TEXT PRIMARY KEY,
  kind              TEXT NOT NULL,
  stage             TEXT NOT NULL,
  status            TEXT NOT NULL CHECK (status IN ('pending', 'running', 'succeeded', 'partial', 'failed', 'cancelled', 'interrupted')),
  input_snapshot_id TEXT,
  started_at        TEXT NOT NULL,
  finished_at       TEXT,
  error_json        TEXT CHECK (error_json IS NULL OR json_valid(error_json))
);

CREATE TABLE batches (
  id                   TEXT PRIMARY KEY,
  run_id               TEXT NOT NULL REFERENCES runs(id),
  producer_key         TEXT NOT NULL,
  replacement_key      TEXT NOT NULL,
  artifact_hash        TEXT NOT NULL,
  input_digest         TEXT NOT NULL,
  state                TEXT NOT NULL CHECK (state IN ('fresh', 'stale', 'unknown')),
  invalidated_generation INTEGER,
  created_at           TEXT NOT NULL
);

CREATE TABLE current_batches (
  replacement_key TEXT PRIMARY KEY,
  batch_id        TEXT NOT NULL UNIQUE REFERENCES batches(id)
);

CREATE TABLE assertions (
  id           TEXT PRIMARY KEY,
  kind         TEXT NOT NULL CHECK (kind IN ('entity', 'relation')),
  batch_id     TEXT NOT NULL REFERENCES batches(id),
  payload_hash TEXT NOT NULL,
  producer_key TEXT NOT NULL,
  basis        TEXT NOT NULL CHECK (basis IN ('observed', 'declared', 'inferred')),
  origin       TEXT NOT NULL CHECK (origin IN ('static', 'human', 'ai')),
  UNIQUE (batch_id, kind, payload_hash)
);

CREATE TABLE entities (
  id           TEXT PRIMARY KEY,
  kind         TEXT NOT NULL,
  scope_id     TEXT NOT NULL REFERENCES scopes(id),
  identity_json TEXT NOT NULL CHECK (json_valid(identity_json)),
  UNIQUE (identity_json)
);

CREATE TABLE entity_assertions (
  id             TEXT PRIMARY KEY,
  entity_id      TEXT NOT NULL REFERENCES entities(id),
  batch_id       TEXT NOT NULL REFERENCES batches(id),
  basis          TEXT NOT NULL CHECK (basis IN ('observed', 'declared', 'inferred')),
  origin         TEXT NOT NULL CHECK (origin IN ('static', 'human', 'ai')),
  name           TEXT,
  attributes_json TEXT NOT NULL CHECK (json_valid(attributes_json)),
  payload_hash   TEXT NOT NULL,
  UNIQUE (batch_id, entity_id, payload_hash),
  FOREIGN KEY (id) REFERENCES assertions(id) DEFERRABLE INITIALLY DEFERRED
);

CREATE TABLE relations (
  id              TEXT PRIMARY KEY,
  kind            TEXT NOT NULL,
  from_id         TEXT NOT NULL REFERENCES entities(id),
  to_id           TEXT NOT NULL REFERENCES entities(id),
  qualifiers_json TEXT NOT NULL CHECK (json_valid(qualifiers_json))
);

CREATE TABLE relation_assertions (
  id             TEXT PRIMARY KEY,
  relation_id    TEXT NOT NULL REFERENCES relations(id),
  batch_id       TEXT NOT NULL REFERENCES batches(id),
  basis          TEXT NOT NULL CHECK (basis IN ('observed', 'declared', 'inferred')),
  origin         TEXT NOT NULL CHECK (origin IN ('static', 'human', 'ai')),
  resolution     TEXT NOT NULL CHECK (resolution IN ('confirmed', 'candidate')),
  payload_hash   TEXT NOT NULL,
  UNIQUE (batch_id, relation_id, payload_hash),
  FOREIGN KEY (id) REFERENCES assertions(id) DEFERRABLE INITIALLY DEFERRED
);

CREATE TABLE evidence (
  id           TEXT PRIMARY KEY,
  source_id    TEXT NOT NULL,
  content_hash TEXT NOT NULL,
  start_byte   INTEGER NOT NULL CHECK (start_byte >= 0),
  end_byte     INTEGER NOT NULL CHECK (end_byte > start_byte),
  start_line   INTEGER NOT NULL CHECK (start_line >= 1),
  end_line     INTEGER NOT NULL CHECK (end_line >= start_line),
  role         TEXT NOT NULL,
  FOREIGN KEY (source_id, content_hash) REFERENCES source_versions(source_id, content_hash)
);

CREATE TABLE assertion_evidence (
  assertion_id TEXT NOT NULL REFERENCES assertions(id),
  evidence_id  TEXT NOT NULL REFERENCES evidence(id),
  PRIMARY KEY (assertion_id, evidence_id)
);

CREATE TABLE dependencies (
  batch_id TEXT NOT NULL REFERENCES batches(id),
  kind     TEXT NOT NULL CHECK (kind IN ('source', 'sourceSet', 'config', 'artifact', 'runtime', 'candidateSet', 'upstreamBatch', 'prompt', 'topicInput')),
  key      TEXT NOT NULL,
  digest   TEXT NOT NULL,
  PRIMARY KEY (batch_id, kind, key)
);

CREATE TABLE selectors (
  id              TEXT PRIMARY KEY,
  definition_json TEXT NOT NULL CHECK (json_valid(definition_json)),
  kind            TEXT NOT NULL CHECK (kind IN ('sourceSet', 'candidateSet')),
  last_digest     TEXT NOT NULL
);

CREATE TABLE coverage (
  batch_id       TEXT NOT NULL REFERENCES batches(id),
  capability     TEXT NOT NULL,
  scope_id       TEXT NOT NULL,
  source_set_id  TEXT NOT NULL,
  status         TEXT NOT NULL CHECK (status IN ('complete', 'partial', 'not_configured', 'failed')),
  counts_json    TEXT NOT NULL CHECK (json_valid(counts_json)),
  gap_ids_json   TEXT NOT NULL CHECK (json_valid(gap_ids_json)),
  PRIMARY KEY (batch_id, capability, scope_id, source_set_id)
);

CREATE TABLE gaps (
  id           TEXT PRIMARY KEY,
  batch_id     TEXT NOT NULL REFERENCES batches(id),
  code         TEXT NOT NULL,
  payload_json TEXT NOT NULL CHECK (json_valid(payload_json))
);

CREATE TABLE search_documents (
  rowid          INTEGER PRIMARY KEY AUTOINCREMENT,
  assertion_id   TEXT NOT NULL REFERENCES assertions(id),
  entity_id      TEXT NOT NULL REFERENCES entities(id),
  scope_id       TEXT NOT NULL,
  route_exact    TEXT,
  text           TEXT NOT NULL,
  tokens         TEXT NOT NULL,
  projection_kind TEXT NOT NULL,
  UNIQUE (assertion_id, projection_kind)
);

CREATE VIRTUAL TABLE search_fts USING fts5(tokens, tokenize = 'unicode61');

CREATE TABLE validations (
  id                  TEXT PRIMARY KEY,
  artifact_hash       TEXT NOT NULL,
  input_digest        TEXT NOT NULL,
  active_version_map_json TEXT NOT NULL CHECK (json_valid(active_version_map_json)),
  candidate_set_digest TEXT NOT NULL,
  truth_hash          TEXT NOT NULL,
  status              TEXT NOT NULL CHECK (status IN ('pending', 'passed', 'failed')),
  staged_result_hash  TEXT,
  created_at          TEXT NOT NULL
);

CREATE TABLE activations (
  id                TEXT PRIMARY KEY,
  expected_generation INTEGER NOT NULL,
  group_json        TEXT NOT NULL CHECK (json_valid(group_json)),
  status            TEXT NOT NULL CHECK (status IN ('pending', 'committed', 'failed')),
  run_id            TEXT,
  created_at        TEXT NOT NULL
);

CREATE TABLE topics (
  topic_id        TEXT NOT NULL,
  version         INTEGER NOT NULL,
  title           TEXT NOT NULL,
  markdown        TEXT NOT NULL,
  snapshot_json   TEXT NOT NULL CHECK (json_valid(snapshot_json)),
  template_hash   TEXT NOT NULL,
  prompt_hash     TEXT NOT NULL,
  model_info_json TEXT CHECK (model_info_json IS NULL OR json_valid(model_info_json)),
  input_digest    TEXT NOT NULL,
  refs_hash       TEXT NOT NULL,
  state           TEXT NOT NULL CHECK (state IN ('fresh', 'stale', 'unknown')),
  created_at      TEXT NOT NULL,
  PRIMARY KEY (topic_id, version)
);

CREATE TABLE topic_inputs (
  topic_id TEXT NOT NULL,
  version  INTEGER NOT NULL,
  kind     TEXT NOT NULL,
  ref      TEXT NOT NULL,
  PRIMARY KEY (topic_id, version, kind, ref)
);

CREATE TABLE knowledge_exports (
  id                   TEXT PRIMARY KEY,
  topic_versions_json  TEXT NOT NULL CHECK (json_valid(topic_versions_json)),
  generation           INTEGER NOT NULL,
  output_hash          TEXT NOT NULL,
  output_path          TEXT NOT NULL,
  state                TEXT NOT NULL CHECK (state IN ('fresh', 'stale', 'unknown')),
  created_at           TEXT NOT NULL
);

-- 必要索引（计划 §3.4）。
CREATE INDEX idx_entity_assertions_entity_batch ON entity_assertions (entity_id, batch_id);
CREATE INDEX idx_assertions_batch_kind        ON assertions (batch_id, kind);
CREATE INDEX idx_relations_from                ON relations (from_id, kind, to_id);
CREATE INDEX idx_relations_to                  ON relations (to_id, kind, from_id);
CREATE INDEX idx_relation_assertions_lookup    ON relation_assertions (relation_id, batch_id, resolution, basis);
CREATE INDEX idx_entities_scope_kind           ON entities (scope_id, kind, id);
CREATE INDEX idx_dependencies_lookup           ON dependencies (kind, key, batch_id);
CREATE INDEX idx_search_documents_lookup        ON search_documents (scope_id, route_exact, entity_id);
CREATE INDEX idx_evidence_source               ON evidence (source_id, content_hash, start_byte);
CREATE INDEX idx_runs_status                   ON runs (status, started_at);
CREATE INDEX idx_batches_replacement           ON batches (replacement_key, state);

-- 断言父表与子表的一致性：batch_id 必须一致，且只能有一个子类型。
CREATE TRIGGER trg_entity_assertions_batch
BEFORE INSERT ON entity_assertions
BEGIN
  SELECT CASE
    WHEN (SELECT batch_id FROM assertions WHERE id = NEW.id) IS NOT NEW.batch_id
    THEN RAISE(ABORT, 'entity_assertions.batch_id must match assertions.batch_id')
  END;
  SELECT CASE
    WHEN EXISTS (SELECT 1 FROM relation_assertions WHERE id = NEW.id)
    THEN RAISE(ABORT, 'assertion cannot be both entity and relation')
  END;
END;

CREATE TRIGGER trg_relation_assertions_batch
BEFORE INSERT ON relation_assertions
BEGIN
  SELECT CASE
    WHEN (SELECT batch_id FROM assertions WHERE id = NEW.id) IS NOT NEW.batch_id
    THEN RAISE(ABORT, 'relation_assertions.batch_id must match assertions.batch_id')
  END;
  SELECT CASE
    WHEN EXISTS (SELECT 1 FROM entity_assertions WHERE id = NEW.id)
    THEN RAISE(ABORT, 'assertion cannot be both entity and relation')
  END;
END;
