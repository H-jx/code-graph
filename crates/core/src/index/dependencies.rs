//! 文件、配置、脚本与候选集合依赖（计划 §3.6）。
//!
//! 依赖记录由 Core 计算或复核，插件不能自报任意新鲜值。失效是单向的：批次一旦
//! 失效就不会因为来源内容改回、候选失败或版本回退而恢复 fresh。

use std::collections::{BTreeMap, BTreeSet};

use rusqlite::Connection;

use crate::contract::error::{CoreError, codes};
use crate::storage;

/// 依赖种类标签。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DependencyKindTag {
    Source,
    SourceSet,
    Config,
    Artifact,
    Runtime,
    CandidateSet,
    UpstreamBatch,
    Prompt,
    TopicInput,
}

impl DependencyKindTag {
    pub fn as_str(&self) -> &'static str {
        match self {
            DependencyKindTag::Source => "source",
            DependencyKindTag::SourceSet => "sourceSet",
            DependencyKindTag::Config => "config",
            DependencyKindTag::Artifact => "artifact",
            DependencyKindTag::Runtime => "runtime",
            DependencyKindTag::CandidateSet => "candidateSet",
            DependencyKindTag::UpstreamBatch => "upstreamBatch",
            DependencyKindTag::Prompt => "prompt",
            DependencyKindTag::TopicInput => "topicInput",
        }
    }

    pub fn parse(value: &str) -> Option<DependencyKindTag> {
        match value {
            "source" => Some(DependencyKindTag::Source),
            "sourceSet" => Some(DependencyKindTag::SourceSet),
            "config" => Some(DependencyKindTag::Config),
            "artifact" => Some(DependencyKindTag::Artifact),
            "runtime" => Some(DependencyKindTag::Runtime),
            "candidateSet" => Some(DependencyKindTag::CandidateSet),
            "upstreamBatch" => Some(DependencyKindTag::UpstreamBatch),
            "prompt" => Some(DependencyKindTag::Prompt),
            "topicInput" => Some(DependencyKindTag::TopicInput),
            _ => None,
        }
    }
}

/// 一条依赖。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyChange {
    pub kind: DependencyKindTag,
    pub key: String,
    pub digest: String,
}

/// 为批次登记依赖。
pub fn record_batch_dependencies(
    connection: &Connection,
    batch_id: &str,
    dependencies: &[DependencyChange],
) -> Result<(), CoreError> {
    for dependency in dependencies {
        storage::insert_dependency(
            connection,
            batch_id,
            dependency.kind.as_str(),
            &dependency.key,
            &dependency.digest,
        )?;
    }
    Ok(())
}

/// 依赖反向闭包：给定变化的依赖，找出所有受影响的批次与替换范围。
///
/// 返回 `(batch_id, replacement_key)` 列表。依赖关系沿 `upstreamBatch` 反向传播：
/// 上游批次变化时，依赖它的下游批次同样受影响。
pub fn dependency_closure(
    connection: &Connection,
    changes: &[DependencyChange],
) -> Result<Vec<(String, String)>, CoreError> {
    let mut affected: BTreeSet<String> = BTreeSet::new();
    let mut frontier: Vec<String> = Vec::new();

    for change in changes {
        let mut statement = connection
            .prepare(
                "SELECT b.id, b.replacement_key
                   FROM dependencies d
                   JOIN batches b ON b.id = d.batch_id
                  WHERE d.kind = ?1 AND d.key = ?2
                    AND (?3 = '' OR d.digest <> ?3)",
            )
            .map_err(|error| {
                CoreError::new(
                    codes::INTERNAL_ERROR,
                    format!("cannot prepare dependency query: {error}"),
                )
            })?;
        let rows = statement
            .query_map(
                rusqlite::params![change.kind.as_str(), change.key, change.digest],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .map_err(|error| {
                CoreError::new(
                    codes::INTERNAL_ERROR,
                    format!("cannot query dependencies: {error}"),
                )
            })?;
        for row in rows {
            let (batch_id, replacement_key) = row.map_err(|error| {
                CoreError::new(
                    codes::INTERNAL_ERROR,
                    format!("cannot read dependency row: {error}"),
                )
            })?;
            if affected.insert(batch_id.clone()) {
                frontier.push(batch_id);
            }
            let _ = replacement_key;
        }
    }

    // 沿上游批次关系继续扩展。
    let mut index = 0usize;
    while index < frontier.len() {
        let upstream = frontier[index].clone();
        index += 1;
        let mut statement = connection
            .prepare(
                "SELECT d.batch_id FROM dependencies d
                  WHERE d.kind = 'upstreamBatch' AND d.key = ?1",
            )
            .map_err(|error| {
                CoreError::new(
                    codes::INTERNAL_ERROR,
                    format!("cannot prepare upstream query: {error}"),
                )
            })?;
        let rows = statement
            .query_map(rusqlite::params![upstream], |row| row.get::<_, String>(0))
            .map_err(|error| {
                CoreError::new(
                    codes::INTERNAL_ERROR,
                    format!("cannot query upstream batches: {error}"),
                )
            })?;
        for row in rows {
            let downstream = row.map_err(|error| {
                CoreError::new(
                    codes::INTERNAL_ERROR,
                    format!("cannot read upstream row: {error}"),
                )
            })?;
            if affected.insert(downstream.clone()) {
                frontier.push(downstream);
            }
        }
    }

    let mut result: Vec<(String, String)> = Vec::new();
    for batch_id in affected {
        let replacement_key: String = connection
            .query_row(
                "SELECT replacement_key FROM batches WHERE id = ?1",
                rusqlite::params![batch_id],
                |row| row.get(0),
            )
            .map_err(|error| {
                CoreError::new(
                    codes::INTERNAL_ERROR,
                    format!("cannot read batch replacement key: {error}"),
                )
            })?;
        result.push((batch_id, replacement_key));
    }
    result.sort();
    Ok(result)
}

/// 读取批次的依赖集合（用于比较新旧依赖）。
pub fn batch_dependencies(
    connection: &Connection,
    batch_id: &str,
) -> Result<BTreeMap<(String, String), String>, CoreError> {
    let mut result = BTreeMap::new();
    for (kind, key, digest) in storage::load_dependencies(connection, batch_id)? {
        result.insert((kind, key), digest);
    }
    Ok(result)
}

/// 当前批次表（替换范围 → 批次）。
pub fn current_batch_map(connection: &Connection) -> Result<BTreeMap<String, String>, CoreError> {
    let mut result = BTreeMap::new();
    for (replacement_key, batch_id) in storage::current_batches(connection)? {
        result.insert(replacement_key, batch_id);
    }
    Ok(result)
}
