//! 采集输出校验（计划 §3.5、T08）。
//!
//! 校验协议版本、ID 重算、kind 与端点约束、必需业务属性、每条断言的原始证据、
//! 字节与行区间；关系端点只能来自本批或已导出快照；依赖与覆盖声明必须自洽。
//!
//! 任何致命问题都让整组失败：issues 保留精确 `instancePath` 与修复提示。

use std::collections::BTreeMap;

use crate::contract::collector::{CollectorInput, CollectorOutput, PluginManifest, StageSpec};
use crate::contract::entity::{EntityKind, Origin};
use crate::contract::error::{CoreError, Issue, codes};
use crate::contract::evidence::SourceClass;
use crate::contract::extensions::ExtensionRegistry;
use crate::contract::identity::{compute_entity_id, compute_evidence_id, compute_relation_id};
use crate::contract::relation::endpoint_allowed;

/// 校验上下文。
pub struct ValidationContext<'a> {
    pub project_id: &'a str,
    pub input: &'a CollectorInput,
    pub manifest: &'a PluginManifest,
    pub stage: &'a StageSpec,
    pub registry: Option<&'a ExtensionRegistry>,
}

/// 校验结果。
#[derive(Debug, Clone)]
pub struct ValidationOutcome {
    pub issues: Vec<Issue>,
    /// 插件声明的依赖（Core 会补齐自动依赖）。
    pub dependencies: Vec<(String, String, String)>,
}

