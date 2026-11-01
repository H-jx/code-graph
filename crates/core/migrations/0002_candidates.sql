CREATE TABLE plugin_candidates (
  id TEXT PRIMARY KEY,
  plugin_id TEXT NOT NULL,
  version TEXT NOT NULL,
  artifact_hash TEXT NOT NULL,
  directory TEXT NOT NULL,
  state TEXT NOT NULL CHECK (state IN ('generated','prepared','validated','published','active','failed','superseded')),
  validation_id TEXT REFERENCES validations(id),
  staged_json TEXT CHECK (staged_json IS NULL OR json_valid(staged_json)),
  error_json TEXT CHECK (error_json IS NULL OR json_valid(error_json)),
  created_at TEXT NOT NULL
);
CREATE INDEX plugin_candidates_identity ON plugin_candidates(plugin_id, version, artifact_hash);
