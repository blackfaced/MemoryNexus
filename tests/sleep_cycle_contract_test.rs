use std::fs;

use chrono::{Duration, Utc};
use memorynexus::db::sleep_cycles::{
    CompleteSleepCycle, CreateSleepCycle, FailSleepCycle, SleepCycleRepository,
};
use memorynexus::domain::sleep_cycle::{SleepCycleStatus, SleepCycleType};
use uuid::Uuid;

#[test]
fn sleep_cycle_migration_scopes_rows_to_space_and_optional_namespace() {
    let sql = fs::read_to_string("migrations/016_sleep_cycles.sql")
        .expect("sleep cycle migration should exist");

    assert!(sql.contains("CREATE TABLE IF NOT EXISTS sleep_cycles"));
    assert!(sql.contains("space_id UUID NOT NULL REFERENCES cognitive_spaces(id)"));
    assert!(sql.contains("namespace_id UUID"));
    assert!(sql.contains("FOREIGN KEY (namespace_id, space_id)"));
    assert!(sql.contains("REFERENCES namespaces(id, space_id)"));
    assert!(!sql.contains("CREATE TABLE IF NOT EXISTS namespaces"));
}

#[test]
fn sleep_cycle_migration_defines_lifecycle_window_and_links() {
    let sql = fs::read_to_string("migrations/016_sleep_cycles.sql")
        .expect("sleep cycle migration should exist");

    for column in [
        "cycle_type VARCHAR(50) NOT NULL",
        "status VARCHAR(50) NOT NULL DEFAULT 'pending'",
        "evidence_window_start TIMESTAMPTZ NOT NULL",
        "evidence_window_end TIMESTAMPTZ NOT NULL",
        "input_trace_ids UUID[] NOT NULL DEFAULT '{}'",
        "input_memory_ids UUID[] NOT NULL DEFAULT '{}'",
        "input_feedback_loop_ids UUID[] NOT NULL DEFAULT '{}'",
        "input_review_report_ids UUID[] NOT NULL DEFAULT '{}'",
        "generated_memory_ids UUID[] NOT NULL DEFAULT '{}'",
        "triggering_trace_id UUID",
        "error TEXT",
        "started_at TIMESTAMPTZ",
        "completed_at TIMESTAMPTZ",
        "metadata JSONB NOT NULL DEFAULT '{}'::jsonb",
    ] {
        assert!(sql.contains(column), "missing column contract: {column}");
    }

    assert!(sql.contains("sleep_cycles_cycle_type_check"));
    assert!(sql.contains("cycle_type IN ('daily', 'weekly', 'manual')"));
    assert!(sql.contains("sleep_cycles_status_check"));
    assert!(sql.contains("status IN ('pending', 'running', 'completed', 'failed', 'cancelled')"));
    assert!(sql.contains("sleep_cycles_window_check"));
    assert!(sql.contains("evidence_window_start < evidence_window_end"));
    assert!(!sql.contains("generated_consolidation_result_ids"));
    assert!(!sql.contains("generated_dream_candidate_ids"));
    assert!(!sql.contains("generated_cognitive_scene_ids"));
    assert!(!sql.contains("updated_cognitive_scene_ids"));
    assert!(!sql.contains("updated_growth_model_ids"));
}

#[test]
fn sleep_cycle_domain_serializes_contract_values() {
    assert_eq!(SleepCycleType::Daily.to_string(), "daily");
    assert_eq!(SleepCycleType::Weekly.to_string(), "weekly");
    assert_eq!(SleepCycleType::Manual.to_string(), "manual");

    assert_eq!(SleepCycleStatus::Pending.to_string(), "pending");
    assert_eq!(SleepCycleStatus::Running.to_string(), "running");
    assert_eq!(SleepCycleStatus::Completed.to_string(), "completed");
    assert_eq!(SleepCycleStatus::Failed.to_string(), "failed");
    assert_eq!(SleepCycleStatus::Cancelled.to_string(), "cancelled");
}

#[test]
fn create_sleep_cycle_keeps_evidence_window_and_links() {
    let space_id = Uuid::new_v4();
    let namespace_id = Uuid::new_v4();
    let window_start = Utc::now() - Duration::days(1);
    let window_end = Utc::now();
    let input_trace_id = Uuid::new_v4();
    let input_feedback_loop_id = Uuid::new_v4();

    let cycle = CreateSleepCycle {
        space_id,
        namespace_id: Some(namespace_id),
        cycle_type: SleepCycleType::Manual,
        status: SleepCycleStatus::Pending,
        evidence_window_start: window_start,
        evidence_window_end: window_end,
        input_trace_ids: vec![input_trace_id],
        input_memory_ids: Vec::new(),
        input_feedback_loop_ids: vec![input_feedback_loop_id],
        input_review_report_ids: Vec::new(),
        triggering_trace_id: None,
        metadata: serde_json::json!({"reason": "manual test"}),
    };

    assert_eq!(cycle.space_id, space_id);
    assert_eq!(cycle.namespace_id, Some(namespace_id));
    assert_eq!(cycle.evidence_window_start, window_start);
    assert_eq!(cycle.evidence_window_end, window_end);
    assert_eq!(cycle.input_trace_ids, vec![input_trace_id]);
    assert_eq!(cycle.input_feedback_loop_ids, vec![input_feedback_loop_id]);
}

#[test]
fn sleep_cycle_repository_exposes_create_completed_and_failed_lifecycle() {
    fn assert_trait<T: SleepCycleRepository>() {}
    assert_trait::<memorynexus::db::sleep_cycles::PostgresSleepCycleRepository>();

    let completed = CompleteSleepCycle {
        generated_memory_ids: vec![Uuid::new_v4()],
        metadata: serde_json::json!({"summary": "local deterministic"}),
    };
    let failed = FailSleepCycle {
        error: "same_space_validation_failed".to_string(),
        metadata: serde_json::json!({"stage": "validation"}),
    };

    assert_eq!(completed.generated_memory_ids.len(), 1);
    assert_eq!(failed.error, "same_space_validation_failed");
}

#[test]
fn sleep_cycle_repository_validates_linked_namespace_same_space() {
    let repository =
        fs::read_to_string("src/db/sleep_cycles.rs").expect("repository should be readable");

    assert!(repository.contains("validate_sleep_cycle_same_space_links"));
    assert!(repository.contains("validate_namespace_same_space"));
    assert!(repository.contains("validate_uuid_array_same_space"));
    assert!(repository.contains("FROM namespaces"));
    assert!(repository.contains("WHERE id = $1 AND space_id = $2"));
    assert!(repository.contains("FROM traces"));
    assert!(repository.contains("FROM memories"));
    assert!(repository.contains("FROM feedback_loops"));
    assert!(repository.contains("FROM cognitive_review_reports"));
    assert!(repository.contains("validate_optional_trace_same_space"));
    assert!(repository.contains("unique_uuid_count"));
}
