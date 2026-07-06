use chrono::{TimeZone, Utc};
use memorynexus::domain::cognitive_scene::{
    CognitiveScene, CognitiveSceneLifecycleState, CognitiveSceneProvenance,
    CognitiveSceneProvenanceMethod, CognitiveSceneType, CognitiveSceneValidationError,
};
use memorynexus::domain::memory_atom::{
    MemoryAtom, MemoryAtomKind, MemoryAtomLifecycleState, MemoryAtomProvenance,
    MemoryAtomProvenanceMethod,
};
use serde_json::json;
use uuid::Uuid;

fn atom_in_space(space_id: Uuid, namespace_id: Option<Uuid>) -> MemoryAtom {
    MemoryAtom {
        id: Uuid::new_v4(),
        space_id,
        namespace_id,
        source_memory_ids: vec![Uuid::new_v4()],
        kind: MemoryAtomKind::PracticeSignal,
        content: "Double-letter spelling mistakes keep recurring.".to_string(),
        confidence: 82,
        salience: 74,
        state: MemoryAtomLifecycleState::Accepted,
        provenance: MemoryAtomProvenance {
            source_trace_ids: vec![Uuid::new_v4()],
            method: MemoryAtomProvenanceMethod::Fixture,
            extractor: Some("cognitive_scene_contract_test".to_string()),
            rationale: Some("fixture-only source atom".to_string()),
        },
        created_at: Utc.with_ymd_and_hms(2026, 7, 2, 9, 0, 0).unwrap(),
        updated_at: Utc.with_ymd_and_hms(2026, 7, 2, 9, 5, 0).unwrap(),
    }
}

#[test]
fn cognitive_scene_serializes_stable_contract_shape() {
    let space_id = Uuid::new_v4();
    let namespace_id = Uuid::new_v4();
    let first_atom_id = Uuid::new_v4();
    let second_atom_id = Uuid::new_v4();
    let trace_id = Uuid::new_v4();

    let scene = CognitiveScene {
        id: Uuid::new_v4(),
        space_id,
        namespace_id: Some(namespace_id),
        scene_type: CognitiveSceneType::PracticeField,
        title: "Double-letter spelling stability".to_string(),
        source_atom_ids: vec![first_atom_id, second_atom_id],
        summary: "Recent dictation attempts show a recurring double-letter pattern.".to_string(),
        active_patterns: vec![
            "double_letter_omission".to_string(),
            "self_correction_after_feedback".to_string(),
        ],
        state: CognitiveSceneLifecycleState::Candidate,
        provenance: CognitiveSceneProvenance {
            source_trace_ids: vec![trace_id],
            method: CognitiveSceneProvenanceMethod::Fixture,
            builder: Some("cognitive_scene_contract_test".to_string()),
            rationale: Some("domain draft with explicit source atoms".to_string()),
        },
        created_at: Utc.with_ymd_and_hms(2026, 7, 2, 10, 0, 0).unwrap(),
        updated_at: Utc.with_ymd_and_hms(2026, 7, 2, 10, 10, 0).unwrap(),
    };

    let value = serde_json::to_value(&scene).expect("CognitiveScene should serialize");

    assert_eq!(value["space_id"], json!(space_id));
    assert_eq!(value["namespace_id"], json!(namespace_id));
    assert_eq!(value["scene_type"], "practice_field");
    assert_eq!(
        value["source_atom_ids"],
        json!([first_atom_id, second_atom_id])
    );
    assert_eq!(value["state"], "candidate");
    assert_eq!(value["provenance"]["source_trace_ids"], json!([trace_id]));
    assert_eq!(value["provenance"]["method"], "fixture");

    let round_trip: CognitiveScene =
        serde_json::from_value(value).expect("CognitiveScene should deserialize");
    assert_eq!(round_trip.scene_type, CognitiveSceneType::PracticeField);
    assert_eq!(round_trip.state, CognitiveSceneLifecycleState::Candidate);
    assert_eq!(
        round_trip.source_atom_ids,
        vec![first_atom_id, second_atom_id]
    );
}

#[test]
fn cognitive_scene_enums_use_stable_snake_case_json() {
    assert_eq!(
        serde_json::to_value(CognitiveSceneType::ContradictionField).unwrap(),
        "contradiction_field"
    );
    assert_eq!(
        serde_json::to_value(CognitiveSceneLifecycleState::Superseded).unwrap(),
        "superseded"
    );
    assert_eq!(
        serde_json::to_value(CognitiveSceneProvenanceMethod::ManualReview).unwrap(),
        "manual_review"
    );

    assert_eq!(
        serde_json::from_value::<CognitiveSceneType>(json!("theme")).unwrap(),
        CognitiveSceneType::Theme
    );
    assert_eq!(
        serde_json::from_value::<CognitiveSceneLifecycleState>(json!("active")).unwrap(),
        CognitiveSceneLifecycleState::Active
    );
    assert_eq!(
        serde_json::from_value::<CognitiveSceneProvenanceMethod>(json!("deterministic_rule"))
            .unwrap(),
        CognitiveSceneProvenanceMethod::DeterministicRule
    );
}

#[test]
fn cognitive_scene_from_atoms_rejects_cross_space_sources() {
    let space_id = Uuid::new_v4();
    let namespace_id = Uuid::new_v4();
    let matching_atom = atom_in_space(space_id, Some(namespace_id));
    let foreign_atom = atom_in_space(Uuid::new_v4(), Some(namespace_id));

    let error = CognitiveScene::from_atoms(
        Uuid::new_v4(),
        space_id,
        Some(namespace_id),
        CognitiveSceneType::Theme,
        "Cross-space source fixture",
        &[matching_atom, foreign_atom],
        "Should be rejected before scene construction.",
        vec!["cross_space_source".to_string()],
        CognitiveSceneProvenance {
            source_trace_ids: vec![Uuid::new_v4()],
            method: CognitiveSceneProvenanceMethod::DeterministicRule,
            builder: Some("cognitive_scene_contract_test".to_string()),
            rationale: Some("same-space validation fixture".to_string()),
        },
        Utc.with_ymd_and_hms(2026, 7, 2, 11, 0, 0).unwrap(),
    )
    .expect_err("cross-space MemoryAtom sources must be rejected");

    assert_eq!(
        error,
        CognitiveSceneValidationError::SourceAtomOutsideSpace { index: 1 }
    );
}

#[test]
fn cognitive_scene_from_atoms_cites_source_atom_ids() {
    let space_id = Uuid::new_v4();
    let namespace_id = Uuid::new_v4();
    let first_atom = atom_in_space(space_id, Some(namespace_id));
    let second_atom = atom_in_space(space_id, None);
    let expected_ids = vec![first_atom.id, second_atom.id];

    let scene = CognitiveScene::from_atoms(
        Uuid::new_v4(),
        space_id,
        Some(namespace_id),
        CognitiveSceneType::PracticeField,
        "Same-space source fixture",
        &[first_atom, second_atom],
        "Scene can cite atoms from the same CognitiveSpace.",
        vec!["same_space_grouping".to_string()],
        CognitiveSceneProvenance {
            source_trace_ids: Vec::new(),
            method: CognitiveSceneProvenanceMethod::Fixture,
            builder: Some("cognitive_scene_contract_test".to_string()),
            rationale: None,
        },
        Utc.with_ymd_and_hms(2026, 7, 2, 11, 30, 0).unwrap(),
    )
    .expect("same-space MemoryAtom sources should build a scene");

    assert_eq!(scene.space_id, space_id);
    assert_eq!(scene.namespace_id, Some(namespace_id));
    assert_eq!(scene.source_atom_ids, expected_ids);
}
