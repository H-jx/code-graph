//! Knowledge persistence; reads never migrate, refresh facts or invoke a model.
use super::{Database, database_error};
use crate::contract::{
    error::{CoreError, codes},
    knowledge::{TopicArtifact, TopicContext},
};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};

pub fn receipt(connection: &Connection, input: &str) -> Result<TopicContext, CoreError> {
    let text: Option<String> = connection
        .query_row(
            "SELECT context_json FROM knowledge_contexts WHERE input_digest=?1",
            [input],
            |r| r.get(0),
        )
        .optional()
        .map_err(database_error)?;
    serde_json::from_str(&text.ok_or_else(|| {
        CoreError::new(
            codes::REFERENCE_MISSING,
            "export a Core context before importing a topic",
        )
    })?)
    .map_err(|_| CoreError::new(codes::INTERNAL_ERROR, "invalid stored context"))
}
pub fn save_receipt(
    database: &mut Database,
    context: &TopicContext,
    now: &str,
    mut checkpoint: impl FnMut() -> Result<(), CoreError>,
) -> Result<(), CoreError> {
    let tx = database.write_transaction()?;
    checkpoint()?;
    let (id, generation): (String, i64) = tx
        .query_row(
            "SELECT index_id,generation FROM index_meta WHERE id=1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(database_error)?;
    if id != context.snapshot.index_id || generation.to_string() != context.snapshot.generation {
        return Err(CoreError::new(
            codes::SNAPSHOT_EXPIRED,
            "index changed while exporting context",
        ));
    }
    tx.execute(
        "INSERT OR IGNORE INTO knowledge_contexts VALUES(?1,?2,?3)",
        params![
            context.input_digest,
            serde_json::to_string(context).unwrap(),
            now
        ],
    )
    .map_err(database_error)?;
    checkpoint()?;
    tx.commit().map_err(database_error)
}
pub fn save_topic(
    database: &mut Database,
    artifact: &TopicArtifact,
    context: &TopicContext,
    now: &str,
    mut checkpoint: impl FnMut() -> Result<(), CoreError>,
) -> Result<u64, CoreError> {
    let tx = database.write_transaction()?;
    checkpoint()?;
    let generation: i64 = tx
        .query_row("SELECT generation FROM index_meta WHERE id=1", [], |r| {
            r.get(0)
        })
        .map_err(database_error)?;
    if generation.to_string() != artifact.snapshot.generation {
        return Err(CoreError::new(
            codes::SNAPSHOT_EXPIRED,
            "index changed while importing topic",
        ));
    }
    let version: i64 = tx
        .query_row(
            "SELECT COALESCE(MAX(version),0)+1 FROM topics WHERE topic_id=?1",
            [&artifact.topic_id],
            |r| r.get(0),
        )
        .map_err(database_error)?;
    tx.execute("INSERT INTO topics(topic_id,version,title,markdown,snapshot_json,template_hash,prompt_hash,model_info_json,input_digest,refs_hash,state,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,'fresh',?11)",params![artifact.topic_id,version,artifact.title,artifact.markdown,serde_json::to_string(&artifact.snapshot).unwrap(),artifact.template_hash,artifact.prompt_hash,artifact.model_info.as_ref().map(|m|serde_json::to_string(m).unwrap()),artifact.input_digest,crate::knowledge::context::digest(&artifact.source_references),now]).map_err(database_error)?;
    tx.execute(
        "INSERT INTO topic_artifacts VALUES(?1,?2,?3,?4)",
        params![
            artifact.topic_id,
            version,
            serde_json::to_string(artifact).unwrap(),
            serde_json::to_string(context).unwrap()
        ],
    )
    .map_err(database_error)?;
    for dep in &context.dependencies {
        tx.execute(
            "INSERT INTO topic_inputs VALUES(?1,?2,?3,?4)",
            params![
                artifact.topic_id,
                version,
                dep.kind,
                serde_json::to_string(dep).unwrap()
            ],
        )
        .map_err(database_error)?;
    }
    tx.execute(
        "UPDATE knowledge_exports SET state='stale' WHERE state<>'stale'",
        [],
    )
    .map_err(database_error)?;
    checkpoint()?;
    tx.commit().map_err(database_error)?;
    Ok(version as u64)
}
pub struct StoredTopic {
    pub artifact: TopicArtifact,
    pub context: TopicContext,
    pub version: u64,
    pub state: String,
}
pub fn topics(connection: &Connection) -> Result<Vec<StoredTopic>, CoreError> {
    let mut stmt=connection.prepare("SELECT a.artifact_json,a.context_json,t.version,t.state FROM topics t JOIN topic_artifacts a USING(topic_id,version) WHERE t.version=(SELECT MAX(x.version) FROM topics x WHERE x.topic_id=t.topic_id) ORDER BY t.topic_id").map_err(database_error)?;
    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, String>(3)?,
            ))
        })
        .map_err(database_error)?;
    rows.map(|r| {
        let (a, c, v, state) = r.map_err(database_error)?;
        Ok(StoredTopic {
            artifact: serde_json::from_str(&a)
                .map_err(|_| CoreError::new(codes::INTERNAL_ERROR, "invalid stored topic"))?,
            context: serde_json::from_str(&c)
                .map_err(|_| CoreError::new(codes::INTERNAL_ERROR, "invalid stored context"))?,
            version: v as u64,
            state,
        })
    })
    .collect()
}
pub fn mark_stale(connection: &Connection, topic_id: &str, version: u64) -> Result<(), CoreError> {
    connection
        .execute(
            "UPDATE topics SET state='stale' WHERE topic_id=?1 AND version=?2",
            params![topic_id, version as i64],
        )
        .map_err(database_error)?;
    connection
        .execute(
            "UPDATE knowledge_exports SET state='stale' WHERE state<>'stale'",
            [],
        )
        .map_err(database_error)?;
    Ok(())
}
pub fn conventions(connection: &Connection) -> Result<Vec<Value>, CoreError> {
    let mut stmt = connection
        .prepare("SELECT payload_json FROM conventions ORDER BY id")
        .map_err(database_error)?;
    stmt.query_map([], |r| r.get::<_, String>(0))
        .map_err(database_error)?
        .map(|r| {
            serde_json::from_str(&r.map_err(database_error)?)
                .map_err(|_| CoreError::new(codes::INTERNAL_ERROR, "invalid stored convention"))
        })
        .collect()
}
pub fn invalidate_batches(connection: &Connection, keys: &[String]) -> Result<(), CoreError> {
    for topic in topics(connection)? {
        if topic
            .context
            .dependencies
            .iter()
            .any(|d| d.kind == "batch" && keys.contains(&d.key))
        {
            mark_stale(connection, &topic.artifact.topic_id, topic.version)?;
        }
    }
    Ok(())
}
pub fn attempt(connection: &Connection, id: &str, path: &str, now: &str) -> Result<(), CoreError> {
    connection
        .execute(
            "INSERT INTO knowledge_export_attempts VALUES(?1,?2,'writing',NULL,?3)",
            params![id, path, now],
        )
        .map_err(database_error)?;
    Ok(())
}
pub fn finish_attempt(
    connection: &Connection,
    id: &str,
    error: Option<&str>,
) -> Result<(), CoreError> {
    connection
        .execute(
            "UPDATE knowledge_export_attempts SET state=?2,error_code=?3 WHERE id=?1",
            params![
                id,
                if error.is_some() {
                    "failed"
                } else {
                    "complete"
                },
                error
            ],
        )
        .map_err(database_error)?;
    Ok(())
}
pub fn save_export(
    database: &mut Database,
    id: &str,
    path: &str,
    manifest: &Value,
    now: &str,
    mut checkpoint: impl FnMut() -> Result<(), CoreError>,
) -> Result<(), CoreError> {
    let tx = database.write_transaction()?;
    checkpoint()?;
    let state = manifest["freshness"].as_str().unwrap_or("unknown");
    let versions = json!(manifest["topics"]);
    tx.execute(
        "INSERT INTO knowledge_exports VALUES(?1,?2,?3,?4,?5,?6,?7)",
        params![
            id,
            versions.to_string(),
            manifest["snapshot"]["generation"]
                .as_str()
                .unwrap()
                .parse::<i64>()
                .unwrap(),
            crate::knowledge::context::digest(manifest),
            path,
            state,
            now
        ],
    )
    .map_err(database_error)?;
    finish_attempt(&tx, id, None)?;
    checkpoint()?;
    tx.commit().map_err(database_error)
}
