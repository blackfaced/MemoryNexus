use std::fs;

#[test]
fn knowledge_refresh_migration_defines_bounded_contract_tables() {
    let migration = fs::read_to_string("migrations/019_knowledge_refresh.sql")
        .expect("knowledge refresh migration should exist");

    for table in [
        "knowledge_acquisition_traces",
        "knowledge_source_candidates",
        "knowledge_source_policies",
        "knowledge_contexts",
    ] {
        assert!(
            migration.contains(&format!("CREATE TABLE {table}")),
            "migration should define {table}"
        );
    }

    for state in [
        "proposed",
        "approved",
        "rejected",
        "expired",
        "active",
        "paused",
        "revoked",
        "candidate",
        "valid",
    ] {
        assert!(
            migration.contains(state),
            "migration should enumerate state {state}"
        );
    }

    for forbidden in [
        "full_source_document",
        "raw_provider_payload",
        "crawler_state",
        "external_search_index",
        "credential",
        "signed_url",
    ] {
        assert!(
            !migration.contains(forbidden),
            "bounded V1 schema must not add storage for {forbidden}"
        );
    }
}

#[test]
fn knowledge_refresh_uses_existing_capture_and_observation_surfaces() {
    let surface_domain =
        fs::read_to_string("src/domain/surface.rs").expect("surface domain should be readable");
    let surface_api =
        fs::read_to_string("src/api/surfaces.rs").expect("surface api should be readable");

    assert!(
        !surface_domain.contains("Knowledge"),
        "issue #199 must not add a Knowledge Surface"
    );
    assert!(
        surface_api.contains("knowledge_source_candidate"),
        "Capture Surface should accept KnowledgeSourceCandidateInput"
    );
    assert!(
        surface_api.contains("knowledge_context"),
        "Capture Surface should accept KnowledgeContextInput"
    );
    assert!(
        surface_api.contains("knowledge_refresh"),
        "Observation Surface should expose shaped knowledge refresh state"
    );
}
