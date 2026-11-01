// Requirement-to-executable mapping, checked against source and actual test output.
const core = (suite, name, file) => ({
  package: 'project-knowledge-core',
  suite,
  name,
  file: file ?? `crates/core/tests/${suite}.rs`,
});
const agent = (name) => ({
  package: 'project-knowledge-agent',
  suite: 'agent_lifecycle',
  name,
  file: 'crates/agent/tests/agent_lifecycle.rs',
});
const scenario = core(
  'business_scenarios',
  'scenario_queries_two_layouts_match_original_facts_and_cli',
  'crates/core/tests/support/scenarios.rs',
);
const semantics = core(
  'business_scenarios',
  'semantics_workflow_report_preserve_original_conflicts_and_all_field_lineage',
  'crates/core/tests/support/semantics.rs',
);
const knowledge = (name) => core('indexing', name, 'crates/core/tests/support/knowledge.rs');
const query = (name) => core('indexing', name, 'crates/core/tests/support/queries.rs');
export const acceptanceCases = Object.fromEntries([
  ...[1, 2, 3].flatMap((group) =>
    Array.from({ length: 7 }, (_, i) => [
      `S${group}-${String(i + 1).padStart(2, '0')}`,
      [scenario],
    ]),
  ),
  ['S2-06', [scenario, semantics]],
  [
    'S2-07',
    [
      core(
        'lifecycle',
        'incremental_candidates_zero_one_two_rename_attributes_delete_and_clean_rebuild_agree',
      ),
      knowledge('knowledge_plugin_fact_and_candidate_sets_invalidate_zero_matches'),
    ],
  ],
  [
    'S3-04',
    [core('business_scenarios', 'truth_numeric_sql_matches_frozen_rows_and_rejects_both_mutants')],
  ],
  [
    'S3-07',
    [
      core(
        'lifecycle',
        'incremental_failure_cancel_base_only_and_restored_content_keep_old_batches_stale',
      ),
      knowledge(
        'knowledge_relevant_sql_stales_rerender_does_not_refresh_and_unrelated_generation_survives',
      ),
    ],
  ],
  [
    'X01',
    [
      core(
        'business_scenarios',
        'linking_two_layouts_use_original_bindings_and_reject_missing_or_incomplete_proof',
        'crates/core/tests/support/linking.rs',
      ),
    ],
  ],
  [
    'X02',
    [
      core(
        'business_scenarios',
        'backend_database_two_asts_preserve_datasources_ddl_keys_query_joins_and_original_ranges',
      ),
    ],
  ],
  [
    'X03',
    [
      core(
        'business_scenarios',
        'backend_database_incomplete_keys_dynamic_instances_and_invalid_sql_never_confirm_mappings',
      ),
    ],
  ],
  [
    'X04',
    [
      core(
        'business_scenarios',
        'frontend_moves_renames_and_ambiguous_dynamic_registrations_remain_conservative',
      ),
    ],
  ],
  [
    'X05',
    [
      core(
        'business_scenarios',
        'linking_new_resolver_proxy_and_datasource_changes_match_clean_rebuild',
        'crates/core/tests/support/linking.rs',
      ),
    ],
  ],
  [
    'X06',
    [core('collectors', 'validation_empty_extract_invalidates_downstream_and_preserves_base')],
  ],
  [
    'X07',
    [
      core(
        'lifecycle',
        'incremental_candidates_zero_one_two_rename_attributes_delete_and_clean_rebuild_agree',
      ),
    ],
  ],
  [
    'X08',
    [
      core('collectors', 'runner_rejects_non_zero_exit_and_invalid_json'),
      core('collectors', 'runner_bounds_blocked_input_and_simultaneous_output_floods'),
    ],
  ],
  [
    'X09',
    [
      core('lifecycle', 'activation_concurrent_candidates_and_readers_never_mix_versions'),
      core('collectors', 'validation_commit_rechecks_source_artifact_and_generation'),
    ],
  ],
  [
    'X10',
    [core('lifecycle', 'activation_real_host_kill_inside_c_preserves_old_snapshot_and_retries')],
  ],
  [
    'X11',
    [
      core(
        'lifecycle',
        'incremental_failure_cancel_base_only_and_restored_content_keep_old_batches_stale',
      ),
    ],
  ],
  [
    'X12',
    [
      query('query_neighbors_bfs_cycles_filters_and_strict_budgets'),
      query('query_read_source_rejects_forgery_and_preserves_expired_versions_explicitly'),
    ],
  ],
  ['X13', [query('query_search_keyset_fields_unicode_generation_and_readonly')]],
  [
    'X14',
    [
      knowledge('knowledge_rejects_forged_cyclic_and_unmarked_explanations'),
      core('collectors', 'validation_rejects_generated_knowledge_as_evidence'),
    ],
  ],
  [
    'X15',
    [
      agent('lifecycle_stops_after_two_failed_repairs'),
      agent('lifecycle_pending_activation_and_skill_partial'),
      agent('config_provider_errors_cancellation_timeout_and_turn_budget'),
    ],
  ],
  [
    'X16',
    [
      core('collectors', 'deps_plan_requires_lock_file_and_reports_missing_runtime'),
      agent('config_missing_key_invalid_response_and_input_budget_fail_closed'),
    ],
  ],
]);
