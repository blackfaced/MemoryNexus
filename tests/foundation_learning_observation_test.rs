use chrono::{Duration, TimeZone, Utc};
use memorynexus::domain::foundation_learning_observation::{
    build_foundation_learning_observation, FoundationAttemptRole, FoundationEvidenceKind,
    FoundationLearningEvidenceRecord, FoundationMistakeType, FoundationObservationStatus,
    ObservationSourceIdentity,
};
use uuid::Uuid;

#[test]
fn weekly_observation_separates_attempt_roles_and_cites_recurring_errors() {
    let space_id = Uuid::new_v4();
    let namespace_id = Uuid::new_v4();
    let now = Utc.with_ymd_and_hms(2026, 8, 15, 8, 0, 0).unwrap();
    let records = vec![
        attempt(
            space_id,
            namespace_id,
            "original-1",
            now - Duration::days(5),
            FoundationAttemptRole::Initial,
            false,
            Some("multiplication_fact"),
        ),
        attempt(
            space_id,
            namespace_id,
            "correction-1",
            now - Duration::days(4),
            FoundationAttemptRole::Correction,
            true,
            None,
        ),
        attempt(
            space_id,
            namespace_id,
            "reinforcement-1",
            now - Duration::days(2),
            FoundationAttemptRole::Reinforcement,
            false,
            Some("multiplication_fact"),
        ),
    ];

    let summary = build_foundation_learning_observation(
        space_id,
        namespace_id,
        now - Duration::days(7),
        now,
        records,
    );

    assert_eq!(summary.status, FoundationObservationStatus::Ready);
    assert_eq!(summary.facts.initial_attempt_count, 1);
    assert_eq!(summary.facts.correction_attempt_count, 1);
    assert_eq!(summary.facts.reinforcement_attempt_count, 1);
    assert_eq!(summary.facts.successful_correction_count, 1);
    assert_eq!(summary.deterministic_aggregates.recurring_errors.len(), 1);
    let recurring = &summary.deterministic_aggregates.recurring_errors[0];
    assert_eq!(recurring.mistake_type, "multiplication_fact");
    assert_eq!(recurring.occurrence_count, 2);
    assert_eq!(recurring.supporting_sources.len(), 2);
    assert_eq!(summary.deterministic_aggregates.delayed_recurrence.len(), 1);
    assert!(summary.hypotheses.is_empty());
    assert!(summary.responsibility_note.contains("source application"));
    assert!(summary.responsibility_note.contains("study_buddy"));
}

#[test]
fn selection_is_scope_window_trust_and_identity_safe() {
    let space_id = Uuid::new_v4();
    let namespace_id = Uuid::new_v4();
    let now = Utc.with_ymd_and_hms(2026, 8, 15, 8, 0, 0).unwrap();
    let eligible = attempt(
        space_id,
        namespace_id,
        "eligible",
        now - Duration::days(1),
        FoundationAttemptRole::Correction,
        true,
        None,
    );
    let duplicate = eligible.clone();
    let mut cross_space = eligible.clone();
    cross_space.space_id = Uuid::new_v4();
    cross_space.source_identity.record_id = "cross-space".to_string();
    let mut cross_namespace = eligible.clone();
    cross_namespace.namespace_id = Uuid::new_v4();
    cross_namespace.source_identity.record_id = "cross-namespace".to_string();
    let mut unreviewed = eligible.clone();
    unreviewed.source_identity.record_id = "unreviewed".to_string();
    unreviewed.evidence_trust = "model_derived_unreviewed".to_string();
    let mut reduced_work = eligible.clone();
    reduced_work.source_identity.record_id = "reduced-work".to_string();
    reduced_work.kind = FoundationEvidenceKind::Unsupported;
    let out_of_window = attempt(
        space_id,
        namespace_id,
        "old",
        now - Duration::days(8),
        FoundationAttemptRole::Correction,
        true,
        None,
    );

    let summary = build_foundation_learning_observation(
        space_id,
        namespace_id,
        now - Duration::days(7),
        now,
        vec![
            eligible,
            duplicate,
            cross_space,
            cross_namespace,
            unreviewed,
            reduced_work,
            out_of_window,
        ],
    );

    assert_eq!(summary.facts.eligible_evidence_count, 1);
    assert_eq!(summary.facts.correction_attempt_count, 1);
    assert_eq!(summary.facts.successful_correction_count, 1);
    assert_eq!(summary.excluded_evidence_count, 6);
    assert!(summary
        .evidence_gaps
        .iter()
        .any(|gap| gap.code == "sparse_evidence"));
}

#[test]
fn all_ineligible_evidence_returns_an_explicit_gap_without_mastery_claims() {
    let space_id = Uuid::new_v4();
    let namespace_id = Uuid::new_v4();
    let now = Utc.with_ymd_and_hms(2026, 8, 15, 8, 0, 0).unwrap();
    let mut unreviewed = attempt(
        space_id,
        namespace_id,
        "summary-1",
        now - Duration::hours(2),
        FoundationAttemptRole::Initial,
        true,
        None,
    );
    unreviewed.evidence_trust = "model_derived_unreviewed".to_string();

    let summary = build_foundation_learning_observation(
        space_id,
        namespace_id,
        now - Duration::days(7),
        now,
        vec![unreviewed],
    );

    assert_eq!(summary.status, FoundationObservationStatus::EvidenceGap);
    assert_eq!(summary.facts.eligible_evidence_count, 0);
    assert!(summary.deterministic_aggregates.recurring_errors.is_empty());
    assert!(summary.hypotheses.is_empty());
    assert!(summary
        .evidence_gaps
        .iter()
        .any(|gap| gap.code == "no_current_trusted_evidence"));
}

fn attempt(
    space_id: Uuid,
    namespace_id: Uuid,
    record_id: &str,
    occurred_at: chrono::DateTime<Utc>,
    role: FoundationAttemptRole,
    is_correct: bool,
    mistake_type: Option<&str>,
) -> FoundationLearningEvidenceRecord {
    FoundationLearningEvidenceRecord {
        space_id,
        namespace_id,
        occurred_at,
        evidence_trust: "contract_trusted".to_string(),
        source_identity: ObservationSourceIdentity {
            source_product: "study_buddy".to_string(),
            source_installation_id: Uuid::from_u128(1),
            record_type: "learning_attempt".to_string(),
            record_id: record_id.to_string(),
            revision: 1,
        },
        kind: FoundationEvidenceKind::LearningAttempt {
            role,
            is_correct,
            mistake_type: mistake_type.and_then(FoundationMistakeType::from_normalized),
        },
    }
}
