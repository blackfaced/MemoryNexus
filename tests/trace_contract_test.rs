use std::fs;

use chrono::Utc;
use memorynexus::db::trace::{
    CreateCompletedTrace, TraceListFilter, TraceMode, TraceRuntime, TraceSourceType, TraceStatus,
    TraceTaskType,
};
use serde_json::json;
use uuid::Uuid;

#[test]
fn trace_migration_scopes_rows_to_space_and_optional_namespace() {
    let sql =
        fs::read_to_string("migrations/014_traces.sql").expect("trace migration should exist");

    assert!(sql.contains("CREATE TABLE IF NOT EXISTS traces"));
    assert!(
        sql.contains("space_id UUID NOT NULL REFERENCES cognitive_spaces(id) ON DELETE CASCADE")
    );
    assert!(sql.contains("namespace_id UUID"));
    assert!(sql.contains("FOREIGN KEY (namespace_id, space_id)"));
    assert!(sql.contains("REFERENCES namespaces(id, space_id)"));
    assert!(sql.contains("ON DELETE SET NULL (namespace_id)"));
}

#[test]
fn trace_migration_defines_runtime_contract_and_indexes() {
    let sql =
        fs::read_to_string("migrations/014_traces.sql").expect("trace migration should exist");

    for column in [
        "source_type VARCHAR(50) NOT NULL",
        "task_type VARCHAR(50) NOT NULL",
        "mode VARCHAR(50) NOT NULL",
        "runtime VARCHAR(50) NOT NULL",
        "input_summary TEXT",
        "output_summary TEXT",
        "started_at TIMESTAMPTZ NOT NULL DEFAULT NOW()",
        "completed_at TIMESTAMPTZ",
        "latency_ms BIGINT",
        "status VARCHAR(50) NOT NULL DEFAULT 'completed'",
        "token_usage JSONB",
        "estimated_cost_usd DOUBLE PRECISION",
        "local_processing_ratio DOUBLE PRECISION",
        "metadata JSONB NOT NULL DEFAULT '{}'::jsonb",
    ] {
        assert!(sql.contains(column), "missing trace column: {column}");
    }

    for check in [
        "traces_source_type_check",
        "traces_task_type_check",
        "traces_mode_check",
        "traces_runtime_check",
        "traces_status_check",
        "traces_completed_status_check",
        "traces_latency_non_negative_check",
        "traces_local_processing_ratio_check",
    ] {
        assert!(sql.contains(check), "missing trace constraint: {check}");
    }

    for index in [
        "idx_traces_space_id",
        "idx_traces_namespace_id",
        "idx_traces_task_type",
        "idx_traces_mode",
        "idx_traces_runtime",
        "idx_traces_status",
        "idx_traces_started_at",
        "idx_traces_space_started_at",
    ] {
        assert!(sql.contains(index), "missing trace index: {index}");
    }
}

#[test]
fn trace_migration_includes_first_phase_generated_object_links() {
    let sql =
        fs::read_to_string("migrations/014_traces.sql").expect("trace migration should exist");

    for column in [
        "related_memory_ids UUID[] NOT NULL DEFAULT '{}'",
        "generated_memory_ids UUID[] NOT NULL DEFAULT '{}'",
        "generated_lens_run_ids UUID[] NOT NULL DEFAULT '{}'",
        "generated_review_report_ids UUID[] NOT NULL DEFAULT '{}'",
        "generated_feedback_loop_ids UUID[] NOT NULL DEFAULT '{}'",
        "generated_reflection_ids UUID[] NOT NULL DEFAULT '{}'",
        "generated_sleep_cycle_ids UUID[] NOT NULL DEFAULT '{}'",
        "generated_consolidation_result_ids UUID[] NOT NULL DEFAULT '{}'",
        "generated_dream_candidate_ids UUID[] NOT NULL DEFAULT '{}'",
    ] {
        assert!(
            sql.contains(column),
            "missing generated link column: {column}"
        );
    }
}

