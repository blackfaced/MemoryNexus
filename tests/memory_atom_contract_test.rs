use chrono::{TimeZone, Utc};
use memorynexus::domain::memory_atom::{
    MemoryAtom, MemoryAtomKind, MemoryAtomLifecycleState, MemoryAtomProvenance,
    MemoryAtomProvenanceMethod,
};
use uuid::Uuid;

#[test]
fn memory_atom_json_round_trip_preserves_space_namespace_and_provenance() {
    let space_id = Uuid::new_v4();
    let namespace_id = Uuid::new_v4();
    let first_memory_id = Uuid::new_v4();
    let second_memory_id = Uuid::new_v4();
    let capture_trace_id = Uuid::new_v4();
    let consolidation_trace_id = Uuid::new_v4();

    let atom = MemoryAtom {
        id: Uuid::new_v4(),
        space_id,
        namespace_id: Some(namespace_id),
        source_memory_ids: vec![first_memory_id, second_memory_id],
        kind: MemoryAtomKind::PatternSignal,
        content: "Double-letter spelling errors recur in recent dictation attempts".to_string(),
        confidence: 82,
        salience: 71,
        state: MemoryAtomLifecycleState::Candidate,
        provenance: MemoryAtomProvenance {
            source_trace_ids: vec![capture_trace_id, consolidation_trace_id],
            method: MemoryAtomProvenanceMethod::Fixture,
            extractor: Some("memory_atom_contract_test".to_string()),
            rationale: Some("fixture-only draft preserving source evidence".to_string()),
        },
        created_at: Utc.with_ymd_and_hms(2026, 7, 1, 9, 30, 0).unwrap(),
        updated_at: Utc.with_ymd_and_hms(2026, 7, 1, 9, 45, 0).unwrap(),
    };

    let json = serde_json::to_string(&atom).expect("MemoryAtom should serialize");
    let parsed: MemoryAtom =
        serde_json::from_str(&json).expect("MemoryAtom should deserialize from its own JSON");

    assert_eq!(parsed, atom);
    assert_eq!(parsed.space_id, space_id);
    assert_eq!(parsed.namespace_id, Some(namespace_id));
    assert_eq!(
        parsed.source_memory_ids,
        vec![first_memory_id, second_memory_id]
    );
    assert_eq!(
        parsed.provenance.source_trace_ids,
        vec![capture_trace_id, consolidation_trace_id]
    );
}

#[test]
fn memory_atom_kind_and_lifecycle_state_use_stable_snake_case_json() {
    let kind = serde_json::to_value(MemoryAtomKind::PracticeSignal)
        .expect("MemoryAtomKind should serialize");
    let state = serde_json::to_value(MemoryAtomLifecycleState::Accepted)
        .expect("MemoryAtomLifecycleState should serialize");
    let method = serde_json::to_value(MemoryAtomProvenanceMethod::DeterministicRule)
        .expect("MemoryAtomProvenanceMethod should serialize");

    assert_eq!(kind, "practice_signal");
    assert_eq!(state, "accepted");
    assert_eq!(method, "deterministic_rule");

    assert_eq!(
        serde_json::from_value::<MemoryAtomKind>(kind).unwrap(),
        MemoryAtomKind::PracticeSignal
    );
    assert_eq!(
        serde_json::from_value::<MemoryAtomLifecycleState>(state).unwrap(),
        MemoryAtomLifecycleState::Accepted
    );
    assert_eq!(
        serde_json::from_value::<MemoryAtomProvenanceMethod>(method).unwrap(),
        MemoryAtomProvenanceMethod::DeterministicRule
    );
}