/// 校验采集输出。
///
/// 返回致命问题时用 `Err`；非致命问题（例如未声明的覆盖）通过 issues 返回。
pub fn validate_output(
    context: &ValidationContext<'_>,
    output: &CollectorOutput,
) -> Result<Vec<Issue>, CoreError> {
    let mut issues: Vec<Issue> = Vec::new();
    let local_registry;
    let registry = if let Some(registry) = context.registry {
        registry
    } else {
        local_registry = ExtensionRegistry::from_manifests([context.manifest])?;
        &local_registry
    };

    for (index, set) in context.input.candidate_sets.iter().enumerate() {
        if !set.complete {
            issues.push(
                Issue::new(
                    codes::CANDIDATE_SET_INCOMPLETE,
                    &format!("/input/candidateSets/{index}"),
                )
                .with_hint("重新导出完整候选集合，不能对截断结果判定唯一匹配"),
            );
        }
    }
    for selector in &context.stage.entity_selectors {
        let sets: Vec<_> = context
            .input
            .candidate_sets
            .iter()
            .filter(|set| set.selector_id == selector.id)
            .collect();
        let expected: std::collections::BTreeSet<_> = context
            .input
            .entities
            .iter()
            .filter(|entity| {
                (selector.kinds.is_empty() || selector.kinds.contains(&entity.kind))
                    && (selector.scope_ids.is_empty()
                        || selector.scope_ids.contains(&entity.scope_id))
            })
            .map(|entity| &entity.entity_id)
            .collect();
        if sets.len() != 1
            || sets.first().is_some_and(|set| {
                set.entity_ids
                    .iter()
                    .collect::<std::collections::BTreeSet<_>>()
                    != expected
                    || set.entity_ids.len() != expected.len()
            })
        {
            issues.push(
                Issue::new(codes::CANDIDATE_SET_INCOMPLETE, "/input/candidateSets")
                    .with_hint("每个声明的 selector 必须有唯一、完整且与导出实体一致的候选集合"),
            );
        }
    }

    if output.protocol_version != crate::contract::PROTOCOL_VERSION {
        return Err(CoreError::new(
            codes::PROTOCOL_UNSUPPORTED,
            format!(
                "collector returned protocol version `{}`, expected `{}`",
                output.protocol_version,
                crate::contract::PROTOCOL_VERSION
            ),
        ));
    }
    if output.run_id != context.input.run_id {
        issues.push(
            Issue::new(codes::SCHEMA_INVALID, "/runId")
                .with_actual(output.run_id.clone())
                .with_expected(context.input.run_id.clone()),
        );
    }
    if output.stage_id != context.input.stage_id {
        issues.push(
            Issue::new(codes::SCHEMA_INVALID, "/stageId")
                .with_actual(output.stage_id.clone())
                .with_expected(context.input.stage_id.clone()),
        );
    }
    if output.input_snapshot_id != context.input.input_snapshot_id {
        issues.push(
            Issue::new(codes::SCHEMA_INVALID, "/inputSnapshotId")
                .with_actual(output.input_snapshot_id.clone())
                .with_expected(context.input.input_snapshot_id.clone()),
        );
    }

    // 证据索引：来源版本必须在输入快照内，且字节/行区间合法。
    let mut evidence_index: BTreeMap<String, EvidenceRecord> = BTreeMap::new();
    for (index, evidence) in output.evidence.iter().enumerate() {
        let path = format!("/evidence/{index}");
        let Some(source) = context
            .input
            .sources
            .iter()
            .find(|source| source.source.id == evidence.source_id)
        else {
            issues.push(
                Issue::new(codes::REFERENCE_MISSING, &path)
                    .with_actual(evidence.source_id.clone())
                    .with_hint("证据必须指向本次输入中的来源"),
            );
            continue;
        };
        if source.source.content_hash != evidence.content_hash {
            issues.push(
                Issue::new(codes::EVIDENCE_INVALID, &path)
                    .with_actual(evidence.content_hash.clone())
                    .with_expected(source.source.content_hash.clone())
                    .with_hint("证据必须使用来源的当前内容版本"),
            );
            continue;
        }
        if evidence.end_byte <= evidence.start_byte {
            issues.push(Issue::new(codes::EVIDENCE_INVALID, &path).with_hint("证据区间必须非空"));
            continue;
        }
        let bytes = source.content.as_bytes();
        if evidence.end_byte > bytes.len() as u64 {
            issues.push(
                Issue::new(codes::EVIDENCE_INVALID, &path)
                    .with_actual(evidence.end_byte.to_string())
                    .with_expected(bytes.len().to_string())
                    .with_hint("证据字节区间超出来源长度"),
            );
            continue;
        }
        let line_map = crate::project::snapshot::LineMap::build(bytes);
        let expected_start = line_map.line_of(evidence.start_byte);
        let expected_end = line_map.end_line_of(evidence.start_byte, evidence.end_byte);
        if expected_start != evidence.start_line || expected_end != evidence.end_line {
            issues.push(
                Issue::new(codes::EVIDENCE_INVALID, &path)
                    .with_actual(format!("{}:{}", evidence.start_line, evidence.end_line))
                    .with_expected(format!("{expected_start}:{expected_end}"))
                    .with_hint("证据行号必须与字节区间一致"),
            );
            continue;
        }
        if !source
            .content
            .is_char_boundary(evidence.start_byte as usize)
            || !source.content.is_char_boundary(evidence.end_byte as usize)
            || !source.source.source_class.is_original()
            || evidence.id
                != compute_evidence_id(
                    &evidence.source_id,
                    &evidence.content_hash,
                    evidence.start_byte,
                    evidence.end_byte,
                    &evidence.role,
                )
            || evidence_index.contains_key(&evidence.id)
        {
            issues.push(
                Issue::new(codes::EVIDENCE_INVALID, &path)
                    .with_hint("使用原始来源、UTF-8 字符边界与重算的唯一证据 ID"),
            );
            continue;
        }
        evidence_index.insert(
            evidence.id.clone(),
            EvidenceRecord {
                source_id: evidence.source_id.clone(),
                content_hash: evidence.content_hash.clone(),
                source_class: source.source.source_class,
            },
        );
    }

    // 实体：ID 重算、类型、作用域、证据。
    let mut batch_entity_ids: BTreeMap<String, EntityKind> = BTreeMap::new();
    for (index, entity) in output.entities.iter().enumerate() {
        let path = format!("/entities/{index}");
        if let Some(scope) = entity.identity.iter().find(|slot| slot.name == "scopeId")
            && scope.value.as_ref().and_then(|v| v.as_text()) != Some(entity.scope_id.as_str())
        {
            issues.push(
                Issue::new(codes::IDENTITY_CONFLICT, &format!("{path}/scopeId"))
                    .with_hint("scopeId 必须与 identity 中的作用域一致"),
            );
        }
        let attributes = serde_json::Value::Object(entity.assertion.attributes.clone());
        let payload = match entity.kind {
            EntityKind::QueryImplementation => {
                crate::contract::schema::validate::<crate::contract::QueryImplementationAttributes>(
                    &attributes,
                    &format!("{path}/assertion/attributes"),
                )
            }
            EntityKind::ReportField => crate::contract::schema::validate::<
                crate::contract::ReportFieldAttributes,
            >(
                &attributes, &format!("{path}/assertion/attributes")
            ),
            EntityKind::Extension(ref name) => registry.validate_entity(
                name.as_str(),
                &entity.identity,
                &entity.assertion.attributes,
                &path,
            ),
            _ => Ok(()),
        };
        if let Err(error) = payload {
            issues.extend(error.issues);
        }
        let recomputed =
            match compute_entity_id(context.project_id, entity.kind.clone(), &entity.identity) {
                Ok(id) => id,
                Err(error) => {
                    // 保留此前 kind/来源的根因；非法身份不能丢弃已收集的精确 issue。
                    issues.push(
                        Issue::new(codes::IDENTITY_CONFLICT, &path).with_actual(error.to_string()),
                    );
                    continue;
                }
            };
        if recomputed != entity.entity_id {
            issues.push(
                Issue::new(codes::IDENTITY_CONFLICT, &path)
                    .with_actual(entity.entity_id.clone())
                    .with_expected(recomputed)
                    .with_hint("Core 必须重算实体 ID；请按槽位顺序构造 identity"),
            );
        }
        if entity.assertion.origin == Origin::Ai
            && entity.assertion.basis != crate::contract::entity::Basis::Inferred
        {
            issues.push(
                Issue::new(codes::SCHEMA_INVALID, &path)
                    .with_actual("ai origin with non-inferred basis")
                    .with_hint("AI 判断必须使用 inferred 依据"),
            );
        }
        if entity.assertion.evidence_refs.is_empty() {
            issues.push(
                Issue::new(codes::EVIDENCE_INVALID, &path)
                    .with_hint("每个实体断言至少需要一个原始证据"),
            );
        }
        for reference in &entity.assertion.evidence_refs {
            match evidence_index.get(reference) {
                None => issues.push(
                    Issue::new(codes::REFERENCE_MISSING, &path)
                        .with_actual(reference.clone())
                        .with_hint("证据引用必须指向本次输出中的证据"),
                ),
                Some(record) => {
                    if !record.source_class.is_original() {
                        issues.push(
                            Issue::new(codes::EVIDENCE_INVALID, &path)
                                .with_actual(record.source_class.as_str().to_string())
                                .with_hint("生成知识不能作为原始证据"),
                        );
                    }
                }
            }
        }
        if batch_entity_ids
            .insert(entity.entity_id.clone(), entity.kind.clone())
            .is_some()
        {
            issues.push(
                Issue::new(codes::IDENTITY_CONFLICT, &path).with_hint("单批实体 ID 不可重复"),
            );
        }
    }

    // 关系：端点与证据。
    let mut seen_relations = std::collections::BTreeSet::new();
    for (index, relation) in output.relations.iter().enumerate() {
        let path = format!("/relations/{index}");
        if !seen_relations.insert(&relation.relation_id) {
            issues.push(
                Issue::new(codes::IDENTITY_CONFLICT, &path).with_hint("单批关系 ID 不可重复"),
            );
        }
        let qualifiers = serde_json::Value::Object(relation.qualifiers.clone());
        let payload = match relation.kind {
            crate::contract::RelationKind::Joins => crate::contract::schema::validate::<
                crate::contract::JoinQualifiers,
            >(
                &qualifiers, &format!("{path}/qualifiers")
            ),
            crate::contract::RelationKind::TransitionsTo => {
                crate::contract::schema::validate::<crate::contract::TransitionQualifiers>(
                    &qualifiers,
                    &format!("{path}/qualifiers"),
                )
            }
            _ => Ok(()),
        };
        if let Err(error) = payload {
            issues.extend(error.issues);
        }
        let from_kind = batch_entity_ids
            .get(&relation.from_id)
            .cloned()
            .or_else(|| exported_kind(context, &relation.from_id));
        let to_kind = batch_entity_ids
            .get(&relation.to_id)
            .cloned()
            .or_else(|| exported_kind(context, &relation.to_id));
        let (Some(from_kind), Some(to_kind)) = (from_kind, to_kind) else {
            issues.push(
                Issue::new(codes::REFERENCE_MISSING, &path)
                    .with_actual(format!("{} -> {}", relation.from_id, relation.to_id))
                    .with_hint("关系端点必须来自本批或已导出快照"),
            );
            continue;
        };
        let expected_id = compute_relation_id(
            context.project_id,
            relation.kind.clone(),
            &relation.from_id,
            &relation.to_id,
            &relation.qualifiers,
        )
        .map_err(|e| CoreError::new(codes::IDENTITY_CONFLICT, e.to_string()))?;
        if relation.relation_id != expected_id {
            issues.push(
                Issue::new(codes::IDENTITY_CONFLICT, &path)
                    .with_expected(expected_id)
                    .with_hint("按关系类型、端点和 qualifiers 重算关系 ID"),
            );
        }
        if relation.assertion.origin == Origin::Ai
            && relation.assertion.basis != crate::contract::entity::Basis::Inferred
        {
            issues.push(
                Issue::new(codes::SCHEMA_INVALID, &path).with_hint("AI 关系必须使用 inferred"),
            );
        }
        if let crate::contract::RelationKind::Extension(name) = &relation.kind {
            if let Err(error) = registry.validate_relation(
                name.as_str(),
                &from_kind,
                &to_kind,
                &relation.qualifiers,
                &path,
            ) {
                issues.extend(error.issues);
            }
        } else if !endpoint_allowed(relation.kind.clone(), from_kind.as_str(), to_kind.as_str()) {
            issues.push(
                Issue::new(codes::SCHEMA_INVALID, &path)
                    .with_actual(format!(
                        "{}: {} -> {}",
                        relation.kind.as_str(),
                        from_kind.as_str(),
                        to_kind.as_str()
                    ))
                    .with_hint("关系端点类型不在该关系类型的允许组合内"),
            );
        }
        if relation.assertion.evidence_refs.is_empty() {
            issues.push(
                Issue::new(codes::EVIDENCE_INVALID, &path)
                    .with_hint("关系断言必须有覆盖实际绑定的证据，不能只借两端实体的位置"),
            );
        }
        for reference in &relation.assertion.evidence_refs {
            if !evidence_index.contains_key(reference) {
                issues.push(
                    Issue::new(codes::REFERENCE_MISSING, &path)
                        .with_actual(reference.clone())
                        .with_hint("证据引用必须指向本次输出中的证据"),
                );
            }
        }
    }

    // 依赖：必须有 kind、key 与 digest。
    for (index, dependency) in output.dependencies.iter().enumerate() {
        if dependency.key.trim().is_empty() || dependency.digest.trim().is_empty() {
            issues.push(
                Issue::new(codes::SCHEMA_INVALID, &format!("/dependencies/{index}"))
                    .with_hint("依赖必须声明 key 与 digest"),
            );
        }
    }

    // 缺口：必须有原因。
    for (index, gap) in output.gaps.iter().enumerate() {
        if gap.reason.trim().is_empty() {
            issues.push(
                Issue::new(codes::SCHEMA_INVALID, &format!("/gaps/{index}"))
                    .with_hint("缺口必须说明原因"),
            );
        }
    }

    // 覆盖：声明的能力必须在本插件的 capabilities 内。
    for (index, coverage) in output.coverage.iter().enumerate() {
        if coverage.status == crate::contract::collector::CoverageStatus::Complete
            && (context.input.source_sets.iter().any(|s| !s.complete)
                || !coverage.gap_ids.is_empty()
                || output.gaps.iter().any(|g| {
                    g.capability == coverage.capability && g.scope_id == coverage.scope_id
                }))
        {
            issues.push(
                Issue::new(codes::SCHEMA_INVALID, &format!("/coverage/{index}/status"))
                    .with_hint("存在跳过来源或对应缺口时只能声明 partial"),
            );
        }
        for gap in &coverage.gap_ids {
            if !output.gaps.iter().any(|g| &g.id == gap) {
                issues.push(Issue::new(
                    codes::REFERENCE_MISSING,
                    &format!("/coverage/{index}/gapIds"),
                ));
            }
        }
        if !context.manifest.capabilities.is_empty()
            && !context
                .manifest
                .capabilities
                .iter()
                .any(|capability| capability == &coverage.capability)
        {
            issues.push(
                Issue::new(
                    codes::SCHEMA_INVALID,
                    &format!("/coverage/{index}/capability"),
                )
                .with_actual(coverage.capability.clone())
                .with_hint("覆盖声明的能力必须出现在 manifest 的 capabilities 中"),
            );
        }
    }

    if issues.is_empty() {
        Ok(issues)
    } else {
        // 信封错误码取第一个问题的类别：问题按根因顺序收集，后续问题往往是它的级联。
        let code = issues[0].code.clone();
        Err(CoreError::new(
            &code,
            format!("collector output has {} invalid field(s)", issues.len()),
        )
        .with_issues(issues))
    }
}

/// 证据记录。
struct EvidenceRecord {
    #[allow(dead_code)]
    source_id: String,
    #[allow(dead_code)]
    content_hash: String,
    source_class: SourceClass,
}

/// 已导出实体的类型。
fn exported_kind(context: &ValidationContext<'_>, entity_id: &str) -> Option<EntityKind> {
    context
        .input
        .entities
        .iter()
        .find(|entity| entity.entity_id == entity_id)
        .map(|entity| entity.kind.clone())
}
