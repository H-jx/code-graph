-- Context receipts bind external explanations to Core-selected material.
CREATE TABLE knowledge_contexts (
  input_digest TEXT PRIMARY KEY,
  context_json TEXT NOT NULL CHECK(json_valid(context_json)),
  created_at TEXT NOT NULL
);
CREATE TABLE topic_artifacts (
  topic_id TEXT NOT NULL,
  version INTEGER NOT NULL,
  artifact_json TEXT NOT NULL CHECK(json_valid(artifact_json)),
  context_json TEXT NOT NULL CHECK(json_valid(context_json)),
  PRIMARY KEY(topic_id,version),
  FOREIGN KEY(topic_id,version) REFERENCES topics(topic_id,version)
);
CREATE TABLE conventions (
  id TEXT PRIMARY KEY,
  payload_json TEXT NOT NULL CHECK(json_valid(payload_json)),
  created_at TEXT NOT NULL
);
CREATE TABLE knowledge_export_attempts (
  id TEXT PRIMARY KEY,
  output_path TEXT NOT NULL,
  state TEXT NOT NULL CHECK(state IN ('writing','complete','failed')),
  error_code TEXT,
  created_at TEXT NOT NULL
);