#[test]
fn trace_enums_accept_only_contract_values() {
    assert_eq!(
        serde_json::from_str::<TraceSourceType>("\"test_fixture\"").unwrap(),
        TraceSourceType::TestFixture
    );
    assert_eq!(
        serde_json::from_str::<TraceTaskType>("\"lens_run\"").unwrap(),
        TraceTaskType::LensRun
    );
    assert_eq!(
        serde_json::from_str::<TraceMode>("\"focused\"").unwrap(),
        TraceMode::Focused
    );
    assert_eq!(
        serde_json::from_str::<TraceRuntime>("\"deterministic\"").unwrap(),
        TraceRuntime::Deterministic
    );
    assert_eq!(
        serde_json::from_str::<TraceStatus>("\"completed\"").unwrap(),
        TraceStatus::Completed
    );

    assert!(serde_json::from_str::<TraceSourceType>("\"browser\"").is_err());
    assert!(serde_json::from_str::<TraceTaskType>("\"fine_tune\"").is_err());
    assert!(serde_json::from_str::<TraceMode>("\"system2\"").is_err());
    assert!(serde_json::from_str::<TraceRuntime>("\"gpu\"").is_err());
    assert!(serde_json::from_str::<TraceStatus>("\"retrying\"").is_err());
}

#[test]
fn create_completed_trace_keeps_summaries_and_generated_links() {
    let space_id = Uuid::new_v4();
    let namespace_id = Uuid::new_v4();
    let memory_id = Uuid::new_v4();
    let review_report_id = Uuid::new_v4();
    let feedback_loop_id = Uuid::new_v4();

    let trace = CreateCompletedTrace {
        space_id,
        namespace_id: Some(namespace_id),
        source_type: TraceSourceType::Ui,
        task_type: TraceTaskType::Review,
        mode: TraceMode::Deep,
        runtime: TraceRuntime::Deterministic,
        input_summary: Some("redacted weekly review request".to_string()),
        output_summary: Some("redacted recurring mistake pattern summary".to_string()),
        started_at: Utc::now(),
        completed_at: Utc::now(),
        latency_ms: Some(125),
        model_provider: Some("deterministic".to_string()),
        model_name: None,
        token_usage: Some(json!({"input": 0, "output": 0, "total": 0})),
        estimated_cost_usd: Some(0.0),
        local_processing_ratio: Some(1.0),
        related_memory_ids: vec![memory_id],
        generated_memory_ids: vec![],
        generated_lens_run_ids: vec![],
        generated_review_report_ids: vec![review_report_id],
        generated_feedback_loop_ids: vec![feedback_loop_id],
        generated_reflection_ids: vec![],
        generated_sleep_cycle_ids: vec![],
        generated_consolidation_result_ids: vec![],
        generated_dream_candidate_ids: vec![],
        user_feedback: None,
        error: None,
        metadata: json!({"redaction": "summary_only"}),
    };

    assert_eq!(trace.space_id, space_id);
    assert_eq!(trace.namespace_id, Some(namespace_id));
    assert_eq!(trace.related_memory_ids, vec![memory_id]);
    assert_eq!(trace.generated_review_report_ids, vec![review_report_id]);
    assert_eq!(trace.generated_feedback_loop_ids, vec![feedback_loop_id]);
    assert_eq!(
        trace.output_summary.as_deref(),
        Some("redacted recurring mistake pattern summary")
    );
}

#[test]
fn trace_repository_uses_space_membership_for_get_and_list() {
    let repository = fs::read_to_string("src/db/trace.rs").expect("trace repository should exist");

    assert!(repository.contains("INNER JOIN cognitive_space_members m ON m.space_id = t.space_id"));
    assert!(repository.contains("WHERE t.id = $1 AND m.user_id = $2"));
    assert!(repository.contains("WHERE t.space_id = $1"));
    assert!(repository.contains("AND m.user_id = $2"));
}

#[test]
fn trace_list_filter_is_space_scoped() {
    let space_id = Uuid::new_v4();
    let filter = TraceListFilter {
        space_id,
        namespace_id: None,
        task_type: Some(TraceTaskType::Practice),
        mode: Some(TraceMode::Focused),
        runtime: Some(TraceRuntime::Local),
        status: Some(TraceStatus::Completed),
        limit: 20,
        offset: 0,
    };

    assert_eq!(filter.space_id, space_id);
    assert_eq!(filter.namespace_id, None);
    assert_eq!(filter.task_type, Some(TraceTaskType::Practice));
    assert_eq!(filter.mode, Some(TraceMode::Focused));
    assert_eq!(filter.runtime, Some(TraceRuntime::Local));
    assert_eq!(filter.status, Some(TraceStatus::Completed));
}
